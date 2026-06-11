#[macro_use]
mod helpers;

use nom::{Check, Err as NErr, Mode, OutputM, combinator::eof, multi::fold};
use std::borrow::Cow;

use crate::{BinOp, Expression, Literal, UnaryOp};
use nom::{
    AsChar, IResult, OutputMode, Parser,
    branch::alt,
    bytes::complete::{tag, take_while},
    character::complete::{alphanumeric0, anychar, char, satisfy, space0},
    combinator::{map, opt, peek, verify},
    error::{Error as NomError, ErrorKind},
    multi::{many0, many1},
    number::complete::double,
    sequence::{delimited, preceded, terminated},
};

pub use helpers::{DebugP, dbg_p};

pub fn parse(input: &str) -> Result<Expression<'_>, String> {
    delimited(space0, opt(expr), eof)
        .parse(input)
        .map(|(_, expr)| expr.unwrap_or(Expression::Empty))
        .map_err(|err| err.to_string())
}

type PResult<'a, O = Expression<'a>> = IResult<&'a str, O>;

macro_rules! parser_type {
    ($lifetime:tt, $output:ty) => {
        impl Parser<&$lifetime str, Output = $output, Error = NomError<&$lifetime str>>
    };
}

fn expr(input: &str) -> PResult<'_> {
    assignment.parse(input)
}

fn assignment(input: &str) -> PResult<'_> {
    let (input2, left) = ternary.parse(input)?;
    let assign_not_eq = tokc('=');
    match (assign_not_eq, expr).parse(input2) {
        Ok((input3, (_assign, right))) => Ok((
            input3,
            Expression::Binary {
                op: BinOp::Assign,
                left: Box::new(left),
                right: Box::new(right),
            },
        )),
        Err(_) => Ok((input2, left)),
    }
}

fn ternary(input: &str) -> PResult<'_> {
    let (input2, cond) = pipe.parse(input)?;
    match (tokc('?'), expr, tokc(':'), expr).parse(input2) {
        Ok((input3, (_, left, _, right))) => Ok((
            input3,
            Expression::Ternary {
                cond: Box::new(cond),
                left: Box::new(left),
                right: Box::new(right),
            },
        )),
        Err(_) => Ok((input2, cond)),
    }
}

macro_rules! left_assoc_binary {
    ($name:ident, $expr:ident, $(( $op:literal, $op_t:ident )),*) => {
        fn $name(input: &str) -> PResult<'_> {
            fn op_p(input: &str) -> PResult<'_, BinOp> {
                alt((
                    $(tok($op).map(|_| BinOp::$op_t),)*
                )).parse(input)
            }

            left_associative(op_p, $expr, Expression::binary).parse(input)
        }

    };
}

left_assoc_binary!(pipe, nullish_coalescing, ("|>", Pipe));
left_assoc_binary!(nullish_coalescing, logical_or, ("??", NullishCoalesce));
left_assoc_binary!(logical_or, logical_and, ("||", LogicalOr));
left_assoc_binary!(logical_and, bitwise_or, ("&&", LogicalAnd));
left_assoc_binary!(bitwise_or, bitwise_xor, ("|", BitwiseOr));
left_assoc_binary!(bitwise_xor, bitwise_and, ("^", BitwiseXor));
left_assoc_binary!(bitwise_and, equality, ("&", BitwiseAnd));
left_assoc_binary!(
    equality,
    relational,
    ("===", StrictEqual),
    ("==", Equal),
    ("!==", StrictNotEqual),
    ("!=", NotEqual)
);
left_assoc_binary!(
    relational,
    additive,
    (">=", GreaterOrEqual),
    ("<=", LessOrEqual),
    (">", GreaterThan),
    ("<", LessThan)
);
left_assoc_binary!(additive, multiplicative, ("+", Add), ("-", Subtract));
left_assoc_binary!(
    multiplicative,
    unary,
    ("*", Multiply),
    ("/", Divide),
    ("%", Modulo)
);

struct LeftAssociative<O, E, F> {
    op_p: O,
    expr_p: E,
    bind: F,
}

fn left_associative<'a, O, E, F>(op_p: O, expr_p: E, bind: F) -> LeftAssociative<O, E, F>
where
    O: Parser<&'a str> + Clone,
    E: Parser<&'a str, Error = O::Error> + Clone,
    F: Fn(E::Output, O::Output, E::Output) -> E::Output,
{
    LeftAssociative { op_p, expr_p, bind }
}

impl<'a, O, E, F> Parser<&'a str> for LeftAssociative<O, E, F>
where
    O: Parser<&'a str> + Clone,
    E: Parser<&'a str, Error = O::Error> + Clone,
    F: Fn(E::Output, O::Output, E::Output) -> E::Output,
{
    type Output = E::Output;

    type Error = E::Error;

    fn process<OM: OutputMode>(
        &mut self,
        input: &'a str,
    ) -> nom::PResult<OM, &'a str, Self::Output, Self::Error> {
        (
            self.expr_p.clone(),
            many0((self.op_p.clone(), self.expr_p.clone())),
        )
            .map(|(left_most, pairs)| {
                pairs
                    .into_iter()
                    .fold(left_most, |left, (op, right)| (self.bind)(left, op, right))
            })
            .process::<OM>(input)
    }
}

fn unary(input: &str) -> PResult<'_> {
    (unary_op, unary)
        .map(|(op, val)| {
            let Expression::Literal(Literal::Number(num)) = val else {
                return Expression::Unary(op, Box::new(val));
            };
            match op {
                UnaryOp::Plus => Expression::Literal(Literal::Number(num)),
                UnaryOp::Minus => Expression::Literal(Literal::Number(-num)),
                _ => Expression::Unary(op, Box::new(val)),
            }
        })
        .or(postfix)
        .parse(input)
}

fn unary_op(input: &str) -> PResult<'_, UnaryOp> {
    alt((
        tokc('+').map(|_| UnaryOp::Plus),
        tokc('-').map(|_| UnaryOp::Minus),
        tokc('!').map(|_| UnaryOp::Not),
    ))
    .parse(input)
}

fn postfix(input: &str) -> PResult<'_> {
    primary
        .flat_map(|receiver| PostfixParser { receiver })
        .parse(input)
}

#[derive(Debug)]
enum PostfixStep<'a> {
    Member(&'a str),
    Index(Option<Box<Expression<'a>>>),
    Invoke(Vec<Expression<'a>>),
}

struct PostfixParser<'a> {
    receiver: Expression<'a>,
}

fn next_step<'a>(input: &'a str) -> PResult<'a, PostfixStep<'a>> {
    alt((
        tokc('.')
            .and(terminated(ident_name, space0))
            .map(|(_, name)| PostfixStep::Member(name)),
        delimited(tokc('['), opt(expr), tokc(']')).map(|idx| PostfixStep::Index(idx.map(Box::new))),
        delimited(tokc('('), comma_separated(expr), tokc(')')).map(PostfixStep::Invoke),
    ))
    .parse(input)
}

impl<'a> Parser<&'a str> for PostfixParser<'a> {
    type Output = Expression<'a>;
    type Error = NomError<&'a str>;

    fn process<OM: OutputMode>(
        &mut self,
        input: &'a str,
    ) -> nom::PResult<OM, &'a str, Self::Output, Self::Error> {
        fold(
            0..,
            next_step,
            || std::mem::replace(&mut self.receiver, Expression::Empty),
            |acc, new| {
                match new {
                    PostfixStep::Member(name) => Expression::getter(acc, name),
                    PostfixStep::Index(argument) => Expression::index(acc, argument.map(|o| *o)),
                    PostfixStep::Invoke(arguments) => {
                        // Match on the *current* state of the accumulator
                        match acc {
                            Expression::Getter(inner_receiver, method) => {
                                Expression::invoke(*inner_receiver, Some(method), arguments)
                            }
                            _ => Expression::invoke(acc, None, arguments),
                        }
                    }
                }
            },
        )
        .process::<OM>(input)
    }
}

fn primary(input: &str) -> PResult<'_> {
    alt((
        literal,
        ident,
        custom_ident.map(Expression::CustomID),
        list,
        js_map,
        paren,
    ))
    .parse(input)
}

fn literal(input: &str) -> PResult<'_> {
    alt((
        tok("true").map(|_| Expression::Literal(Literal::Boolean(true))),
        tok("false").map(|_| Expression::Literal(Literal::Boolean(false))),
        tok("null").map(|_| Expression::Literal(Literal::Null)),
        tok("undefined").map(|_| Expression::Literal(Literal::Undefined)),
        terminated(double, space0).map(|n| Expression::Literal(Literal::Number(n))),
        terminated(string_literal, space0)
            .map(|s| Expression::Literal(Literal::String(Cow::Owned(s)))),
    ))
    .parse(input)
}

fn ident(input: &str) -> PResult<'_> {
    let (input2, id) = terminated(ident_name, space0).parse(input)?;
    match (tok("=>"), expr).parse(input2) {
        Ok((input3, (_arrow, body))) => Ok((input3, Expression::arrow_func(vec![id], body))),
        Err(_) => Ok((input2, Expression::id(id))),
    }
}

struct CommaSeparated<P> {
    parser: P,
}

impl<'a, P> Parser<&'a str> for CommaSeparated<P>
where
    P: Parser<&'a str, Error = NomError<&'a str>> + Clone,
{
    type Output = Vec<P::Output>;

    type Error = P::Error;

    fn process<OM: OutputMode>(
        &mut self,
        input: &'a str,
    ) -> nom::PResult<OM, &'a str, Self::Output, Self::Error> {
        let Ok((input2, first)) = self.parser.process::<OM>(input) else {
            return Ok((input, OM::Output::bind(Vec::new)));
        };
        let mut next_input = input2;
        let mut items = OM::Output::map(first, |first| vec![first]);

        let mut entry_p = preceded(tokc(','), self.parser.clone());

        while let Ok((next_, entry)) = entry_p.process::<OM>(next_input) {
            next_input = next_;
            items = OM::Output::combine(items, entry, |mut current, new| {
                current.push(new);
                current
            });
        }
        opt(tokc(','))
            .process::<OutputM<Check, OM::Error, OM::Incomplete>>(next_input)
            .map(|(rest, _)| (rest, items))
    }
}

fn comma_separated<'a, P>(
    item: P,
) -> impl Parser<&'a str, Output = Vec<P::Output>, Error = P::Error>
where
    P: Parser<&'a str, Error = NomError<&'a str>> + Clone,
{
    CommaSeparated { parser: item }
}

fn list(input: &str) -> PResult<'_> {
    delimited(tokc('['), comma_separated(expr), tokc(']'))
        .map(Expression::List)
        .parse(input)
}

fn js_map(input: &str) -> PResult<'_> {
    delimited(tokc('{'), comma_separated(map_entry), tokc('}'))
        .map(|entries| Expression::Map(FromIterator::from_iter(entries)))
        .parse(input)
}

fn map_entry(input: &str) -> PResult<'_, (Cow<'_, str>, Expression<'_>)> {
    (
        terminated(ident_name, space0)
            .map(Cow::Borrowed)
            .or(string_literal.map(Cow::Owned)),
        preceded(space0, tokc(':')),
        expr,
    )
        .map(|(key, _colon, val)| (key, val))
        .parse(input)
}

fn paren<'a>(input: &'a str) -> PResult<'a> {
    let (i2, args) = delimited(tokc('('), comma_separated(expr), tokc(')')).parse(input)?;
    let mut first = (args.len() == 1).then(|| args[0].clone());
    allow_func_p(args)
        .or(move |_| {
            std::mem::take(&mut first).map_or_else(
                || Err(NErr::Error(NomError::new(i2, ErrorKind::Tag))),
                |first| Ok((i2, Expression::paren(first))),
            )
        })
        .parse(i2)
}

fn allow_func_p<'a>(
    params: Vec<Expression<'a>>,
) -> impl Parser<&'a str, Output = Expression<'a>, Error = NomError<&'a str>> {
    (tok("=>"), expr).map_res(move |(_arrow, body)| {
        params
            .iter()
            .map(Expression::as_id)
            .collect::<Option<Vec<_>>>()
            .map(|params| Expression::arrow_func(params, body))
            .ok_or(ErrorKind::Tag) // TODO: use proper error
    })
}

fn tok<'a>(pattern: &'static str) -> parser_type!('a, &'a str) {
    tag(pattern).and(space0).map(|(pat, _ws)| pat)
}

fn tokc<'a>(c: char) -> parser_type!('a,char) {
    char(c).and(space0).map(|(c, _ws)| c)
}

fn ident_name(input: &str) -> PResult<'_, &str> {
    peek(satisfy(AsChar::is_alpha))
        .flat_map(|_| alphanumeric0)
        .parse(input)
}

fn custom_ident(input: &str) -> PResult<'_, &str> {
    delimited(tag("${"), take_while(|char| char != '}'), char('}')).parse(input)
}

pub fn string_literal(input: &str) -> PResult<'_, String> {
    let (input2, delim) = char('"').or(char('\'')).parse(input)?;
    let escaped_p = alt((escaped_char, normal_char(delim)));
    terminated(many1(escaped_p), char(delim))
        .map(|chars| chars.iter().collect())
        .parse(input2)
}

fn normal_char<'a>(delim: char) -> parser_type!('a, char) {
    verify(anychar, move |&c| c != delim && c != '\\')
}

fn escaped_char(input: &str) -> PResult<'_, char> {
    preceded(
        char('\\'),
        alt((
            char('"'),
            char('\\'),
            char('/'),
            map(char('b'), |_| '\x08'), // Backspace
            map(char('f'), |_| '\x0C'), // Form feed
            map(char('n'), |_| '\n'),
            map(char('r'), |_| '\r'),
            map(char('t'), |_| '\t'),
        )),
    )
    .parse(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_comma_separated() {
        fn entry<'a>(input: &'a str) -> PResult<'a, &'a str> {
            terminated(ident_name, space0).parse(input)
        }

        let (rest, parsed) = comma_separated(entry).parse("abc, def, ad,").unwrap();
        assert!(rest.is_empty());
        assert_eq!(parsed, vec!["abc", "def", "ad"]);
    }

    #[test]
    fn string_lit() {
        let (_, parsed) = string_literal("'abc'").unwrap();
        assert_eq!(parsed, "abc".to_string());
    }

    #[test]
    fn test_comma_separated_fields() {
        let (_, parsed) = comma_separated(map_entry).parse("'a': bc, b: 12").unwrap();
        assert_eq!(
            parsed,
            vec![
                (Cow::Borrowed("a"), Expression::id("bc")),
                (
                    Cow::Borrowed("b"),
                    Expression::literal(Literal::Number(12.0))
                ),
            ]
        );
    }

    #[test]
    fn try_stuff() {
        let (_, parsed) = paren.parse("(abc)").unwrap();
        assert_eq!(parsed, Expression::paren(Expression::id("abc")));
    }
}
