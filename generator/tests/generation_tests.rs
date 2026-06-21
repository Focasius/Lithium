use generator::core::regex::parser::Parser;
use generator::scanner::dfa::build_dfa;
use generator::scanner::generator::generate_code;
use generator::scanner::nfa::build_nfa;
use std::process::Command;
use tempfile::tempdir;

#[test]
fn test_generated_scanner_accepts_valid_input() {
    let patterns = vec![
        ("NUM".to_string(), r"[0-9]+".to_string()),
        ("ID".to_string(), r"[a-zA-Z_]+".to_string()),
        ("WHITESPACE".to_string(), r"\s+".to_string()),
    ];

    let mut regex_asts = Vec::new();
    for (name, regex) in &patterns {
        let mut parser = Parser::new(regex);
        let ast = parser.parse().unwrap();
        regex_asts.push((name.clone(), ast));
    }

    let mut token_names = Vec::new();
    let nfa_input: Vec<(usize, _)> = regex_asts
        .into_iter()
        .enumerate()
        .map(|(id, (name, ast))| {
            token_names.push(name);
            (id, ast)
        })
        .collect();

    let nfa = build_nfa(&nfa_input);
    eprintln!("=== NFA accepts: {:?}", nfa.accepts);

    let mut dfa = build_dfa(&nfa);

    eprintln!("=== DFA before minimization ===");
    eprintln!("{}", dfa);
    eprintln!("Accept array: {:?}", dfa.accept);
    eprintln!("Intervals: {:?}", dfa.intervals);
    eprintln!("Start state: {}", dfa.start_state);

    dfa.minimize();

    eprintln!("=== DFA after minimization ===");
    eprintln!("{}", dfa);
    eprintln!("Accept array: {:?}", dfa.accept);

    let code = generate_code(&dfa, token_names, "TestScanner", None, false, &[]).unwrap();
    println!("=== Generated scanner code ===");
    println!("{}", code);
    let dir = tempdir().unwrap();
    let scanner_path = dir.path().join("scanner.rs");
    std::fs::write(&scanner_path, code).unwrap();

    let test_driver = r#"
extern crate test_scanner;
use test_scanner::{TestScanner, Token};

fn main() {
    let mut scanner = TestScanner::new("123 abc");
    let mut tokens = Vec::new();
    loop {
        let (tok, start, end) = scanner.next_token();
        match tok {
            Token::Eof => break,
            Token::WHITESPACE => continue,
            _ => {
                let lexeme = scanner.lexeme(start, end);
                tokens.push((tok, lexeme));
            }
        }
    }
    assert_eq!(tokens, vec![
        (Token::NUM, "123".to_string()),
        (Token::ID, "abc".to_string()),
    ]);
}
"#;

    let driver_path = dir.path().join("driver.rs");
    std::fs::write(&driver_path, test_driver).unwrap();

    let status = Command::new("rustc")
        .arg("--crate-type=lib")
        .arg(&scanner_path)
        .arg("-o")
        .arg(dir.path().join("libtest_scanner.rlib"))
        .current_dir(dir.path())
        .status()
        .unwrap();
    assert!(status.success());

    let status = Command::new("rustc")
        .arg("--extern")
        .arg(format!(
            "test_scanner={}",
            dir.path().join("libtest_scanner.rlib").display()
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
    if !output.status.success() {
        eprintln!("stderr: {}", String::from_utf8_lossy(&output.stderr));
        eprintln!("stdout: {}", String::from_utf8_lossy(&output.stdout));
    }
    assert!(output.status.success());
}
