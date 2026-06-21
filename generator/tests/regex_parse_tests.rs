use generator::core::regex::ast::{RegexType, RepeatType};
use generator::core::regex::parser::Parser;

#[test]
fn test_parse_char() {
    let mut p = Parser::new("a");
    let ast = p.parse().unwrap();
    assert!(matches!(ast, RegexType::Char(97)));
}

#[test]
fn test_parse_escape() {
    let mut p = Parser::new(r"\n");
    let ast = p.parse().unwrap();
    assert!(matches!(ast, RegexType::Char(10)));
}

#[test]
fn test_parse_escape_unicode() {
    let mut p = Parser::new(r"\u{1F600}");
    let ast = p.parse().unwrap();
    match ast {
        RegexType::Char(cp) => assert_eq!(cp, 0x1F600),
        _ => panic!("Expected Char"),
    }
}

#[test]
fn test_parse_escape_invalid_unicode() {
    let mut p = Parser::new(r"\u{FFFFFFF}");
    let result = p.parse();
    assert!(result.is_err());
    let err = result.unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("Invalid Unicode code point"));
}

#[test]
fn test_parse_concat() {
    let mut p = Parser::new("ab");
    let ast = p.parse().unwrap();
    match ast {
        RegexType::Concat(left, right) => {
            assert!(matches!(*left, RegexType::Char(97)));
            assert!(matches!(*right, RegexType::Char(98)));
        }
        _ => panic!("Expected Concat"),
    }
}

#[test]
fn test_parse_concat_complex() {
    let mut p = Parser::new("a(b|c)d");
    let ast = p.parse().unwrap();
    match ast {
        RegexType::Concat(left, right) => {
            assert!(matches!(*left, RegexType::Char(97)));
            match *right {
                RegexType::Concat(mid, last) => {
                    assert!(matches!(*last, RegexType::Char(100)));
                    match *mid {
                        RegexType::Alt(l, r) => {
                            assert!(matches!(*l, RegexType::Char(98)));
                            assert!(matches!(*r, RegexType::Char(99)));
                        }
                        _ => panic!("Expected Alt"),
                    }
                }
                _ => panic!("Expected Concat"),
            }
        }
        _ => panic!("Expected Concat"),
    }
}

#[test]
fn test_parse_alt() {
    let mut p = Parser::new("a|b");
    let ast = p.parse().unwrap();
    match ast {
        RegexType::Alt(left, right) => {
            assert!(matches!(*left, RegexType::Char(97)));
            assert!(matches!(*right, RegexType::Char(98)));
        }
        _ => panic!("Expected Alt"),
    }
}

#[test]
fn test_parse_nested_parentheses() {
    let mut p = Parser::new("(a(b)*)c");
    let ast = p.parse().unwrap();
    assert!(matches!(ast, RegexType::Concat(_, _)));
}

#[test]
fn test_parse_star() {
    let mut p = Parser::new("a*");
    let ast = p.parse().unwrap();
    match ast {
        RegexType::Star(inner) => assert!(matches!(*inner, RegexType::Char(97))),
        _ => panic!("Expected Star"),
    }
}

#[test]
fn test_parse_plus() {
    let mut p = Parser::new("a+");
    let ast = p.parse().unwrap();
    match ast {
        RegexType::Plus(inner) => assert!(matches!(*inner, RegexType::Char(97))),
        _ => panic!("Expected Plus"),
    }
}

#[test]
fn test_parse_opt() {
    let mut p = Parser::new("a?");
    let ast = p.parse().unwrap();
    match ast {
        RegexType::Opt(inner) => assert!(matches!(*inner, RegexType::Char(97))),
        _ => panic!("Expected Opt"),
    }
}

#[test]
fn test_parse_quantifier_exact() {
    let mut p = Parser::new("a{3}");
    let ast = p.parse().unwrap();
    match ast {
        RegexType::Repeat(inner, RepeatType::Exactly(3)) => {
            assert!(matches!(*inner, RegexType::Char(97)));
        }
        _ => panic!("Expected Repeat Exactly 3"),
    }
}

#[test]
fn test_parse_quantifier_at_least() {
    let mut p = Parser::new("a{3,}");
    let ast = p.parse().unwrap();
    match ast {
        RegexType::Repeat(inner, RepeatType::AtLeast(3)) => {
            assert!(matches!(*inner, RegexType::Char(97)));
        }
        _ => panic!("Expected AtLeast quantifier"),
    }
}

#[test]
fn test_parse_quantifier_between() {
    let mut p = Parser::new("a{2,5}");
    let ast = p.parse().unwrap();
    match ast {
        RegexType::Repeat(inner, RepeatType::Between(2, 5)) => {
            assert!(matches!(*inner, RegexType::Char(97)));
        }
        _ => panic!("Expected Between quantifier"),
    }
}

#[test]
fn test_parse_quantifier_exactly_zero() {
    let mut p = Parser::new("a{0}");
    let ast = p.parse().unwrap();
    match ast {
        RegexType::Repeat(inner, RepeatType::Exactly(0)) => {
            assert!(matches!(*inner, RegexType::Char(97)));
        }
        _ => panic!("Expected Exactly 0"),
    }
}

#[test]
fn test_parse_quantifier_bound_reversed() {
    let mut p = Parser::new("a{5,2}");
    let result = p.parse();
    assert!(result.is_err());
    let err = result.unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("lower bound > upper bound"));
}

#[test]
fn test_parse_empty_quantifier() {
    let mut p = Parser::new("a{}");
    let result = p.parse();
    assert!(result.is_err());
}

#[test]
fn test_parse_char_class_simple() {
    let mut p = Parser::new("[abc]");
    let ast = p.parse().unwrap();
    match ast {
        RegexType::CharClass(cls) => {
            assert!(!cls.negated);
            assert_eq!(cls.items.len(), 1);
            assert_eq!(cls.items[0].start, 97);
            assert_eq!(cls.items[0].end, 99);
        }
        _ => panic!("Expected CharClass"),
    }
}

#[test]
fn test_parse_char_class_range() {
    let mut p = Parser::new("[a-z]");
    let ast = p.parse().unwrap();
    match ast {
        RegexType::CharClass(cls) => {
            assert!(!cls.negated);
            assert_eq!(cls.items.len(), 1);
            assert_eq!(cls.items[0].start, 97);
            assert_eq!(cls.items[0].end, 122);
        }
        _ => panic!("Expected CharClass"),
    }
}

#[test]
fn test_parse_char_class_with_range_and_single() {
    let mut p = Parser::new("[a-cx-z]");
    let ast = p.parse().unwrap();
    match ast {
        RegexType::CharClass(cls) => {
            assert!(!cls.negated);
            assert_eq!(cls.items.len(), 2);
            assert_eq!(cls.items[0].start, 97);
            assert_eq!(cls.items[0].end, 99);
            assert_eq!(cls.items[1].start, 120);
            assert_eq!(cls.items[1].end, 122);
        }
        _ => panic!("Expected CharClass"),
    }
}

#[test]
fn test_parse_negated_char_class() {
    let mut p = Parser::new("[^0-9]");
    let ast = p.parse().unwrap();
    match ast {
        RegexType::CharClass(cls) => {
            assert!(cls.negated);
            assert_eq!(cls.items.len(), 1);
            assert_eq!(cls.items[0].start, 48);
            assert_eq!(cls.items[0].end, 57);
        }
        _ => panic!("Expected CharClass"),
    }
}

#[test]
fn test_parse_char_class_hyphen_at_start() {
    let mut p = Parser::new("[-a]");
    let ast = p.parse().unwrap();
    match ast {
        RegexType::CharClass(cls) => {
            assert_eq!(cls.items.len(), 2);
            assert_eq!(cls.items[0].start, 45);
            assert_eq!(cls.items[0].end, 45);
            assert_eq!(cls.items[1].start, 97);
            assert_eq!(cls.items[1].end, 97);
        }
        _ => panic!("Expected CharClass"),
    }
}

#[test]
fn test_parse_char_class_hyphen_at_end() {
    let mut p = Parser::new("[a-]");
    let ast = p.parse().unwrap();
    match ast {
        RegexType::CharClass(cls) => {
            assert_eq!(cls.items.len(), 2);
            assert_eq!(cls.items[0].start, 45);
            assert_eq!(cls.items[0].end, 45);
            assert_eq!(cls.items[1].start, 97);
            assert_eq!(cls.items[1].end, 97);
        }
        _ => panic!("Expected CharClass"),
    }
}

#[test]
fn test_parse_char_class_escaped_bracket() {
    let mut p = Parser::new(r"\[\]");
    let ast = p.parse().unwrap();
    match ast {
        RegexType::Concat(left, right) => {
            assert!(matches!(*left, RegexType::Char(91)));
            assert!(matches!(*right, RegexType::Char(93)));
        }
        _ => panic!("Expected Concat of [ and ]"),
    }
}

#[test]
fn test_escape_d() {
    let mut p = Parser::new("\\d+");
    let ast = p.parse().unwrap();
    match ast {
        RegexType::Plus(boxed) => match *boxed {
            RegexType::CharClass(cls) => {
                assert!(!cls.negated);
                assert_eq!(cls.items[0].start, '0' as u32);
                assert_eq!(cls.items[0].end, '9' as u32);
            }
            _ => panic!("Expected CharClass inside Plus"),
        },
        _ => panic!("Expected Plus"),
    }
}

#[test]
fn test_escape_w() {
    let mut p = Parser::new("\\w");
    let ast = p.parse().unwrap();
    match ast {
        RegexType::CharClass(cls) => {
            assert!(!cls.negated);
            assert!(cls.items.len() >= 3);
        }
        _ => panic!("Expected CharClass"),
    }
}

#[test]
fn test_escape_s() {
    let mut p = Parser::new("\\s");
    let ast = p.parse().unwrap();
    match ast {
        RegexType::CharClass(cls) => {
            assert!(!cls.negated);
            assert!(cls.items.len() >= 1);
        }
        _ => panic!("Expected CharClass"),
    }
}

#[test]
#[allow(non_snake_case)]
fn test_negated_escape_D() {
    let mut p = Parser::new("\\D");
    let ast = p.parse().unwrap();
    match ast {
        RegexType::CharClass(cls) => {
            assert!(cls.negated);
        }
        _ => panic!("Expected CharClass"),
    }
}

#[test]
#[allow(non_snake_case)]
fn test_negated_escape_W() {
    let mut p = Parser::new("\\W");
    let ast = p.parse().unwrap();
    match ast {
        RegexType::CharClass(cls) => {
            assert!(cls.negated);
        }
        _ => panic!("Expected CharClass"),
    }
}

#[test]
fn test_parse_dot() {
    let mut p = Parser::new(".");
    let ast = p.parse().unwrap();
    match ast {
        RegexType::CharClass(cls) => {
            assert!(!cls.negated);
            assert_eq!(cls.items.len(), 2);
            assert_eq!(cls.items[0].start, 0);
            assert_eq!(cls.items[0].end, 9);
            assert_eq!(cls.items[1].start, 11);
            assert_eq!(cls.items[1].end, 0x10FFFF);
        }
        _ => panic!("Expected CharClass for ."),
    }
}

#[test]
fn test_unmatched_paren() {
    let mut p = Parser::new("(a(b");
    let result = p.parse();
    assert!(result.is_err());
    let err = result.unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("Expected ')'"));
}

#[test]
fn test_unmatched_bracket() {
    let mut p = Parser::new("[a-z");
    let result = p.parse();
    assert!(result.is_err());
    let err = result.unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("Expected ']'"));
}

#[test]
fn test_invalid_escape() {
    let mut p = Parser::new(r"\x");
    let result = p.parse();
    assert!(result.is_err());
}

#[test]
fn test_incomplete_escape_at_end() {
    let mut p = Parser::new("\\");
    let result = p.parse();
    assert!(result.is_err());
}

#[test]
fn test_unexpected_character() {
    let mut p = Parser::new("a$b");
    let result = p.parse();
    assert!(result.is_err());
}

#[test]
fn test_complex_regex() {
    let mut p = Parser::new(r"^[A-Za-z_][A-Za-z0-9_]*$");
    let ast = p.parse().unwrap();
    assert!(matches!(ast, RegexType::Concat(_, _)));
}

#[test]
fn test_whitespace_in_regex() {
    let mut p = Parser::new(r"\s+");
    let ast = p.parse().unwrap();
    assert!(matches!(ast, RegexType::Plus(_)));
}
