use crate::core::error::{Error, Result};
use crate::parser::grammar::{CharClass, CharClassItem, Expr, Grammar, Rule};
pub struct PegParser {
    input: Vec<char>,
    pos: usize,
    line: usize,
    col: usize,
}

impl PegParser {
    pub fn new(input: &str) -> Self {
        Self {
            input: input.chars().collect(),
            pos: 0,
            line: 1,
            col: 1,
        }
    }

    pub fn parse(&mut self) -> Result<Grammar> {
        let mut rules = Vec::new();
        let mut start_rule = None;

        self.skip_whitespace_and_comments();
        while self.peek().is_some() {
            let rule = self.parse_rule()?;
            if start_rule.is_none() {
                start_rule = Some(rule.name.clone());
            }
            rules.push(rule);
            self.skip_whitespace_and_comments();
        }

        let start = start_rule.ok_or_else(|| "No rules defined".to_string())?;
        Ok(Grammar { start, rules })
    }

    fn parse_rule(&mut self) -> Result<Rule> {
        self.skip_whitespace_and_comments();
        let name = self.parse_identifier()?;
        self.skip_whitespace_and_comments();
        self.expect_str("<-")?;
        self.skip_whitespace_and_comments();
        let expr = self.parse_expr()?;
        self.skip_whitespace_and_comments();

        let ast = if self.peek() == Some('{') {
            self.advance();
            let mut depth = 1;
            let mut code = String::new();
            while let Some(c) = self.peek() {
                if c == '{' {
                    depth += 1;
                } else if c == '}' {
                    depth -= 1;
                    if depth == 0 {
                        self.advance();
                        break;
                    }
                }
                code.push(c);
                self.advance();
            }

            let trimmed = code.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        } else {
            None
        };

        Ok(Rule { name, expr, ast })
    }

    fn parse_expr(&mut self) -> Result<Expr> {
        self.parse_choice()
    }

    fn parse_choice(&mut self) -> Result<Expr> {
        let mut exprs = vec![self.parse_sequence()?];
        while self.skip_whitespace_and_comments() && self.peek() == Some('/') {
            self.advance();
            self.skip_whitespace_and_comments();
            exprs.push(self.parse_sequence()?);
        }
        Ok(Expr::choice(exprs))
    }

    fn parse_sequence(&mut self) -> Result<Expr> {
        let mut exprs = Vec::new();
        self.skip_whitespace_and_comments();
        while let Some(c) = self.peek() {
            if c == '/' || c == ')' || c == '*' || c == '+' || c == '?' || c == '&' || c == '!' {
                break;
            }
            exprs.push(self.parse_prefix()?);
            self.skip_whitespace_and_comments();
        }
        if exprs.is_empty() {
            Err(self.error("Expected expression"))
        } else {
            Ok(Expr::seq(exprs))
        }
    }

    fn parse_prefix(&mut self) -> Result<Expr> {
        self.skip_whitespace_and_comments();
        match self.peek() {
            Some('&') => {
                self.advance();
                let inner = self.parse_prefix()?;
                Ok(Expr::AndPredicate(Box::new(inner)))
            }
            Some('!') => {
                self.advance();
                let inner = self.parse_prefix()?;
                Ok(Expr::NotPredicate(Box::new(inner)))
            }
            _ => self.parse_suffix(),
        }
    }

    fn parse_suffix(&mut self) -> Result<Expr> {
        let mut expr = self.parse_primary()?;
        while let Some(c) = self.peek() {
            match c {
                '*' => {
                    self.advance();
                    expr = Expr::Repeat(Box::new(expr));
                }
                '+' => {
                    self.advance();
                    expr = Expr::Plus(Box::new(expr));
                }
                '?' => {
                    self.advance();
                    expr = Expr::Optional(Box::new(expr));
                }
                _ => break,
            }
        }
        Ok(expr)
    }

    fn parse_primary(&mut self) -> Result<Expr> {
        self.skip_whitespace_and_comments();
        match self.peek() {
            Some('(') => {
                self.advance();
                let expr = self.parse_expr()?;
                self.expect(')')?;
                Ok(Expr::Group(Box::new(expr)))
            }
            Some('\'') | Some('"') => self.parse_string_literal(),
            Some(c) if c.is_ascii_alphabetic() || c == '_' => {
                let name = self.parse_identifier()?;
                if name == "EOF" {
                    Ok(Expr::Eof)
                } else {
                    Ok(Expr::RuleRef(name))
                }
            }
            Some('.') => {
                self.advance();
                Ok(Expr::AnyChar)
            }
            Some('[') => self.parse_char_class(),
            _ => Err(self.error("Unexpected token in expression")),
        }
    }

    fn parse_string_literal(&mut self) -> Result<Expr> {
        let quote = self.consume().ok_or_else(|| self.error("Expected quote"))?;
        let mut s = String::new();
        while let Some(c) = self.peek() {
            if c == quote {
                self.advance();
                break;
            }
            if c == '\\' {
                self.advance();
                let escaped = self
                    .consume()
                    .ok_or_else(|| self.error("Escape at end of string"))?;
                let ch = match escaped {
                    'n' => '\n',
                    't' => '\t',
                    'r' => '\r',
                    '\\' => '\\',
                    '\'' => '\'',
                    '"' => '"',
                    _ => return Err(self.error(&format!("Invalid escape sequence: \\{}", escaped))),
                };
                s.push(ch);
            } else {
                s.push(c);
                self.advance();
            }
        }
        Ok(crate::parser::grammar::string_literal_to_expr(&s))
    }

    fn parse_char_class(&mut self) -> Result<Expr> {
        self.expect('[')?;
        let negated = if self.peek() == Some('^') {
            self.advance();
            true
        } else {
            false
        };
        let mut items = Vec::new();
        while let Some(c) = self.peek() {
            if c == ']' {
                self.advance();
                break;
            }
            let start = c;
            self.advance();
            let end = if self.peek() == Some('-') {
                self.advance();
                self.consume()
                    .ok_or_else(|| self.error("Expected character after '-'"))?
            } else {
                start
            };
            if start > end {
                return Err(self.error(&format!("Invalid range: {} > {}", start, end)));
            }
            items.push(CharClassItem {
                start: start as u32,
                end: end as u32,
            });
        }
        let class = CharClass { negated, items };
        Ok(Expr::CharClass(class))
    }

    fn parse_identifier(&mut self) -> Result<String> {
        self.skip_whitespace_and_comments();
        let mut name = String::new();
        if let Some(c) = self.peek() {
            if c.is_ascii_alphabetic() || c == '_' {
                name.push(c);
                self.advance();
                while let Some(c2) = self.peek() {
                    if c2.is_ascii_alphanumeric() || c2 == '_' {
                        name.push(c2);
                        self.advance();
                    } else {
                        break;
                    }
                }
                Ok(name)
            } else {
                Err(self.error(&format!("Expected identifier, found '{}'", c)))
            }
        } else {
            Err(self.error("Expected identifier"))
        }
    }

    fn expect(&mut self, expected: char) -> Result<()> {
        if let Some(c) = self.consume() {
            if c == expected {
                Ok(())
            } else {
                Err(self.error(&format!("Expected '{}', found '{}'", expected, c)))
            }
        } else {
            Err(self.error(&format!("Expected '{}'", expected)))
        }
    }

    fn expect_str(&mut self, s: &str) -> Result<()> {
        for ch in s.chars() {
            self.expect(ch)?;
        }
        Ok(())
    }

    fn peek(&self) -> Option<char> {
        self.input.get(self.pos).copied()
    }

    fn advance(&mut self) {
        if let Some(c) = self.peek() {
            self.pos += 1;
            if c == '\n' {
                self.line += 1;
                self.col = 1;
            } else {
                self.col += 1;
            }
        }
    }

    fn consume(&mut self) -> Option<char> {
        let c = self.peek()?;
        self.advance();
        Some(c)
    }

    fn skip_whitespace_and_comments(&mut self) -> bool {
        let mut skipped = false;
        while let Some(c) = self.peek() {
            if c.is_whitespace() {
                self.advance();
                skipped = true;
            } else if c == '/' && self.peek_ahead(1) == Some('/') {
                self.advance();
                self.advance();
                while let Some(ch) = self.peek() {
                    if ch == '\n' {
                        break;
                    }
                    self.advance();
                }
                skipped = true;
            } else {
                break;
            }
        }
        skipped
    }

    fn peek_ahead(&self, offset: usize) -> Option<char> {
        self.input.get(self.pos + offset).copied()
    }

    fn error(&self, msg: &str) -> Error {
        Error::parse(msg, self.line, self.col)
    }
}
