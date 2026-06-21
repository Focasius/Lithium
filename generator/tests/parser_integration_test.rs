use generator::parser::generator::generate_parser;
use generator::parser::grammar::{Expr, Grammar, Rule};
use std::fs;
use std::process::Command;
use tempfile::tempdir;

#[test]
fn test_arithmetic_parser() {
    let grammar = Grammar {
        start: "Add".to_string(),
        rules: vec![
            Rule {
                name: "Add".to_string(),
                expr: Expr::seq(vec![
                    Expr::RuleRef("Mul".to_string()),
                    Expr::Repeat(Box::new(Expr::seq(vec![
                        Expr::Char('+' as u32),
                        Expr::RuleRef("Mul".to_string()),
                    ]))),
                ]),
                ast: Some("AST::Value(format!(\"Add({:?})\", $_r0))".to_string()),
            },
            Rule {
                name: "Mul".to_string(),
                expr: Expr::seq(vec![
                    Expr::RuleRef("Primary".to_string()),
                    Expr::Repeat(Box::new(Expr::seq(vec![
                        Expr::Char('*' as u32),
                        Expr::RuleRef("Primary".to_string()),
                    ]))),
                ]),
                ast: Some("AST::Value(format!(\"Mul({:?})\", $_r0))".to_string()),
            },
            Rule {
                name: "Primary".to_string(),
                expr: Expr::choice(vec![
                    Expr::seq(vec![
                        Expr::Char('(' as u32),
                        Expr::RuleRef("Add".to_string()),
                        Expr::Char(')' as u32),
                    ]),
                    Expr::Repeat(Box::new(Expr::Char('0' as u32))),
                ]),
                ast: Some("AST::Value(format!(\"Primary({:?})\", $_r0))".to_string()),
            },
        ],
    };

    let parser_name = "ArithmeticParser";
    let code = generate_parser(&grammar, parser_name, None).unwrap();

    let dir = tempdir().unwrap();
    let parser_path = dir.path().join("parser.rs");
    fs::write(&parser_path, &code).unwrap();

    let status = Command::new("rustc")
        .arg("--crate-type=lib")
        .arg(&parser_path)
        .arg("-o")
        .arg(dir.path().join("libparser.rlib"))
        .current_dir(dir.path())
        .status()
        .unwrap();
    assert!(status.success());

    let driver = r#"
extern crate parser;
use parser::{ArithmeticParser, AST};

fn main() {
    let mut parser = ArithmeticParser::new("2+3*4");
    let result = parser.parse();
    match result {
        Ok(ast) => println!("OK: {:?}", ast),
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}
"#;
    let driver_path = dir.path().join("driver.rs");
    fs::write(&driver_path, driver).unwrap();

    let status = Command::new("rustc")
        .arg("--extern")
        .arg(format!(
            "parser={}",
            dir.path().join("libparser.rlib").display()
        ))
        .arg(&driver_path)
        .arg("-o")
        .arg(dir.path().join("driver"))
        .current_dir(dir.path())
        .status()
        .unwrap();
    assert!(status.success());

    let output = Command::new(dir.path().join("driver"))
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("OK"));
}
