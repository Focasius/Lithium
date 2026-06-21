use crate::core::error::{Error, Result};
use std::collections::HashMap;

pub fn expand_templates(
    input: &str,
    templates: &HashMap<String, String>,
    seen: &mut Vec<String>,
) -> Result<String> {
    let mut result = String::new();
    let chars: Vec<char> = input.chars().collect();
    let mut i = 0;
    let len = chars.len();

    while i < len {
        match chars[i] {
            '\\' => {
                if i + 1 < len {
                    result.push('\\');
                    result.push(chars[i + 1]);
                    i += 2;
                } else {
                    result.push('\\');
                    i += 1;
                }
            }
            '{' => {
                let start = i + 1;
                let mut end = start;
                while end < len && (chars[end].is_ascii_alphanumeric() || chars[end] == '_') {
                    end += 1;
                }
                let is_valid_template =
                    end > start && (chars[start].is_ascii_alphabetic() || chars[start] == '_');

                if end < len && chars[end] == '}' && is_valid_template {
                    let name: String = chars[start..end].iter().collect();
                    i = end + 1;
                    let template_body = templates
                        .get(&name)
                        .ok_or_else(|| Error::config(format!("Undefined template: '{}'", name)))?;
                    if seen.contains(&name) {
                        return Err(Error::config(format!(
                            "Cyclic template reference: {} -> {}",
                            seen.join(" -> "),
                            name
                        )));
                    }
                    seen.push(name.clone());
                    let expanded = expand_templates(template_body, templates, seen)?;
                    seen.pop();
                    result.push_str(&expanded);
                } else {
                    result.push('{');
                    i += 1;
                }
            }
            _ => {
                result.push(chars[i]);
                i += 1;
            }
        }
    }
    Ok(result)
}

pub fn expand_patterns<F>(
    patterns: Vec<F>,
    templates: &HashMap<String, String>,
    mut get_regex: impl FnMut(&mut F) -> &mut String,
) -> Result<Vec<F>> {
    let mut expanded = Vec::with_capacity(patterns.len());
    for mut pat in patterns {
        let mut seen = Vec::new();
        let new_regex = expand_templates(get_regex(&mut pat), templates, &mut seen)?;
        *get_regex(&mut pat) = new_regex;
        expanded.push(pat);
    }
    Ok(expanded)
}
