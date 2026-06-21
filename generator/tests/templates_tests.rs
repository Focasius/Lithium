use generator::config::PatternDef;
use generator::core::template::expander::{expand_patterns, expand_templates};
use std::collections::HashMap;

#[test]
fn test_nested_templates() {
    let mut templates = HashMap::new();
    templates.insert("A".to_string(), r"{B}".to_string());
    templates.insert("B".to_string(), r"abc".to_string());
    let input = r"{A}";
    let mut seen = Vec::new();
    let result = expand_templates(input, &templates, &mut seen).unwrap();
    assert_eq!(result, "abc");
}

#[test]
fn test_template_with_braces_not_template() {
    let templates = HashMap::new();
    let input = r"\{not a template\}";
    let mut seen = Vec::new();
    let result = expand_templates(input, &templates, &mut seen).unwrap();
    assert_eq!(result, r"\{not a template\}");
}

#[test]
fn test_template_undefined() {
    let templates = HashMap::new();
    let input = r"{UNDEF}";
    let mut seen = Vec::new();
    let result = expand_templates(input, &templates, &mut seen);
    assert!(result.is_err());
    let err = result.unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("Undefined template"));
}

#[test]
fn test_template_with_escape() {
    let mut templates = HashMap::new();
    templates.insert("ESC".to_string(), r"\n".to_string());
    let input = r"{ESC}";
    let mut seen = Vec::new();
    let result = expand_templates(input, &templates, &mut seen).unwrap();
    assert_eq!(result, r"\n");
}

#[test]
fn test_expand_patterns_with_templates() {
    let mut templates = HashMap::new();
    templates.insert("DIGIT".to_string(), r"[0-9]".to_string());
    let patterns = vec![PatternDef {
        token: "NUM".to_string(),
        regex: "{DIGIT}+".to_string(),
    }];
    let expanded = expand_patterns(patterns, &templates, |pat| &mut pat.regex).unwrap();
    assert_eq!(expanded[0].regex, r"[0-9]+");
}
