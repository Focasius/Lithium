

use crate::core::error::{Error, Result};
use crate::core::template::engine::CoreTemplateEngine;
use crate::scanner::dfa::DFA;
use serde_json::{Value, json};

pub fn generate_code(
    dfa: &DFA,
    token_names: Vec<String>,
    scanner_name: &str,
    template_content: Option<&str>,
    compress: bool,
    skip_token_ids: &[usize],
) -> Result<String> {
    if compress {
        generate_code_compressed(
            dfa,
            token_names,
            scanner_name,
            template_content,
            skip_token_ids,
        )
    } else {
        generate_code_dense(
            dfa,
            token_names,
            scanner_name,
            template_content,
            skip_token_ids,
        )
    }
}

fn generate_code_dense(
    dfa: &DFA,
    token_names: Vec<String>,
    scanner_name: &str,
    template_content: Option<&str>,
    skip_token_ids: &[usize],
) -> Result<String> {
    let original_states = dfa.transitions.len();
    let num_classes = dfa.intervals.len();
    let state_offset = 1;
    let start_state = dfa.start_state + state_offset;

    let mut trans_table = vec![vec![0; num_classes]; original_states + 1];
    for (i, row) in dfa.transitions.iter().enumerate() {
        for (j, &next) in row.iter().enumerate() {
            trans_table[i + state_offset][j] = next.map(|s| s + state_offset).unwrap_or(0);
        }
    }

    let mut accept_strs = vec!["None".to_string(); original_states + 1];
    for (i, &opt_token) in dfa.accept.iter().enumerate() {
        let idx = i + state_offset;
        accept_strs[idx] = match opt_token {
            Some(tid) => format!("Some({})", tid),
            None => "None".to_string(),
        };
    }

    let total_tokens = token_names.len() + skip_token_ids.len();
    let mut skip_flags = vec![false; total_tokens];
    for &id in skip_token_ids {
        if id < total_tokens {
            skip_flags[id] = true;
        }
    }

    let intervals: Vec<Value> = dfa
        .intervals
        .iter()
        .map(|&(s, e)| json!({"start": s, "end": e}))
        .collect();

    let data = json!({
        "scanner_name": scanner_name,
        "token_names": token_names,
        "start_state": start_state,
        "num_classes": num_classes,
        "transitions": trans_table,
        "accept": accept_strs,
        "skip_flags": skip_flags,
        "skip_flags_len": skip_flags.len(),  
        "intervals": intervals,
        "transitions_len": trans_table.len(),
        "accept_len": accept_strs.len(),
    });

    let mut engine = CoreTemplateEngine::new();
    let template_str = template_content.unwrap_or(include_str!("templates/rust_template.hbs"));
    engine
        .render(template_str, &data)
        .map_err(|e| Error::other(e.to_string()))
}

fn generate_code_compressed(
    dfa: &DFA,
    token_names: Vec<String>,
    scanner_name: &str,
    template_content: Option<&str>,
    skip_token_ids: &[usize],
) -> Result<String> {
    let compressed = dfa.compress();

    let intervals: Vec<Value> = compressed
        .intervals
        .iter()
        .map(|&(s, e)| json!({"start": s, "end": e}))
        .collect();

    let mut flat_special = Vec::new();
    for (state, vec) in compressed.special_trans.iter().enumerate() {
        for &(class, next) in vec {
            flat_special.push(json!({"state": state, "class": class, "next": next}));
        }
    }
    flat_special.sort_by_key(|v| (v["state"].as_u64().unwrap(), v["class"].as_u64().unwrap()));

    let accept_strs: Vec<String> = compressed
        .accept
        .iter()
        .map(|opt| match opt {
            Some(tid) => format!("Some({})", tid),
            None => "None".to_string(),
        })
        .collect();

    let total_tokens = token_names.len() + skip_token_ids.len();
    let mut skip_flags = vec![false; total_tokens];
    for &id in skip_token_ids {
        if id < total_tokens {
            skip_flags[id] = true;
        }
    }

    let data = json!({
        "scanner_name": scanner_name,
        "token_names": token_names,
        "start_state": compressed.start_state,
        "num_classes": compressed.intervals.len(),
        "states_len": compressed.default_trans.len(),
        "accept_len": compressed.accept.len(),
        "default_trans": compressed.default_trans,
        "special_trans": flat_special,
        "accept": accept_strs,
        "skip_flags": skip_flags,
        "skip_flags_len": skip_flags.len(),  
        "intervals": intervals,
    });

    let mut engine = CoreTemplateEngine::new();
    let template_str = template_content.unwrap_or(include_str!("templates/rust_compressed.hbs"));
    engine
        .render(template_str, &data)
        .map_err(|e| Error::other(e.to_string()))
}
