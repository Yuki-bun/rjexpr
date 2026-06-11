use nom::{Check, Err as NErr, Mode, OutputM, combinator::eof, multi::fold};
use std::{borrow::Cow, hash::Hash};

use crate::{BinOp, Expression, Literal, UnaryOp, iter::comma_separated_iter};
use nom::{
    AsChar, IResult, OutputMode, Parser,
    branch::alt,
    bytes::complete::{tag, take_while},
    character::complete::{alphanumeric0, anychar, char, satisfy, space0},
    combinator::{map, opt, peek, verify},
    error::Error as NomError,
    multi::many0,
    number::complete::double,
    sequence::{delimited, preceded, terminated},
};

pub fn parse<'a, S, B>(input: &'a str) -> Result<Expression<S, B>, String>
where
    S: From<&'a str> + Clone,
    B: From<String> + From<Cow<'a, str>> + Hash + Eq,
{
    delimited(space0, opt(expr), eof)
        .parse(input)
        .map(|(_, expr)| expr.unwrap_or(Expression::Empty))
        .map_err(|err| err.to_string())
}

type PResult<'a, S, B> = IResult<&'a str, Expression<S, B>>;

macro_rules! parser_type {
    ($lifetime:tt, $output:ty) => {
        impl Parser<&$lifetime str, Output = $output, Error = NomError<&$lifetime str>>
    };
}

fn expr<'a, S, B>(input: &'a str) -> PResult<'a, S, B>
where
    S: From<&'a str> + Clone,
    B: From<String> + From<Cow<'a, str>> + Hash + Eq,
{
    assignment.parse(input)
}

fn assignment<'a, S, B>(input: &'a str) -> PResult<'a, S, B>
where
    S: From<&'a str> + Clone,
    B: From<String> + From<Cow<'a, str>> + Hash + Eq,
{
    let (input2, left) = ternary.parse(input)?;
    let assign_not_eq = tokc('=');
    match (assign_not_eq, expr).parse(input2) {
        Ok((input3, (_assign, right))) => {
            Ok((input3, Expression::binary(left, BinOp::Assign, right)))
        }
        Err(_) => Ok((input2, left)),
    }
}

fn ternary<'a, S, B>(input: &'a str) -> PResult<'a, S, B>
where
    S: From<&'a str> + Clone,
    B: From<String> + From<Cow<'a, str>> + Hash + Eq,
{
    let (input2, cond) = pipe.parse(input)?;
    match (tokc('?'), expr, tokc(':'), expr).parse(input2) {
        Ok((input3, (_, left, _, right))) => Ok((input3, Expression::ternary(cond, left, right))),
        Err(_) => Ok((input2, cond)),
    }
}

macro_rules! left_assoc_binary {
    ($name:ident, $expr:ident, $(( $op:literal, $op_t:ident )),*) => {
        fn $name<'a, S, B>(input: &'a str) -> PResult<'a, S, B>
        where
            S: From<&'a str> + Clone,
            B: From<String> + From<Cow<'a, str>> + Hash + Eq
        {
            fn op_p(input: &str) -> IResult<&str, BinOp> {
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

fn unary<'a, S, B>(input: &'a str) -> PResult<'a, S, B>
where
    S: From<&'a str> + Clone,
    B: From<String> + From<Cow<'a, str>> + Hash + Eq,
{
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

fn unary_op(input: &str) -> IResult<&str, UnaryOp> {
    alt((
        tokc('+').map(|_| UnaryOp::Plus),
        tokc('-').map(|_| UnaryOp::Minus),
        tokc('!').map(|_| UnaryOp::Not),
    ))
    .parse(input)
}

fn postfix<'a, S, B>(input: &'a str) -> PResult<'a, S, B>
where
    S: From<&'a str> + Clone,
    B: From<String> + From<Cow<'a, str>> + Hash + Eq,
{
    primary.flat_map(collect_post_prefix).parse(input)
}

/// Parser returned by this function should only be used once
fn collect_post_prefix<'a, S, B>(
    mut receiver: Expression<S, B>,
) -> parser_type!('a, Expression<S, B>)
where
    S: From<&'a str> + Clone,
    B: From<String> + From<Cow<'a, str>> + Hash + Eq,
{
    fold(
        0..,
        next_step,
        move || std::mem::replace(&mut receiver, Expression::Empty),
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
                        _ => Expression::invoke(acc, None::<S>, arguments),
                    }
                }
            }
        },
    )
}

#[derive(Debug)]
enum PostfixStep<S, B: Eq + Hash> {
    Member(S),
    Index(Option<Box<Expression<S, B>>>),
    Invoke(Vec<Expression<S, B>>),
}

fn next_step<'a, S, B>(input: &'a str) -> IResult<&'a str, PostfixStep<S, B>>
where
    S: From<&'a str> + Clone,
    B: From<String> + From<Cow<'a, str>> + Hash + Eq,
{
    alt((
        tokc('.')
            .and(terminated(ident_name, space0))
            .map(|(_, name)| PostfixStep::Member(name.into())),
        delimited(tokc('['), opt(expr), tokc(']')).map(|idx| PostfixStep::Index(idx.map(Box::new))),
        delimited(tokc('('), comma_separated(expr), tokc(')')).map(PostfixStep::Invoke),
    ))
    .parse(input)
}

fn primary<'a, S, B>(input: &'a str) -> PResult<'a, S, B>
where
    S: From<&'a str> + Clone,
    B: From<String> + From<Cow<'a, str>> + Hash + Eq,
{
    alt((
        literal.map(Expression::literal),
        ident,
        custom_ident.map(Expression::custom_id),
        list,
        js_map,
        paren,
    ))
    .parse(input)
}

fn literal<'a, B>(input: &'a str) -> IResult<&'a str, Literal<B>>
where
    B: From<Cow<'a, str>>,
{
    alt((
        tok("true").map(|_| Literal::Boolean(true)),
        tok("false").map(|_| Literal::Boolean(false)),
        tok("null").map(|_| Literal::Null),
        tok("undefined").map(|_| Literal::Undefined),
        terminated(double, space0).map(Literal::Number),
        terminated(string_literal, space0).map(Literal::string),
    ))
    .parse(input)
}

fn ident<'a, S, B>(input: &'a str) -> PResult<'a, S, B>
where
    S: From<&'a str> + Clone,
    B: From<String> + From<Cow<'a, str>> + Hash + Eq,
{
    let (input2, id) = terminated(ident_name, space0).parse(input)?;
    match (tok("=>"), expr).parse(input2) {
        Ok((input3, (_arrow, body))) => {
            Ok((input3, Expression::arrow_func(vec![S::from(id)], body)))
        }
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

fn list<'a, S, B>(input: &'a str) -> PResult<'a, S, B>
where
    S: From<&'a str> + Clone,
    B: From<String> + From<Cow<'a, str>> + Hash + Eq,
{
    delimited(tokc('['), comma_separated(expr), tokc(']'))
        .map(Expression::List)
        .parse(input)
}

fn js_map<'a, S, B>(input: &'a str) -> PResult<'a, S, B>
where
    S: From<&'a str> + Clone,
    B: From<String> + From<Cow<'a, str>> + Hash + Eq,
{
    let (i2, _) = tokc('{').parse(input)?;
    let mut entry_iter = comma_separated_iter(i2, map_entry);
    let entries = entry_iter
        .by_ref()
        .map(|(key, value)| (key.into(), value))
        .collect();
    let (i3, _) = entry_iter.finish()?;
    let (i4, _) = tokc('}').parse(i3)?;
    Ok((i4, Expression::map(entries)))
}

fn map_entry<'a, S, B>(input: &'a str) -> IResult<&'a str, (Cow<'a, str>, Expression<S, B>)>
where
    S: From<&'a str> + Clone,
    B: From<String> + From<Cow<'a, str>> + Hash + Eq,
{
    (
        terminated(ident_name, space0)
            .map(Cow::Borrowed)
            .or(string_literal),
        preceded(space0, tokc(':')),
        expr,
    )
        .map(|(key, _colon, val)| (key, val))
        .parse(input)
}

fn paren<'a, S, B>(input: &'a str) -> PResult<'a, S, B>
where
    S: From<&'a str> + Clone,
    B: From<String> + From<Cow<'a, str>> + Hash + Eq,
{
    let (i2, _) = tokc('(').parse(input)?;
    let (i3, first_arg) = match expr.parse(i2) {
        // (id) => body or (expr)
        Ok((i3, Expression::ID(first_arg))) => (i3, first_arg),
        Ok((i3, expr)) => {
            let (i3, _) = tokc(')').parse(i3)?;
            return Ok((i3, Expression::paren(expr)));
        }
        // Has to be ( ) => body
        Err(NErr::Error(_)) => {
            let (rest, (_, body)) = (tokc(')').and(tok("=>")), expr).parse(i2)?;
            return Ok((rest, Expression::arrow_func(Vec::<S>::new(), body)));
        }
        Err(NErr::Failure(e)) => return Err(NErr::Failure(e)),
        Err(NErr::Incomplete(e)) => return Err(NErr::Incomplete(e)),
    };

    // check if there are more than 2 ident insdie paren
    let (i4, _) = match tokc(',').parse(i3) {
        Ok(res) => res,
        // Single ident insdie (). either (a) => body or (a)
        Err(NErr::Error(_)) => {
            let (i4, _) = tokc(')').parse(i3)?;
            match preceded(tok("=>"), expr).parse(i4) {
                Ok((rest, body)) => {
                    return Ok((rest, Expression::arrow_func(vec![first_arg], body)));
                }
                Err(_) => return Ok((i4, Expression::paren(Expression::ID(first_arg)))),
            }
        }
        Err(NErr::Failure(e)) => return Err(NErr::Failure(e)),
        Err(NErr::Incomplete(e)) => return Err(NErr::Incomplete(e)),
    };
    let mut first_arg = Some(first_arg);
    let (i5, (mut args, last_arg)) = fold(
        0..,
        (ident_name, space0.and(tokc(','))),
        || vec![std::mem::take(&mut first_arg).unwrap()],
        |mut acc, (new, _comma)| {
            acc.push(new.into());
            acc
        },
    )
    .and(opt(terminated(ident_name, space0)))
    .parse(i4)?;
    args.extend(last_arg.map(S::from));
    let (rest, body) = preceded(tok(")").and(tok("=>")), expr).parse(i5)?;
    Ok((rest, Expression::arrow_func(args, body)))
}

fn tok<'a>(pattern: &'static str) -> parser_type!('a, &'a str) {
    tag(pattern).and(space0).map(|(pat, _ws)| pat)
}

fn tokc<'a>(c: char) -> parser_type!('a,char) {
    char(c).and(space0).map(|(c, _ws)| c)
}

fn ident_name(input: &str) -> IResult<&str, &str> {
    peek(satisfy(AsChar::is_alpha))
        .flat_map(|_| alphanumeric0)
        .parse(input)
}

fn custom_ident(input: &str) -> IResult<&str, &str> {
    delimited(tag("${"), take_while(|char| char != '}'), char('}')).parse(input)
}

pub fn string_literal(input: &str) -> IResult<&str, Cow<'_, str>> {
    let (input2, delim) = char('"').or(char('\'')).parse(input)?;
    let (input3, non_escape) = take_while(move |c| c != delim && c != '\\').parse(input2)?;
    match anychar.parse(input3) {
        Ok((input4, delim_or_escape)) => {
            if delim_or_escape == delim {
                Ok((input4, Cow::Borrowed(non_escape)))
            } else {
                fold(
                    0..,
                    alt((normal_char(delim), escaped_char)),
                    || non_escape.to_string(),
                    |mut acc, new| {
                        acc.push(new);
                        acc
                    },
                )
                .map(Cow::Owned)
                .and(char(delim))
                .map(|(string, _delim)| string)
                .parse(input3)
            }
        }
        Err(e) => Err(e),
    }
}

fn normal_char<'a>(delim: char) -> parser_type!('a, char) {
    verify(anychar, move |&c| c != delim && c != '\\')
}

fn escaped_char(input: &str) -> IResult<&str, char> {
    preceded(
        char('\\'),
        alt((
            char('"'),
            char('\''),
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

    type Expression<'a> = super::Expression<&'a str, Cow<'a, str>>;

    #[test]
    fn test_comma_separated() {
        fn entry<'a>(input: &'a str) -> IResult<&'a str, &'a str> {
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
        let (_, parsed) = comma_separated(map_entry::<&str, Cow<_>>)
            .parse("'a': bc, b: 12")
            .unwrap();
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
