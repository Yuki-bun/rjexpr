mod combinator;
mod helpers;

use std::{borrow::Cow, collections::HashMap};

pub use combinator::{DebugP, dbg_p, parse};

#[derive(Debug, Clone, PartialEq)]
pub enum Literal<'a> {
    String(Cow<'a, String>),
    Number(f64),
    Boolean(bool),
    Null,
    Undefined,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Plus,
    Minus,
    Not,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    Assign,          // =
    Add,             // +
    Subtract,        // -
    Multiply,        // *
    Divide,          // /
    Modulo,          // %
    Equal,           // ==
    NotEqual,        // !=
    GreaterThan,     // >
    LessThan,        // <
    GreaterOrEqual,  // >=
    LessOrEqual,     // <=
    LogicalOr,       // ||
    LogicalAnd,      // &&
    NullishCoalesce, // ??
    BitwiseAnd,      // &
    StrictEqual,     // ===
    StrictNotEqual,  // !==
    BitwiseOr,       // |
    BitwiseXor,      // ^
    Pipe,            // |>
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expression<'a> {
    Literal(Literal<'a>),
    Empty,
    ID(&'a str),
    CustomID(&'a str),
    Unary(UnaryOp, Box<Expression<'a>>),
    Binary {
        op: BinOp,
        left: Box<Expression<'a>>,
        right: Box<Expression<'a>>,
    },
    Getter(Box<Expression<'a>>, &'a str),
    Paren(Box<Expression<'a>>),
    Index {
        receiver: Box<Expression<'a>>,
        argument: Option<Box<Expression<'a>>>,
    },
    Invoke {
        receiver: Box<Expression<'a>>,
        method: Option<&'a str>,
        arguments: Vec<Expression<'a>>,
    },
    Ternary {
        cond: Box<Expression<'a>>,
        left: Box<Expression<'a>>,
        right: Box<Expression<'a>>,
    },
    Map(HashMap<Cow<'a, str>, Expression<'a>>),
    List(Vec<Expression<'a>>),
    ArrowFunc {
        params: Vec<&'a str>,
        body: Box<Expression<'a>>,
    },
}

impl<'a> Expression<'a> {
    // Constructors

    pub fn literal(literal: Literal<'a>) -> Self {
        Self::Literal(literal)
    }

    pub fn binary(left: Self, op: BinOp, right: Self) -> Self {
        Self::Binary {
            left: Box::new(left),
            op,
            right: Box::new(right),
        }
    }

    pub fn empty() -> Self {
        Self::Empty
    }

    pub fn id(name: &'a str) -> Self {
        Self::ID(name)
    }

    pub fn custom_id(name: &'a str) -> Self {
        Self::CustomID(name)
    }

    pub fn unary(op: UnaryOp, operand: Self) -> Self {
        Self::Unary(op, Box::new(operand))
    }

    pub fn getter(object: Self, key: &'a str) -> Self {
        Self::Getter(Box::new(object), key)
    }

    pub fn paren(expr: Self) -> Self {
        Self::Paren(Box::new(expr))
    }

    pub fn index(receiver: Self, argument: Option<Self>) -> Self {
        Self::Index {
            receiver: Box::new(receiver),
            argument: argument.map(Box::new),
        }
    }

    pub fn invoke(receiver: Self, method: Option<&'a str>, arguments: Vec<Self>) -> Self {
        Self::Invoke {
            receiver: Box::new(receiver),
            method,
            arguments,
        }
    }

    pub fn ternary(cond: Self, left: Self, right: Self) -> Self {
        Self::Ternary {
            cond: Box::new(cond),
            left: Box::new(left),
            right: Box::new(right),
        }
    }

    pub fn map(entries: HashMap<Cow<'a, str>, Self>) -> Self {
        Self::Map(entries)
    }

    pub fn list(items: Vec<Self>) -> Self {
        Self::List(items)
    }

    pub fn arrow_func(params: Vec<&'a str>, body: Self) -> Self {
        Self::ArrowFunc {
            params,
            body: Box::new(body),
        }
    }

    // Accessors

    pub fn as_literal(&self) -> Option<&Literal<'a>> {
        match self {
            Self::Literal(lit) => Some(lit),
            _ => None,
        }
    }

    pub fn is_empty(&self) -> bool {
        matches!(self, Self::Empty)
    }

    pub fn as_id(&self) -> Option<&'a str> {
        match self {
            Self::ID(name) => Some(*name),
            _ => None,
        }
    }

    pub fn as_custom_id(&self) -> Option<&'a str> {
        match self {
            Self::CustomID(name) => Some(*name),
            _ => None,
        }
    }

    pub fn as_unary(&self) -> Option<(UnaryOp, &Expression<'a>)> {
        match self {
            Self::Unary(op, operand) => Some((*op, operand.as_ref())),
            _ => None,
        }
    }

    pub fn as_binary(&self) -> Option<(BinOp, &Expression<'a>, &Expression<'a>)> {
        match self {
            Self::Binary { op, left, right } => Some((*op, left.as_ref(), right.as_ref())),
            _ => None,
        }
    }

    pub fn as_getter(&self) -> Option<(&Expression<'a>, &'a str)> {
        match self {
            Self::Getter(object, key) => Some((object.as_ref(), *key)),
            _ => None,
        }
    }

    pub fn as_paren(&self) -> Option<&Expression<'a>> {
        match self {
            Self::Paren(expr) => Some(expr.as_ref()),
            _ => None,
        }
    }

    pub fn as_index(&self) -> Option<(&Expression<'a>, Option<&Expression<'a>>)> {
        match self {
            Self::Index { receiver, argument } => {
                Some((receiver.as_ref(), argument.as_ref().map(Box::as_ref)))
            }
            _ => None,
        }
    }

    pub fn as_invoke(&self) -> Option<(&Expression<'a>, Option<&'a str>, &[Expression<'a>])> {
        match self {
            Self::Invoke {
                receiver,
                method,
                arguments,
            } => Some((receiver.as_ref(), *method, arguments.as_slice())),
            _ => None,
        }
    }

    pub fn as_ternary(&self) -> Option<(&Expression<'a>, &Expression<'a>, &Expression<'a>)> {
        match self {
            Self::Ternary { cond, left, right } => {
                Some((cond.as_ref(), left.as_ref(), right.as_ref()))
            }
            _ => None,
        }
    }

    pub fn as_map(&self) -> Option<&HashMap<Cow<'a, str>, Expression<'a>>> {
        match self {
            Self::Map(map) => Some(map),
            _ => None,
        }
    }

    pub fn as_list(&self) -> Option<&[Expression<'a>]> {
        match self {
            Self::List(items) => Some(items.as_slice()),
            _ => None,
        }
    }

    pub fn as_arrow_func(&self) -> Option<(&[&'a str], &Expression<'a>)> {
        match self {
            Self::ArrowFunc { params, body } => Some((params.as_slice(), body.as_ref())),
            _ => None,
        }
    }
}
