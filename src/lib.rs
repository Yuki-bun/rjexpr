#[macro_use]
mod helpers;
mod iter;
mod parser;

use std::{collections::HashMap, hash::Hash};

pub use parser::parse;

#[derive(Debug, Clone, PartialEq)]
pub enum Literal<S> {
    String(S),
    Number(f64),
    Boolean(bool),
    Null,
    Undefined,
}

impl<S> Literal<S> {
    fn string(str: impl Into<S>) -> Self {
        Self::String(str.into())
    }
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
pub enum Expression<S, B: Eq + Hash> {
    Literal(Literal<B>),
    Empty,
    ID(S),
    CustomID(S),
    Unary(UnaryOp, Box<Expression<S, B>>),
    Binary {
        op: BinOp,
        left: Box<Expression<S, B>>,
        right: Box<Expression<S, B>>,
    },
    Getter(Box<Expression<S, B>>, S),
    Index {
        receiver: Box<Expression<S, B>>,
        argument: Option<Box<Expression<S, B>>>,
    },
    Invoke {
        receiver: Box<Expression<S, B>>,
        method: Option<S>,
        arguments: Vec<Expression<S, B>>,
    },
    Ternary {
        cond: Box<Expression<S, B>>,
        left: Box<Expression<S, B>>,
        right: Box<Expression<S, B>>,
    },
    Map(HashMap<B, Expression<S, B>>),
    List(Vec<Expression<S, B>>),
    ArrowFunc {
        params: Vec<S>,
        body: Box<Expression<S, B>>,
    },
}

impl<S, B> Expression<S, B>
where
    B: Eq + Hash,
{
    // Constructors

    pub fn literal(literal: Literal<B>) -> Self {
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

    pub fn id(name: impl Into<S>) -> Self {
        Self::ID(name.into())
    }

    pub fn custom_id(name: impl Into<S>) -> Self {
        Self::CustomID(name.into())
    }

    pub fn unary(op: UnaryOp, operand: Self) -> Self {
        Self::Unary(op, Box::new(operand))
    }

    pub fn getter(object: Self, key: impl Into<S>) -> Self {
        Self::Getter(Box::new(object), key.into())
    }

    pub fn index(receiver: Self, argument: Option<Self>) -> Self {
        Self::Index {
            receiver: Box::new(receiver),
            argument: argument.map(Box::new),
        }
    }

    pub fn invoke(receiver: Self, method: Option<impl Into<S>>, arguments: Vec<Self>) -> Self {
        Self::Invoke {
            receiver: Box::new(receiver),
            method: method.map(Into::into),
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

    pub fn map(entries: HashMap<B, Self>) -> Self {
        Self::Map(entries)
    }

    pub fn list(items: Vec<Self>) -> Self {
        Self::List(items)
    }

    pub fn arrow_func(params: Vec<S>, body: Self) -> Self {
        Self::ArrowFunc {
            params,
            body: Box::new(body),
        }
    }

    // Accessors

    pub fn as_literal(&self) -> Option<&Literal<B>> {
        match self {
            Self::Literal(lit) => Some(lit),
            _ => None,
        }
    }

    pub fn is_empty(&self) -> bool {
        matches!(self, Self::Empty)
    }

    pub fn take_id(self) -> Option<S> {
        match self {
            Self::ID(name) => Some(name),
            _ => None,
        }
    }

    pub fn as_id(&self) -> Option<&S> {
        match self {
            Self::ID(name) => Some(name),
            _ => None,
        }
    }

    pub fn as_custom_id(&self) -> Option<&S> {
        match self {
            Self::CustomID(name) => Some(name),
            _ => None,
        }
    }

    pub fn as_unary(&self) -> Option<(UnaryOp, &Self)> {
        match self {
            Self::Unary(op, operand) => Some((*op, operand.as_ref())),
            _ => None,
        }
    }

    pub fn as_binary(&self) -> Option<(BinOp, &Self, &Self)> {
        match self {
            Self::Binary { op, left, right } => Some((*op, left.as_ref(), right.as_ref())),
            _ => None,
        }
    }

    pub fn as_getter(&self) -> Option<(&Self, &S)> {
        match self {
            Self::Getter(object, key) => Some((object.as_ref(), key)),
            _ => None,
        }
    }

    pub fn as_index(&self) -> Option<(&Self, Option<&Self>)> {
        match self {
            Self::Index { receiver, argument } => {
                Some((receiver.as_ref(), argument.as_ref().map(Box::as_ref)))
            }
            _ => None,
        }
    }

    pub fn as_invoke(&self) -> Option<(&Self, Option<&'_ S>, &[Self])> {
        match self {
            Self::Invoke {
                receiver,
                method,
                arguments,
            } => Some((receiver.as_ref(), method.as_ref(), arguments.as_slice())),
            _ => None,
        }
    }

    pub fn as_ternary(&self) -> Option<(&Self, &Self, &Self)> {
        match self {
            Self::Ternary { cond, left, right } => {
                Some((cond.as_ref(), left.as_ref(), right.as_ref()))
            }
            _ => None,
        }
    }

    pub fn as_map(&self) -> Option<&HashMap<B, Self>> {
        match self {
            Self::Map(map) => Some(map),
            _ => None,
        }
    }

    pub fn as_list(&self) -> Option<&[Self]> {
        match self {
            Self::List(items) => Some(items.as_slice()),
            _ => None,
        }
    }

    pub fn as_arrow_func(&self) -> Option<(&[S], &Self)> {
        match self {
            Self::ArrowFunc { params, body } => Some((params.as_slice(), body.as_ref())),
            _ => None,
        }
    }
}
