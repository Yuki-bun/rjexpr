use std::{borrow::Cow, collections::HashMap};

use jexpr::{BinOp, Expression, Literal, UnaryOp, parse};

fn expect_parse(s: &str, expected: Expression<'_>) {
    let parsed = parse(s).unwrap();
    assert_eq!(parsed, expected)
}

fn op(s: &str) -> BinOp {
    match s {
        "=" => BinOp::Assign,
        "+" => BinOp::Add,
        "-" => BinOp::Subtract,
        "*" => BinOp::Multiply,
        "/" => BinOp::Divide,
        "%" => BinOp::Modulo,
        "^" => BinOp::BitwiseXor,
        "==" => BinOp::Equal,
        "!=" => BinOp::NotEqual,
        ">" => BinOp::GreaterThan,
        "<" => BinOp::LessThan,
        ">=" => BinOp::GreaterOrEqual,
        "<=" => BinOp::LessOrEqual,
        "||" => BinOp::LogicalOr,
        "&&" => BinOp::LogicalAnd,
        "??" => BinOp::NullishCoalesce,
        "&" => BinOp::BitwiseAnd,
        "===" => BinOp::StrictEqual,
        "!==" => BinOp::StrictNotEqual,
        "|" => BinOp::BitwiseOr,
        _ => unreachable!(),
    }
}

fn num(n: f64) -> Expression<'static> {
    Expression::literal(Literal::Number(n))
}

fn string(s: &str) -> Expression<'static> {
    Expression::literal(Literal::String(Cow::Owned(s.to_string())))
}

fn boolean(b: bool) -> Expression<'static> {
    Expression::literal(Literal::Boolean(b))
}

fn null() -> Expression<'static> {
    Expression::literal(Literal::Null)
}

fn id(name: &'static str) -> Expression<'static> {
    Expression::id(name)
}

#[test]
fn should_parse_an_empty_expression() {
    expect_parse("", Expression::empty());
}

#[test]
fn should_parse_an_identifier() {
    expect_parse("abc", id("abc"));
}

#[test]
fn should_parse_a_string_literal() {
    expect_parse("\"abc\"", string("abc"));
}

#[test]
fn should_parse_a_bool_literal() {
    expect_parse("true", boolean(true));
    expect_parse("false", boolean(false));
}

#[test]
fn should_parse_a_null_literal() {
    expect_parse("null", null());
}

#[test]
fn should_parse_an_undefined_literal() {
    expect_parse("undefined", Expression::literal(Literal::Undefined));
}

#[test]
fn should_parse_an_integer_literal() {
    expect_parse("123", num(123.0));
}

#[test]
fn should_parse_a_double_literal() {
    expect_parse("1.23", num(1.23));
}

#[test]
fn should_parse_a_positive_double_literal() {
    expect_parse("+1.23", num(1.23));
}

#[test]
fn should_parse_a_negative_double_literal() {
    expect_parse("-1.23", num(-1.23));
}

#[test]
fn should_parse_unary_operators() {
    expect_parse("!a", Expression::unary(UnaryOp::Not, id("a")));
    expect_parse("-a", Expression::unary(UnaryOp::Minus, id("a")));
}

#[test]
fn should_parse_binary_operators() {
    let operators = [
        "=", "+", "-", "*", "/", "%", "^", "==", "!=", ">", "<", ">=", "<=", "||", "&&", "??", "&",
        "===", "!==", "|", "??",
    ];
    for &op_str in &operators {
        expect_parse(
            &format!("a {} b", op_str),
            Expression::binary(id("a"), op(op_str), id("b")),
        );
        expect_parse(
            &format!("1 {} 2", op_str),
            Expression::binary(num(1.0), op(op_str), num(2.0)),
        );
        expect_parse(
            &format!("this {} null", op_str),
            Expression::binary(id("this"), op(op_str), null()),
        );
    }
}

#[test]
fn should_parse_assign_with_equality() {
    expect_parse(
        "a = c == d",
        Expression::binary(
            Expression::id("a"),
            BinOp::Assign,
            Expression::binary(Expression::id("c"), BinOp::Equal, Expression::id("d")),
        ),
    );

    expect_parse(
        "a == c = d",
        Expression::binary(
            Expression::binary(Expression::id("a"), BinOp::Equal, Expression::id("c")),
            BinOp::Assign,
            Expression::id("d"),
        ),
    );
}

#[test]
fn should_parse_arrow_functions() {
    expect_parse("() => x", Expression::arrow_func(vec![], id("x")));
    expect_parse("(a) => a", Expression::arrow_func(vec!["a"], id("a")));
    expect_parse(
        "(a, b) => a + b",
        Expression::arrow_func(
            vec!["a", "b"],
            Expression::binary(id("a"), BinOp::Add, id("b")),
        ),
    );
    expect_parse(
        "fn(() => x)",
        Expression::invoke(
            id("fn"),
            None,
            vec![Expression::arrow_func(vec![], id("x"))],
        ),
    );
    expect_parse(
        "fn ?? () => x",
        Expression::binary(
            id("fn"),
            BinOp::NullishCoalesce,
            Expression::arrow_func(vec![], id("x")),
        ),
    );
    expect_parse(
        "(() => x)()",
        Expression::invoke(
            Expression::paren(Expression::arrow_func(vec![], id("x"))),
            None,
            vec![],
        ),
    );
    expect_parse(
        "(a => x)",
        Expression::paren(Expression::arrow_func(vec!["a"], id("x"))),
    );
    expect_parse("a => x", Expression::arrow_func(vec!["a"], id("x")));
}

#[test]
fn should_give_multiply_higher_associativity_than_plus() {
    expect_parse(
        "a + b * c",
        Expression::binary(
            id("a"),
            BinOp::Add,
            Expression::binary(id("b"), BinOp::Multiply, id("c")),
        ),
    );
    expect_parse(
        "a * b + c",
        Expression::binary(
            Expression::binary(id("a"), BinOp::Multiply, id("b")),
            BinOp::Add,
            id("c"),
        ),
    );
}

#[test]
fn should_parse_a_dot_operator() {
    expect_parse("a.b", Expression::getter(id("a"), "b"));
}

#[test]
fn should_parse_chained_dot_operators() {
    expect_parse(
        "a.b.c",
        Expression::getter(Expression::getter(id("a"), "b"), "c"),
    );
}

#[test]
fn should_give_dot_high_associativity() {
    expect_parse(
        "a * b.c",
        Expression::binary(id("a"), BinOp::Multiply, Expression::getter(id("b"), "c")),
    );
}

#[test]
fn should_parse_a_function_with_no_arguments() {
    expect_parse("a()", Expression::invoke(id("a"), None, vec![]));
}

#[test]
fn should_parse_a_single_function_argument() {
    expect_parse("a(b)", Expression::invoke(id("a"), None, vec![id("b")]));
}

#[test]
fn should_parse_a_function_call_as_a_subexpression() {
    expect_parse(
        "a() + 1",
        Expression::binary(
            Expression::invoke(id("a"), None, vec![]),
            BinOp::Add,
            num(1.0),
        ),
    );
}

#[test]
fn should_parse_multiple_function_arguments() {
    expect_parse(
        "a(b, c)",
        Expression::invoke(id("a"), None, vec![id("b"), id("c")]),
    );
}

#[test]
fn should_parse_nested_function_calls() {
    expect_parse(
        "a(b(c))",
        Expression::invoke(
            id("a"),
            None,
            vec![Expression::invoke(id("b"), None, vec![id("c")])],
        ),
    );
}

#[test]
fn should_parse_an_empty_method_call() {
    expect_parse("a.b()", Expression::invoke(id("a"), Some("b"), vec![]));
}

#[test]
fn should_parse_a_method_call_with_a_single_argument() {
    expect_parse(
        "a.b(c)",
        Expression::invoke(id("a"), Some("b"), vec![id("c")]),
    );
}

#[test]
fn should_parse_a_method_call_with_multiple_arguments() {
    expect_parse(
        "a.b(c, d)",
        Expression::invoke(id("a"), Some("b"), vec![id("c"), id("d")]),
    );
}

#[test]
fn should_parse_chained_method_calls() {
    expect_parse(
        "a.b().c()",
        Expression::invoke(
            Expression::invoke(id("a"), Some("b"), vec![]),
            Some("c"),
            vec![],
        ),
    );
}

#[test]
fn should_parse_chained_function_calls() {
    expect_parse(
        "a()()",
        Expression::invoke(Expression::invoke(id("a"), None, vec![]), None, vec![]),
    );
}

#[test]
fn should_parse_parenthesized_expression() {
    expect_parse("(a)", Expression::paren(id("a")));
    expect_parse(
        "(( 3 * ((1 + 2)) ))",
        Expression::paren(Expression::paren(Expression::binary(
            num(3.0),
            BinOp::Multiply,
            Expression::paren(Expression::paren(Expression::binary(
                num(1.0),
                BinOp::Add,
                num(2.0),
            ))),
        ))),
    );
}

#[test]
fn should_parse_an_index_operator() {
    expect_parse("a[b]", Expression::index(id("a"), Some(id("b"))));
    expect_parse(
        "a.b[c]",
        Expression::index(Expression::getter(id("a"), "b"), Some(id("c"))),
    );
}

#[test]
fn should_parse_chained_index_operators() {
    expect_parse(
        "a[][]",
        Expression::index(Expression::index(id("a"), None), None),
    );
}

#[test]
fn should_parse_multiple_index_operators() {
    expect_parse(
        "a[b] + c[d]",
        Expression::binary(
            Expression::index(id("a"), Some(id("b"))),
            BinOp::Add,
            Expression::index(id("c"), Some(id("d"))),
        ),
    );
}

#[test]
fn should_parse_ternary_operators() {
    expect_parse("a ? b : c", Expression::ternary(id("a"), id("b"), id("c")));
    expect_parse(
        "a.a ? b.a : c.a",
        Expression::ternary(
            Expression::getter(id("a"), "a"),
            Expression::getter(id("b"), "a"),
            Expression::getter(id("c"), "a"),
        ),
    );
}

#[test]
fn ternary_operators_have_lowest_associativity() {
    expect_parse(
        "a == b ? c + d : e - f",
        Expression::ternary(
            Expression::binary(id("a"), BinOp::Equal, id("b")),
            Expression::binary(id("c"), BinOp::Add, id("d")),
            Expression::binary(id("e"), BinOp::Subtract, id("f")),
        ),
    );
    expect_parse(
        "a.x == b.y ? c + d : e - f",
        Expression::ternary(
            Expression::binary(
                Expression::getter(id("a"), "x"),
                BinOp::Equal,
                Expression::getter(id("b"), "y"),
            ),
            Expression::binary(id("c"), BinOp::Add, id("d")),
            Expression::binary(id("e"), BinOp::Subtract, id("f")),
        ),
    );
}

#[test]
fn should_parse_a_filter_chain() {
    expect_parse(
        "a | b | c",
        Expression::binary(
            Expression::binary(id("a"), BinOp::BitwiseOr, id("b")),
            BinOp::BitwiseOr,
            id("c"),
        ),
    );
}

#[test]
fn should_parse_map_literals() {
    expect_parse(
        "{'a': 1}",
        Expression::map(HashMap::from([(Cow::Borrowed("a"), num(1.0))])),
    );
    expect_parse(
        "{a: 1}",
        Expression::map(HashMap::from([(Cow::Borrowed("a"), num(1.0))])),
    );
    expect_parse(
        "{'a': 1, 'b': 2 + 3}",
        Expression::map(HashMap::from([
            (Cow::Borrowed("a"), num(1.0)),
            (
                Cow::Borrowed("b"),
                Expression::binary(num(2.0), BinOp::Add, num(3.0)),
            ),
        ])),
    );
    expect_parse(
        "{'a': foo()}",
        Expression::map(HashMap::from([(
            Cow::Borrowed("a"),
            Expression::invoke(id("foo"), None, vec![]),
        )])),
    );
    expect_parse(
        "{'a': foo('a')}",
        Expression::map(HashMap::from([(
            Cow::Borrowed("a"),
            Expression::invoke(id("foo"), None, vec![string("a")]),
        )])),
    );
}

#[test]
fn should_parse_map_literals_with_method_calls() {
    expect_parse(
        "{'a': 1}.length",
        Expression::getter(
            Expression::map(HashMap::from([(Cow::Borrowed("a"), num(1.0))])),
            "length",
        ),
    );
}

#[test]
fn should_parse_list_literals() {
    expect_parse(
        "[1, \"a\", b]",
        Expression::list(vec![num(1.0), string("a"), id("b")]),
    );
    expect_parse(
        "[[1, 2], [3, 4]]",
        Expression::list(vec![
            Expression::list(vec![num(1.0), num(2.0)]),
            Expression::list(vec![num(3.0), num(4.0)]),
        ]),
    );
}
