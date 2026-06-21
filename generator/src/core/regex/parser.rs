use crate::core::error::{Error, Result};
use crate::core::interval::Interval;
use crate::core::regex::ast::{CharClass, CharClassItem, RegexType, RepeatType};

pub struct Parser {
    chars: Vec<char>,
    pos: usize,
    pub line: usize,
    pub col: usize,
}

impl Parser {
    pub fn new(input: &str) -> Self {
        Parser {
            chars: input.chars().collect(),
            pos: 0,
            line: 1,
            col: 1,
        }
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn consume(&mut self) -> Option<char> {
        let ch = self.peek()?;
        self.pos += 1;
        if ch == '\n' {
            self.line += 1;
            self.col = 1;
        } else {
            self.col += 1;
        }
        Some(ch)
    }

    fn consume_expected(&mut self, expected: char) -> Result<()> {
        if let Some(ch) = self.consume() {
            if ch == expected {
                Ok(())
            } else {
                Err(self.error_at_pos(&format!("Expected '{}', got '{}'", expected, ch)))
            }
        } else {
            Err(self.error_at_pos(&format!("Expected '{}', got EOF", expected)))
        }
    }

    fn error_at_pos(&self, msg: &str) -> Error {
        Error::parse(msg, self.line, self.col)
    }

    fn error_with_context(&self, msg: &str, context: &str) -> Error {
        Error::parse(
            format!("{} (context: {})", msg, context),
            self.line,
            self.col,
        )
    }

    pub fn parse(&mut self) -> Result<RegexType> {
        let expr = self.parse_alt()?;
        if self.pos < self.chars.len() {
            let ch = self.peek().unwrap();
            return Err(self.error_with_context(
                &format!("Unexpected character '{}'", ch),
                &self.chars[self.pos..].iter().take(20).collect::<String>(),
            ));
        }
        Ok(expr)
    }

    fn parse_alt(&mut self) -> Result<RegexType> {
        let mut left = self.parse_concat()?;
        while self.peek() == Some('|') {
            self.consume().unwrap();
            let right = self.parse_concat()?;
            left = RegexType::Alt(Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn parse_concat(&mut self) -> Result<RegexType> {
        let mut nodes = Vec::new();
        while let Some(ch) = self.peek() {
            if ch == '|' || ch == ')' {
                break;
            }
            nodes.push(self.parse_item()?);
        }
        if nodes.is_empty() {
            return Err(self.error_at_pos("Unexpected end of pattern"));
        }
        let mut iter = nodes.into_iter();
        let mut result = iter.next().unwrap();
        for node in iter {
            result = RegexType::Concat(Box::new(result), Box::new(node));
        }
        Ok(result)
    }

    fn parse_item(&mut self) -> Result<RegexType> {
        let expr = match self.peek() {
            Some('(') => {
                self.consume().unwrap();
                let sub = self.parse_alt()?;
                self.consume_expected(')')?;
                sub
            }
            Some('[') => {
                self.consume().unwrap();
                self.parse_char_class()?
            }
            Some('\\') => {
                self.consume().unwrap();
                self.parse_escape()?
            }
            Some('.') => {
                self.consume().unwrap();
                let items = vec![
                    CharClassItem {
                        start: 0,
                        end: '\n' as u32 - 1,
                    },
                    CharClassItem {
                        start: '\n' as u32 + 1,
                        end: 0x10FFFF,
                    },
                ];
                RegexType::CharClass(CharClass {
                    negated: false,
                    items,
                })
            }
            Some(ch) => {
                self.consume().unwrap();
                RegexType::Char(ch as u32)
            }
            None => return Err(self.error_at_pos("Unexpected end of pattern")),
        };

        let mut expr = expr;
        while let Some(op) = self.peek() {
            match op {
                '*' => {
                    self.consume().unwrap();
                    expr = RegexType::Star(Box::new(expr));
                }
                '+' => {
                    self.consume().unwrap();
                    expr = RegexType::Plus(Box::new(expr));
                }
                '?' => {
                    self.consume().unwrap();
                    expr = RegexType::Opt(Box::new(expr));
                }
                '{' => {
                    let kind = self.parse_quantifier()?;
                    expr = RegexType::Repeat(Box::new(expr), kind);
                }
                _ => break,
            }
        }
        Ok(expr)
    }

    fn parse_char_class(&mut self) -> Result<RegexType> {
        let negated = if self.peek() == Some('^') {
            self.consume().unwrap();
            true
        } else {
            false
        };

        let mut items = Vec::new();
        while let Some(ch) = self.peek() {
            if ch == ']' {
                break;
            }
            let first = self.parse_char_literal()?;
            if self.peek() == Some('-') {
                self.consume().unwrap();
                if self.peek() == Some(']') {
                    items.push(CharClassItem {
                        start: first,
                        end: first,
                    });
                    items.push(CharClassItem {
                        start: '-' as u32,
                        end: '-' as u32,
                    });
                    continue;
                } else {
                    let second = self.parse_char_literal()?;
                    if first > second {
                        return Err(self.error_at_pos(&format!(
                            "Invalid range: 0x{:X} > 0x{:X}",
                            first, second
                        )));
                    }
                    items.push(CharClassItem {
                        start: first,
                        end: second,
                    });
                }
            } else {
                items.push(CharClassItem {
                    start: first,
                    end: first,
                });
            }
        }
        self.consume_expected(']')?;

        let merged = merge_intervals(items);
        Ok(RegexType::CharClass(CharClass {
            negated,
            items: merged,
        }))
    }

    fn parse_char_literal(&mut self) -> Result<u32> {
        match self.peek() {
            Some('\\') => {
                self.consume().unwrap();
                let ch = self
                    .consume()
                    .ok_or_else(|| self.error_at_pos("Expected escape character"))?;
                self.escape_char_to_codepoint(ch)
            }
            Some(ch) => {
                self.consume().unwrap();
                Ok(ch as u32)
            }
            None => Err(self.error_at_pos("Unexpected end of pattern in character class")),
        }
    }

    fn parse_quantifier(&mut self) -> Result<RepeatType> {
        self.consume_expected('{')?;
        let n = self.parse_number()?;
        match self.peek() {
            Some('}') => {
                self.consume().unwrap();
                Ok(RepeatType::Exactly(n))
            }
            Some(',') => {
                self.consume().unwrap();
                if self.peek() == Some('}') {
                    self.consume().unwrap();
                    Ok(RepeatType::AtLeast(n))
                } else {
                    let m = self.parse_number()?;
                    self.consume_expected('}')?;
                    if n > m {
                        return Err(self.error_at_pos(&format!(
                            "Invalid quantifier {{ {}, {} }}: lower bound > upper bound",
                            n, m
                        )));
                    }
                    Ok(RepeatType::Between(n, m))
                }
            }
            _ => Err(self.error_at_pos("Invalid quantifier syntax")),
        }
    }

    fn parse_number(&mut self) -> Result<u32> {
        let mut num_str = String::new();
        let start_line = self.line;
        let start_col = self.col;
        while let Some(ch) = self.peek() {
            if ch.is_ascii_digit() {
                num_str.push(ch);
                self.consume().unwrap();
            } else {
                break;
            }
        }
        if num_str.is_empty() {
            return Err(Error::parse(
                "Expected number in quantifier",
                start_line,
                start_col,
            ));
        }
        num_str
            .parse()
            .map_err(|_| Error::parse("Number too large", start_line, start_col))
    }

    fn parse_escape(&mut self) -> Result<RegexType> {
        let ch = self
            .consume()
            .ok_or_else(|| self.error_at_pos("Expected escape character"))?;
        match ch {
            'd' => Ok(RegexType::CharClass(CharClass {
                negated: false,
                items: vec![CharClassItem {
                    start: '0' as u32,
                    end: '9' as u32,
                }],
            })),
            'D' => Ok(RegexType::CharClass(CharClass {
                negated: true,
                items: vec![CharClassItem {
                    start: '0' as u32,
                    end: '9' as u32,
                }],
            })),
            'w' => {
                let mut items = vec![
                    CharClassItem {
                        start: 'a' as u32,
                        end: 'z' as u32,
                    },
                    CharClassItem {
                        start: 'A' as u32,
                        end: 'Z' as u32,
                    },
                    CharClassItem {
                        start: '0' as u32,
                        end: '9' as u32,
                    },
                    CharClassItem {
                        start: '_' as u32,
                        end: '_' as u32,
                    },
                ];
                items.sort_by_key(|i| i.start);
                let merged = merge_intervals(items);
                Ok(RegexType::CharClass(CharClass {
                    negated: false,
                    items: merged,
                }))
            }
            'W' => {
                let items = vec![
                    CharClassItem {
                        start: 'a' as u32,
                        end: 'z' as u32,
                    },
                    CharClassItem {
                        start: 'A' as u32,
                        end: 'Z' as u32,
                    },
                    CharClassItem {
                        start: '0' as u32,
                        end: '9' as u32,
                    },
                    CharClassItem {
                        start: '_' as u32,
                        end: '_' as u32,
                    },
                ];
                Ok(RegexType::CharClass(CharClass {
                    negated: true,
                    items,
                }))
            }
            's' => {
                let items = vec![
                    CharClassItem {
                        start: ' ' as u32,
                        end: ' ' as u32,
                    },
                    CharClassItem {
                        start: '\t' as u32,
                        end: '\t' as u32,
                    },
                    CharClassItem {
                        start: '\n' as u32,
                        end: '\n' as u32,
                    },
                    CharClassItem {
                        start: '\r' as u32,
                        end: '\r' as u32,
                    },
                    CharClassItem {
                        start: '\x0C' as u32,
                        end: '\x0C' as u32,
                    },
                    CharClassItem {
                        start: '\x0B' as u32,
                        end: '\x0B' as u32,
                    },
                ];
                Ok(RegexType::CharClass(CharClass {
                    negated: false,
                    items,
                }))
            }
            'S' => Ok(RegexType::CharClass(CharClass {
                negated: true,
                items: vec![
                    CharClassItem {
                        start: ' ' as u32,
                        end: ' ' as u32,
                    },
                    CharClassItem {
                        start: '\t' as u32,
                        end: '\t' as u32,
                    },
                    CharClassItem {
                        start: '\n' as u32,
                        end: '\n' as u32,
                    },
                    CharClassItem {
                        start: '\r' as u32,
                        end: '\r' as u32,
                    },
                    CharClassItem {
                        start: '\x0C' as u32,
                        end: '\x0C' as u32,
                    },
                    CharClassItem {
                        start: '\x0B' as u32,
                        end: '\x0B' as u32,
                    },
                ],
            })),
            'u' => {
                let cp = self.escape_char_to_codepoint(ch)?;
                Ok(RegexType::Char(cp))
            }
            'n' | 't' | 'r' | '\\' | '(' | ')' | '|' | '*' | '+' | '?' | '[' | ']' | '{' | '}'
            | '-' => {
                let cp = self.escape_char_to_codepoint(ch)?;
                Ok(RegexType::Char(cp))
            }
            _ => Err(self.error_at_pos(&format!("Invalid escape sequence: \\{}", ch))),
        }
    }

    fn escape_char_to_codepoint(&mut self, ch: char) -> Result<u32> {
        let cp = match ch {
            'n' => '\n' as u32,
            't' => '\t' as u32,
            'r' => '\r' as u32,
            '\\' => '\\' as u32,
            '(' => '(' as u32,
            ')' => ')' as u32,
            '|' => '|' as u32,
            '*' => '*' as u32,
            '+' => '+' as u32,
            '?' => '?' as u32,
            '[' => '[' as u32,
            ']' => ']' as u32,
            '{' => '{' as u32,
            '}' => '}' as u32,
            '-' => '-' as u32,
            'u' => {
                self.consume_expected('{')?;
                let mut hex_str = String::new();
                while let Some(c) = self.peek() {
                    if c == '}' {
                        break;
                    }
                    if !c.is_ascii_hexdigit() {
                        return Err(self
                            .error_at_pos(&format!("Invalid hex digit in Unicode escape: {}", c)));
                    }
                    hex_str.push(c);
                    self.consume().unwrap();
                }
                self.consume_expected('}')?;
                let code = u32::from_str_radix(&hex_str, 16).map_err(|_| {
                    self.error_at_pos(&format!("Invalid Unicode code point: {}", hex_str))
                })?;
                std::char::from_u32(code).ok_or_else(|| {
                    self.error_at_pos(&format!("Invalid Unicode code point: 0x{:X}", code))
                })?;
                code
            }
            _ => ch as u32,
        };
        Ok(cp)
    }
}

pub fn merge_intervals(items: Vec<CharClassItem>) -> Vec<CharClassItem> {
    let intervals: Vec<Interval> = items
        .into_iter()
        .map(|item| Interval::new(item.start, item.end + 1))
        .collect();
    let merged = Interval::merge(intervals);
    merged
        .into_iter()
        .map(|int| CharClassItem {
            start: int.start,
            end: int.end - 1,
        })
        .collect()
}
