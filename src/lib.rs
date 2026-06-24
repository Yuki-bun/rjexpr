#[macro_use]
mod helpers;
mod iter;
mod parser;

use std::{borrow::Cow, collections::HashMap, hash::Hash, marker::PhantomData};

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
pub enum Expression<S: StrRepr> {
    Literal(Literal<S::Escaped>),
    Empty,
    ID(S::Str),
    CustomID(S::Str),
    Assign(Box<AssignTarget<S>>, Box<Self>),
    Unary(UnaryOp, Box<Self>),
    Binary {
        op: BinOp,
        left: Box<Self>,
        right: Box<Self>,
    },
    Getter(Box<Self>, S::Str),
    Index {
        receiver: Box<Self>,
        argument: Option<Box<Self>>,
    },
    Invoke {
        receiver: Box<Self>,
        method: Option<S::Str>,
        arguments: Vec<Self>,
    },
    Ternary {
        cond: Box<Self>,
        left: Box<Self>,
        right: Box<Self>,
    },
    Map(HashMap<S::Escaped, Self>),
    List(Vec<Self>),
    ArrowFunc {
        params: Vec<S::Str>,
        body: Box<Self>,
    },
}

pub trait StrRepr {
    type Str: Clone + std::fmt::Debug + PartialEq;
    type Escaped: Clone + std::fmt::Debug + PartialEq + Hash + Eq;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BorrowedRepr<'a>(PhantomData<&'a ()>);
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OwnedRepr;
impl<'a> StrRepr for BorrowedRepr<'a> {
    type Str = &'a str;
    type Escaped = Cow<'a, str>;
}

impl StrRepr for OwnedRepr {
    type Str = String;
    type Escaped = String;
}

#[derive(Clone, Debug, PartialEq)]
pub enum AssignTarget<S: StrRepr> {
    ID(S::Str),
    Getter(Expression<S>, S::Str),
}

impl<S: StrRepr> TryFrom<Expression<S>> for AssignTarget<S> {
    type Error = Expression<S>;

    fn try_from(value: Expression<S>) -> Result<Self, Self::Error> {
        match value {
            Expression::ID(id) => Ok(AssignTarget::ID(id)),
            Expression::Getter(receiver, field) => Ok(AssignTarget::Getter(*receiver, field)),
            val => Err(val),
        }
    }
}

impl<S: StrRepr> From<AssignTarget<S>> for Expression<S> {
    fn from(value: AssignTarget<S>) -> Self {
        match value {
            AssignTarget::ID(id) => Self::ID(id),
            AssignTarget::Getter(receiver, field) => Self::Getter(Box::new(receiver), field),
        }
    }
}

impl<S: StrRepr> Expression<S> {
    // Constructors

    pub fn literal(literal: Literal<S::Escaped>) -> Self {
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

    pub fn id(name: impl Into<S::Str>) -> Self {
        Self::ID(name.into())
    }

    pub fn custom_id(name: impl Into<S::Str>) -> Self {
        Self::CustomID(name.into())
    }

    pub fn assign(assigned: AssignTarget<S>, value: Self) -> Self {
        Self::Assign(Box::new(assigned), Box::new(value))
    }

    pub fn unary(op: UnaryOp, operand: Self) -> Self {
        Self::Unary(op, Box::new(operand))
    }

    pub fn getter(object: Self, key: impl Into<S::Str>) -> Self {
        Self::Getter(Box::new(object), key.into())
    }

    pub fn index(receiver: Self, argument: Option<Self>) -> Self {
        Self::Index {
            receiver: Box::new(receiver),
            argument: argument.map(Box::new),
        }
    }

    pub fn invoke(receiver: Self, method: Option<impl Into<S::Str>>, arguments: Vec<Self>) -> Self {
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

    pub fn map(entries: HashMap<S::Escaped, Self>) -> Self {
        Self::Map(entries)
    }

    pub fn list(items: Vec<Self>) -> Self {
        Self::List(items)
    }

    pub fn arrow_func(params: Vec<S::Str>, body: Self) -> Self {
        Self::ArrowFunc {
            params,
            body: Box::new(body),
        }
    }
}
