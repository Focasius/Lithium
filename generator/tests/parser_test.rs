use generator::parser::generator::generate_parser;
use generator::parser::grammar::{Expr, Grammar, Rule};
use generator::parser::peg_parser::PegParser;
use std::fs;
use std::process::Command;
use tempfile::tempdir;

#[test]
fn test_grammar_serialization() {
    let grammar = Grammar {
        start: "S".to_string(),
        rules: vec![
            Rule {
                name: "S".to_string(),
                expr: Expr::seq(vec![Expr::Char('a' as u32), Expr::RuleRef("A".to_string())]),
                ast: Some("S".to_string()),
            },
            Rule {
                name: "A".to_string(),
                expr: Expr::choice(vec![Expr::Char('b' as u32), Expr::Char('c' as u32)]),
                ast: Some("A".to_string()),
            },
        ],
    };

    let json = serde_json::to_string(&grammar).unwrap();
    let parsed: Grammar = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.start, "S");
    assert_eq!(parsed.rules.len(), 2);
}

#[test]
fn test_peg_parser_basic() {
    let input = r#"
        S <- 'a' A
        A <- 'b' / 'c'
    "#;
    let grammar = PegParser::new(input).parse().unwrap();
    assert_eq!(grammar.start, "S");
    assert_eq!(grammar.rules.len(), 2);
    assert_eq!(grammar.rules[0].name, "S");
    assert_eq!(grammar.rules[1].name, "A");
}

#[test]
fn test_peg_parser_quantifiers() {
    let input = r#"
        S <- 'a'* 'b'+
        T <- 'c'?
    "#;
    let grammar = PegParser::new(input).parse().unwrap();
    assert_eq!(grammar.rules.len(), 2);
}

#[test]
fn test_peg_parser_predicates() {
    let input = r#"
        S <- &'a' 'b'
        T <- !'c'
    "#;
    let grammar = PegParser::new(input).parse().unwrap();
    assert_eq!(grammar.rules.len(), 2);
}

#[test]
fn test_peg_parser_error() {
    let input = r#"
        S <- 'a'
        S <- 'b' // 重复规则
    "#;
    let result = PegParser::new(input).parse();
    assert!(result.is_err());
}

#[test]
fn test_direct_left_recursion_detection() {
    let grammar = Grammar {
        start: "S".to_string(),
        rules: vec![Rule {
            name: "S".to_string(),
            expr: Expr::seq(vec![Expr::RuleRef("S".to_string()), Expr::Char('a' as u32)]),
            ast: None,
        }],
    };
    let result = generate_parser(&grammar, "TestParser", None);
    assert!(result.is_err());
    let err = result.unwrap_err();

    let err_msg = err.to_string();
    assert!(err_msg.contains("Left recursion"));
}

#[test]
fn test_indirect_left_recursion_detection() {
    let grammar = Grammar {
        start: "S".to_string(),
        rules: vec![
            Rule {
                name: "S".to_string(),
                expr: Expr::RuleRef("A".to_string()),
                ast: None,
            },
            Rule {
                name: "A".to_string(),
                expr: Expr::seq(vec![Expr::RuleRef("S".to_string()), Expr::Char('b' as u32)]),
                ast: None,
            },
        ],
    };
    let result = generate_parser(&grammar, "TestParser", None);
    assert!(result.is_err());
    let err = result.unwrap_err();
    let err_msg = err.to_string();
    assert!(err_msg.contains("Left recursion"));
}

#[test]
fn test_generate_parser_code() {
    let grammar = Grammar {
        start: "Add".to_string(),
        rules: vec![
            Rule {
                name: "Add".to_string(),
                expr: Expr::seq(vec![
                    Expr::RuleRef("Term".to_string()),
                    Expr::Repeat(Box::new(Expr::seq(vec![
                        Expr::Char('+' as u32),
                        Expr::RuleRef("Term".to_string()),
                    ]))),
                ]),
                ast: Some("Add".to_string()),
            },
            Rule {
                name: "Term".to_string(),
                expr: Expr::seq(vec![
                    Expr::RuleRef("Factor".to_string()),
                    Expr::Repeat(Box::new(Expr::seq(vec![
                        Expr::Char('*' as u32),
                        Expr::RuleRef("Factor".to_string()),
                    ]))),
                ]),
                ast: Some("Mul".to_string()),
            },
            Rule {
                name: "Factor".to_string(),
                expr: Expr::choice(vec![
                    Expr::seq(vec![
                        Expr::Char('(' as u32),
                        Expr::RuleRef("Add".to_string()),
                        Expr::Char(')' as u32),
                    ]),
                    Expr::Repeat(Box::new(Expr::Char('0' as u32))),
                ]),
                ast: Some("Number".to_string()),
            },
        ],
    };

    let code = generate_parser(&grammar, "ArithmeticParser", None).unwrap();
    assert!(code.contains("struct ArithmeticParser"));
    assert!(code.contains("parse_Add"));
    assert!(code.contains("parse_Term"));
    assert!(code.contains("parse_Factor"));
    assert!(code.contains("AST"));
}

#[test]
fn test_compile_generated_parser() {
    let grammar = Grammar {
        start: "S".to_string(),
        rules: vec![
            Rule {
                name: "S".to_string(),
                expr: Expr::seq(vec![Expr::Char('a' as u32), Expr::RuleRef("A".to_string())]),
                ast: None,
            },
            Rule {
                name: "A".to_string(),
                expr: Expr::choice(vec![Expr::Char('b' as u32), Expr::Char('c' as u32)]),
                ast: None,
            },
        ],
    };

    let code = generate_parser(&grammar, "TestScanner", None).unwrap();

    let dir = tempdir().unwrap();
    let parser_path = dir.path().join("parser.rs");
    fs::write(&parser_path, code).unwrap();

    let status = Command::new("rustc")
        .arg("--crate-type=lib")
        .arg(&parser_path)
        .arg("-o")
        .arg(dir.path().join("libparser.rlib"))
        .current_dir(dir.path())
        .status()
        .unwrap();
    assert!(status.success());
}

#[test]
fn test_generated_parser_works() {
    let grammar = Grammar {
        start: "Expr".to_string(),
        rules: vec![
            Rule {
                name: "Expr".to_string(),
                expr: Expr::seq(vec![
                    Expr::RuleRef("Term".to_string()),
                    Expr::Repeat(Box::new(Expr::seq(vec![
                        Expr::Char('+' as u32),
                        Expr::RuleRef("Term".to_string()),
                    ]))),
                ]),
                ast: Some("Add".to_string()),
            },
            Rule {
                name: "Term".to_string(),
                expr: Expr::Repeat(Box::new(Expr::Char('0' as u32))),
                ast: Some("Number".to_string()),
            },
        ],
    };

    let code = generate_parser(&grammar, "ExprParser", None).unwrap();
    let dir = tempdir().unwrap();
    let parser_path = dir.path().join("parser.rs");
    fs::write(&parser_path, code).unwrap();

    let driver = r#"
extern crate parser;
use parser::{ExprParser, AST};

fn main() {
    let mut parser = ExprParser::new("2+3");
    let ast = parser.parse().unwrap();
    match ast {
        AST::Add(_) => println!("OK"),
        _ => panic!("Expected Add"),
    }
}
"#;
    let driver_path = dir.path().join("driver.rs");
    fs::write(&driver_path, driver).unwrap();

    let status = Command::new("rustc")
        .arg("--crate-type=lib")
        .arg(&parser_path)
        .arg("-o")
        .arg(dir.path().join("libparser.rlib"))
        .current_dir(dir.path())
        .status()
        .unwrap();
    assert!(status.success());

    let status = Command::new("rustc")
        .arg("--extern")
        .arg(format!(
            "parser={}",
            dir.path().join("libparser.rlib").display()
        ))
        .arg(&driver_path)
        .arg("-o")
        .arg(dir.path().join("driver.exe"))
        .current_dir(dir.path())
        .status()
        .unwrap();
    assert!(status.success());

    let output = Command::new(dir.path().join("driver.exe"))
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("OK"));
}
