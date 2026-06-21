use generator::core::regex::parser::Parser;

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
fn test_empty_quantifier() {
    let mut p = Parser::new("a{}");
    let result = p.parse();
    assert!(result.is_err());
}

#[test]
fn test_incomplete_escape_at_end() {
    let mut p = Parser::new("\\");
    let result = p.parse();
    assert!(result.is_err());
}
