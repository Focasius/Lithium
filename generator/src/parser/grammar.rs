use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Grammar {
    pub start: String,
    pub rules: Vec<Rule>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    pub name: String,
    pub expr: Expr,

    pub ast: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharClassItem {
    pub start: u32,
    pub end: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharClass {
    pub negated: bool,
    pub items: Vec<CharClassItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum Expr {
    Sequence(Vec<Expr>),

    Choice(Vec<Expr>),

    Repeat(Box<Expr>),

    Plus(Box<Expr>),

    Optional(Box<Expr>),

    AndPredicate(Box<Expr>),

    NotPredicate(Box<Expr>),

    Group(Box<Expr>),

    RuleRef(String),

    Char(u32),

    String(String),

    Regex(String),

    CharClass(CharClass),

    Eof,

    AnyChar,
}

impl Expr {
    pub fn seq(exprs: Vec<Expr>) -> Self {
        if exprs.is_empty() {
            panic!("seq called with empty list");
        }
        if exprs.len() == 1 {
            return exprs.into_iter().next().unwrap();
        }
        let mut flat = Vec::new();
        for e in exprs {
            match e {
                Expr::Sequence(inner) => flat.extend(inner),
                _ => flat.push(e),
            }
        }
        if flat.len() == 1 {
            flat.into_iter().next().unwrap()
        } else {
            Expr::Sequence(flat)
        }
    }

    pub fn choice(exprs: Vec<Expr>) -> Self {
        if exprs.is_empty() {
            panic!("choice called with empty list");
        }
        if exprs.len() == 1 {
            return exprs.into_iter().next().unwrap();
        }
        let mut flat = Vec::new();
        for e in exprs {
            match e {
                Expr::Choice(inner) => flat.extend(inner),
                _ => flat.push(e),
            }
        }
        if flat.len() == 1 {
            flat.into_iter().next().unwrap()
        } else {
            Expr::Choice(flat)
        }
    }
}

pub fn string_literal_to_expr(s: &str) -> Expr {
    let chars: Vec<_> = s.chars().map(|c| Expr::Char(c as u32)).collect();
    if chars.len() == 1 {
        chars.into_iter().next().unwrap()
    } else {
        Expr::Sequence(chars)
    }
}
