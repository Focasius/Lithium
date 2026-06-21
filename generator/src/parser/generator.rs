use crate::core::error::{Error, Result};
use crate::core::template::engine::CoreTemplateEngine;
use crate::parser::grammar::{Expr, Grammar, Rule};
use std::collections::{HashMap, HashSet};

pub fn generate_parser(
    grammar: &Grammar,
    parser_name: &str,
    template_content: Option<&str>,
) -> Result<String> {
    let grammar = eliminate_left_recursion(grammar)?;

    use rayon::prelude::*;
    let rule_bodies: Vec<serde_json::Value> = grammar
        .rules
        .par_iter()
        .map(|rule| {
            let body = generate_expr_code(&rule.expr, &rule.ast);
            serde_json::json!({
                "name": rule.name,
                "body": body,
            })
        })
        .collect();

    let data = serde_json::json!({
        "parser_name": parser_name,
        "start": grammar.start,
        "rules": rule_bodies,
    });

    let mut engine = CoreTemplateEngine::new();
    let template_str = template_content.unwrap_or(include_str!("templates/parser_template.hbs"));
    engine
        .render(template_str, &data)
        .map_err(|e| Error::other(e.to_string()))
}

fn eliminate_left_recursion(grammar: &Grammar) -> Result<Grammar> {
    let mut dependencies = HashMap::new();
    for rule in &grammar.rules {
        let deps = collect_nonterminal_deps(&rule.expr);
        dependencies.insert(rule.name.clone(), deps);
    }

    let mut rules_map: HashMap<String, Rule> = grammar
        .rules
        .iter()
        .map(|r| (r.name.clone(), r.clone()))
        .collect();

    let rule_names: Vec<String> = grammar.rules.iter().map(|r| r.name.clone()).collect();

    for i in 0..rule_names.len() {
        let ai = &rule_names[i];
        for j in rule_names.iter().take(i) {
            let aj = j;

            if let Some(rule_ai) = rules_map.get(ai)
                && contains_rule_ref(&rule_ai.expr, aj)
            {
                let rule_aj = rules_map.get(aj).unwrap();
                let new_expr = replace_rule_ref(&rule_ai.expr, aj, &rule_aj.expr);
                let mut new_rule = rule_ai.clone();
                new_rule.expr = new_expr;
                rules_map.insert(ai.clone(), new_rule);
            }
        }

        if let Some(rule) = rules_map.get(ai)
            && let Some((new_rule, new_tail)) = eliminate_direct_left_recursion(rule)
        {
            rules_map.insert(ai.clone(), new_rule);
            rules_map.insert(new_tail.name.clone(), new_tail);
        }
    }

    let mut new_rules: Vec<Rule> = rules_map.into_values().collect();

    new_rules.sort_by_key(|r| r.name != grammar.start);
    Ok(Grammar {
        start: grammar.start.clone(),
        rules: new_rules,
    })
}

fn collect_nonterminal_deps(expr: &Expr) -> HashSet<String> {
    let mut deps = HashSet::new();
    match expr {
        Expr::RuleRef(name) => {
            deps.insert(name.clone());
        }
        Expr::Sequence(seq) | Expr::Choice(seq) => {
            for e in seq {
                deps.extend(collect_nonterminal_deps(e));
            }
        }
        Expr::Repeat(e)
        | Expr::Plus(e)
        | Expr::Optional(e)
        | Expr::Group(e)
        | Expr::AndPredicate(e)
        | Expr::NotPredicate(e) => {
            deps.extend(collect_nonterminal_deps(e));
        }
        _ => {}
    }
    deps
}

fn contains_rule_ref(expr: &Expr, name: &str) -> bool {
    match expr {
        Expr::RuleRef(n) => n == name,
        Expr::Sequence(seq) | Expr::Choice(seq) => seq.iter().any(|e| contains_rule_ref(e, name)),
        Expr::Repeat(e)
        | Expr::Plus(e)
        | Expr::Optional(e)
        | Expr::Group(e)
        | Expr::AndPredicate(e)
        | Expr::NotPredicate(e) => contains_rule_ref(e, name),
        _ => false,
    }
}

fn replace_rule_ref(expr: &Expr, target: &str, replacement: &Expr) -> Expr {
    match expr {
        Expr::RuleRef(name) if name == target => replacement.clone(),
        Expr::Sequence(seq) => Expr::Sequence(
            seq.iter()
                .map(|e| replace_rule_ref(e, target, replacement))
                .collect(),
        ),
        Expr::Choice(seq) => Expr::Choice(
            seq.iter()
                .map(|e| replace_rule_ref(e, target, replacement))
                .collect(),
        ),
        Expr::Repeat(e) => Expr::Repeat(Box::new(replace_rule_ref(e, target, replacement))),
        Expr::Plus(e) => Expr::Plus(Box::new(replace_rule_ref(e, target, replacement))),
        Expr::Optional(e) => Expr::Optional(Box::new(replace_rule_ref(e, target, replacement))),
        Expr::Group(e) => Expr::Group(Box::new(replace_rule_ref(e, target, replacement))),
        Expr::AndPredicate(e) => {
            Expr::AndPredicate(Box::new(replace_rule_ref(e, target, replacement)))
        }
        Expr::NotPredicate(e) => {
            Expr::NotPredicate(Box::new(replace_rule_ref(e, target, replacement)))
        }
        _ => expr.clone(),
    }
}

fn eliminate_direct_left_recursion(rule: &Rule) -> Option<(Rule, Rule)> {
    let mut beta_alts = Vec::new();
    let mut alpha_alts = Vec::new();

    match &rule.expr {
        Expr::Choice(alts) => {
            for alt in alts {
                if let Expr::Sequence(seq) = alt
                    && let Some(first) = seq.first()
                    && let Expr::RuleRef(name) = first
                    && name == &rule.name
                {
                    let alpha = if seq.len() > 1 {
                        Expr::seq(seq[1..].to_vec())
                    } else {
                        continue;
                    };
                    alpha_alts.push(alpha);
                    continue;
                }
                beta_alts.push(alt.clone());
            }
        }
        _ => return None,
    }

    if alpha_alts.is_empty() {
        return None;
    }

    if beta_alts.is_empty() {
        panic!("Rule {} is pure left-recursive (A -> A α)", rule.name);
    }

    let tail_name = format!("{}__tail", rule.name);
    let tail_expr = Expr::choice(vec![
        Expr::seq(vec![
            Expr::choice(alpha_alts),
            Expr::RuleRef(tail_name.clone()),
        ]),
        Expr::seq(vec![]),
    ]);
    let tail_rule = Rule {
        name: tail_name.clone(),
        expr: tail_expr,
        ast: None,
    };

    let new_expr = Expr::seq(vec![Expr::choice(beta_alts), Expr::RuleRef(tail_name)]);
    let new_rule = Rule {
        name: rule.name.clone(),
        expr: new_expr,
        ast: rule.ast.clone(),
    };

    Some((new_rule, tail_rule))
}

fn generate_expr_code(expr: &Expr, ast_action: &Option<String>) -> String {
    match expr {
        Expr::Sequence(seq) => {
            let mut code = String::new();
            let mut results = Vec::new();
            for (i, e) in seq.iter().enumerate() {
                let var = format!("_r{}", i);
                let sub = generate_expr_code(e, &None);
                code.push_str(&format!("        let {} = {};\n", var, sub));
                results.push(var);
            }
            if let Some(action) = ast_action {
                let args = results.join(", ");
                code.push_str(&format!("        Ok({})\n", action.replace("$", &args)));
            } else if let Some(last) = results.last() {
                code.push_str(&format!("        Ok({})\n", last));
            } else {
                code.push_str("        Ok(AST::Value(\"\".to_string()))\n");
            }
            code
        }
        Expr::Choice(alts) => {
            let mut code = String::new();
            code.push_str("        let saved_pos = self.pos;\n");
            code.push_str("        let saved_line = self.line;\n");
            code.push_str("        let saved_col = self.col;\n");
            for (i, alt) in alts.iter().enumerate() {
                let sub = generate_expr_code(alt, &None);
                if i > 0 {
                    code.push_str("        self.pos = saved_pos;\n");
                    code.push_str("        self.line = saved_line;\n");
                    code.push_str("        self.col = saved_col;\n");
                }
                code.push_str(&format!("        match {} {{\n", sub));
                code.push_str("            Ok(val) => return Ok(val),\n");
                code.push_str("            Err(_) => {}\n");
                code.push_str("        }\n");
            }
            code.push_str("        Err(self.error(\"No alternative matched\"))\n");
            code
        }
        Expr::Repeat(e) => {
            let sub = generate_expr_code(e, &None);
            format!(
                r#"        let mut results = Vec::new();
        while let Ok(val) = {} {{
            results.push(val);
        }}
        Ok(AST::Value(format!("{{:?}}", results)))"#,
                sub
            )
        }
        Expr::Plus(e) => {
            let sub = generate_expr_code(e, &None);
            format!(
                r#"        let mut results = Vec::new();
        match {} {{
            Ok(val) => results.push(val),
            Err(e) => return Err(e),
        }}
        while let Ok(val) = {} {{
            results.push(val);
        }}
        Ok(AST::Value(format!("{{:?}}", results)))"#,
                sub, sub
            )
        }
        Expr::Optional(e) => {
            let sub = generate_expr_code(e, &None);
            format!(
                r#"        let saved_pos = self.pos;
        match {} {{
            Ok(val) => Ok(val),
            Err(_) => {{
                self.pos = saved_pos;
                Ok(AST::Value("none".to_string()))
            }}
        }}"#,
                sub
            )
        }
        Expr::AndPredicate(e) => {
            let sub = generate_expr_code(e, &None);
            format!(
                r#"        let saved_pos = self.pos;
        match {} {{
            Ok(_) => {{
                self.pos = saved_pos;
                Ok(AST::Value("satisfied".to_string()))
            }}
            Err(_) => Err(self.error("And predicate failed")),
        }}"#,
                sub
            )
        }
        Expr::NotPredicate(e) => {
            let sub = generate_expr_code(e, &None);
            format!(
                r#"        let saved_pos = self.pos;
        match {} {{
            Ok(_) => {{
                self.pos = saved_pos;
                Err(self.error("Not predicate failed"))
            }}
            Err(_) => {{
                self.pos = saved_pos;
                Ok(AST::Value("not satisfied".to_string()))
            }}
        }}"#,
                sub
            )
        }
        Expr::Group(e) => generate_expr_code(e, ast_action),
        Expr::RuleRef(name) => {
            format!("self.parse_{}()", name)
        }
        Expr::Char(c) => {
            let ch = *c as u8 as char;
            format!(
                r#"        self.expect_char('{}')?;
        Ok(AST::Value(format!("'{{}}'", '{}')))"#,
                ch, ch
            )
        }
        Expr::String(s) => {
            let mut code = String::new();
            for c in s.chars() {
                code.push_str(&format!("        self.expect_char('{}')?;\n", c));
            }
            code.push_str(&format!("        Ok(AST::Value(\"{}\".to_string()))\n", s));
            code
        }
        Expr::CharClass(class) => {
            let items: Vec<String> = class
                .items
                .iter()
                .map(|item| format!("({}, {})", item.start, item.end))
                .collect();
            let negated = class.negated;
            format!(
                r#"        self.match_char_class({}, &[{}])?;
        Ok(AST::Value("char_class".to_string()))"#,
                negated,
                items.join(", ")
            )
        }
        Expr::Regex(pattern) => {
            format!(
                r#"        self.match_regex("{}")?;
        Ok(AST::Value("regex".to_string()))"#,
                pattern
            )
        }
        Expr::Eof => r#"        if self.peek().is_none() {
            Ok(AST::Value("EOF".to_string()))
        } else {
            Err(self.error("Expected EOF"))
        }"#
        .to_string(),
        Expr::AnyChar => r#"        match self.consume() {
            Some(ch) => Ok(AST::Value(format!("{}", ch))),
            None => Err(self.error("Expected any character")),
        }"#
        .to_string(),
    }
}
