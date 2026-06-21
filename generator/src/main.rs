use generator::DFA;
use generator::args::Args;
use generator::config::Config;
use generator::core::error::{Error, Result};
use generator::core::logging::init_logging;
use generator::core::regex::ast::RegexType;
use generator::core::regex::parser::Parser as RegexParser;
use generator::core::template::expander::expand_patterns;
use generator::scanner::dfa::build_dfa;
use generator::scanner::generator::generate_code;
use generator::scanner::nfa::build_nfa;
use log::{debug, info};
use std::fs;
use std::io::Read;
use std::path::Path;
use std::process::Command;
use std::time::Instant;
use tempfile::tempdir;
mod version;
mod meow;
fn main() -> Result<()> {
    
    if std::env::args().any(|arg| arg == "--version" || arg == "-V") {
        version::print_version();
        return Ok(());
    }

    let start_time = Instant::now();
    let args = Args::from_env();

    init_logging(args.verbose);

    if args.meow {
        meow::print_meow();
        return Ok(());
    }
    
    
    let is_test_mode = args.is_test_mode();
    let is_validate = args.is_validate();
    let is_dump_dfa = args.is_dump_dfa();
    let test_input = args.get_test_input().map(|s| s.to_string());
    let dump_output = args.get_dump_dfa_output().cloned();

    
    let config_file = args.get_config_file().cloned();
    if config_file.is_none() && !is_test_mode && !is_validate {
        return Err(Error::config("No configuration file provided (see --help)"));
    }

    
    let config_content = if let Some(path) = config_file {
        if path == Path::new("-") {
            let mut buffer = String::new();
            std::io::stdin()
                .read_to_string(&mut buffer)
                .map_err(|e| Error::io(format!("Failed to read from stdin: {}", e)))?;
            buffer
        } else {
            fs::read_to_string(&path)
                .map_err(|e| Error::io(format!("Failed to read config file {:?}: {}", path, e)))?
        }
    } else {
        String::new()
    };

    let mut config: Config = serde_json::from_str(&config_content)
        .map_err(|e| Error::config(format!("Invalid JSON in config: {}", e)))?;

    
    if let Some(output) = args.output_file {
        config.output_file = Some(output);
    }
    if let Some(name) = args.scanner_name {
        config.scanner_name = Some(name);
    }
    if let Some(tmpl) = args.template_file {
        config.template_file = Some(tmpl);
    }
    let compress = args.compress;
    let parallel_minimize = args.parallel_minimize;

    
    if is_validate {
        info!("Configuration is valid");
        return Ok(());
    }

    
    let patterns = if let Some(templates) = config.templates {
        info!("Expanding templates ({} defined)", templates.len());
        let expand_start = Instant::now();
        let expanded = expand_patterns(config.patterns, &templates, |pat| &mut pat.regex)
            .map_err(|e| Error::config(format!("Template expansion failed: {}", e)))?;
        debug!(
            "Template expansion completed in {:?}",
            expand_start.elapsed()
        );
        expanded
    } else {
        config.patterns
    };

    
    info!("Parsing regular expressions...");
    let parse_start = Instant::now();

    let mut all_asts: Vec<(String, RegexType)> = Vec::new();
    let mut token_names = Vec::new();

    for pat in &patterns {
        debug!("Parsing token '{}': '{}'", pat.token, pat.regex);
        let mut parser = RegexParser::new(&pat.regex);
        let ast = parser.parse().map_err(|e| match e {
            Error::Parse { message, line, col } => Error::parse(
                format!("{} (while parsing token '{}')", message, pat.token),
                line,
                col,
            ),
            _ => Error::config(format!(
                "Failed to parse regex for token '{}': {}",
                pat.token, e
            )),
        })?;
        token_names.push(pat.token.clone());
        all_asts.push((pat.token.clone(), ast));
    }

    let mut skip_token_ids = Vec::new();
    for (idx, regex_str) in config.skip_regexes.iter().enumerate() {
        debug!("Parsing skip regex '{}'", regex_str);
        let mut parser = RegexParser::new(regex_str);
        let ast = parser.parse().map_err(|e| {
            Error::config(format!("Failed to parse skip regex '{}': {}", regex_str, e))
        })?;
        let token_id = all_asts.len();
        let token_name = format!("__SKIP_{}", idx);
        all_asts.push((token_name, ast));
        skip_token_ids.push(token_id);
    }

    info!(
        "All {} regexes parsed in {:?}",
        all_asts.len(),
        parse_start.elapsed()
    );

    
    info!("Building NFA...");
    let nfa_start = Instant::now();
    let patterns_for_nfa: Vec<(usize, RegexType)> = all_asts
        .into_iter()
        .enumerate()
        .map(|(id, (_, ast))| (id, ast))
        .collect();
    let nfa = build_nfa(&patterns_for_nfa);
    info!(
        "NFA built: {} states, {} transitions, {} accept states in {:?}",
        nfa.states.len(),
        nfa.states.iter().map(|s| s.edges.len()).sum::<usize>(),
        nfa.accepts.len(),
        nfa_start.elapsed()
    );

    
    info!("Building DFA...");
    let dfa_build_start = Instant::now();
    let mut dfa = build_dfa(&nfa);
    info!(
        "DFA built: {} states, {} character classes in {:?}",
        dfa.transitions.len(),
        dfa.intervals.len(),
        dfa_build_start.elapsed()
    );

    
    info!("Minimizing DFA...");
    let minimize_start = Instant::now();
    let states_before = dfa.transitions.len();
    if parallel_minimize {
        info!("Using parallel minimization (experimental)");
        dfa.minimize_parallel();
    } else {
        info!("Using serial minimization");
        dfa.minimize();
    }
    let states_after = dfa.transitions.len();
    info!(
        "DFA minimized: {} -> {} states (reduced by {:.1}%) in {:?}",
        states_before,
        states_after,
        (states_before - states_after) as f64 / states_before as f64 * 100.0,
        minimize_start.elapsed()
    );

    
    if is_dump_dfa {
        let dot = dfa_to_dot(&dfa);
        if let Some(output) = dump_output {
            fs::write(&output, dot)
                .map_err(|e| Error::io(format!("Failed to write dot file {:?}: {}", &output, e)))?;
            info!("DFA dot output written to {:?}", &output);
        } else {
            print!("{}", dot);
        }
        return Ok(());
    }

    
    let template_content = if let Some(template_file) = config.template_file {
        info!("Loading external template from {:?}", template_file);
        let path = Path::new(&template_file);
        let content = fs::read_to_string(path)
            .map_err(|e| Error::io(format!("Failed to read template file {:?}: {}", path, e)))?;
        Some(content)
    } else {
        info!("Using built-in Rust template");
        None
    };

    let scanner_name = config
        .scanner_name
        .as_deref()
        .unwrap_or("Scanner")
        .to_string();

    
    info!("Generating code for scanner '{}'...", scanner_name);
    let gen_start = Instant::now();
    let code = generate_code(
        &dfa,
        token_names,
        &scanner_name,
        template_content.as_deref(),
        compress,
        &skip_token_ids,
    )?;
    debug!("Code generation took {:?}", gen_start.elapsed());
    info!("Generated {} characters of Rust source", code.len());

    
    if let Some(input) = test_input {
        info!(
            "Test mode: compiling and running scanner on input '{}'",
            input
        );
        run_test(&code, &scanner_name, &input)?;
        return Ok(());
    }

    
    if let Some(output_file) = config.output_file {
        info!("Writing output to {}", output_file);
        fs::write(&output_file, &code).map_err(|e| {
            Error::io(format!(
                "Failed to write output file {}: {}",
                output_file, e
            ))
        })?;
        info!("Scanner successfully written to {}", output_file);
    } else {
        info!("Printing generated code to stdout");
        print!("{}", code);
    }

    info!("Total execution time: {:?}", start_time.elapsed());
    Ok(())
}


fn run_test(code: &str, scanner_name: &str, input: &str) -> Result<()> {
    let dir = tempdir().map_err(|e| Error::io(format!("Failed to create temp dir: {}", e)))?;
    let scanner_path = dir.path().join("scanner.rs");
    let driver_path = dir.path().join("driver.rs");

    fs::write(&scanner_path, code)
        .map_err(|e| Error::io(format!("Failed to write scanner.rs: {}", e)))?;

    let driver = format!(
        r#"
extern crate scanner;
use scanner::{{ {}, Token }};

fn main() {{
    let input = "{}";
    let mut scanner = {}::new(input);
    loop {{
        let (tok, start, end) = scanner.next_token();
        match tok {{
            Token::Eof => break,
            Token::Error(c) => {{
                eprintln!("Lexical error at pos {{}}: '{{}}'", start, c);
                std::process::exit(1);
            }}
            _ => {{
                let lexeme = scanner.lexeme(start, end);
                println!("{{:?}} {{}}", tok, lexeme);
            }}
        }}
    }}
}}
"#,
        scanner_name,
        input.replace('\\', "\\\\").replace('"', "\\\""),
        scanner_name
    );

    fs::write(&driver_path, driver)
        .map_err(|e| Error::io(format!("Failed to write driver.rs: {}", e)))?;

    let status = Command::new("rustc")
        .arg("--crate-type=lib")
        .arg(&scanner_path)
        .arg("-o")
        .arg(dir.path().join("libscanner.rlib"))
        .current_dir(dir.path())
        .status()
        .map_err(|e| Error::io(format!("Failed to compile scanner: {}", e)))?;
    if !status.success() {
        return Err(Error::other("Compilation of scanner failed"));
    }

    let status = Command::new("rustc")
        .arg("--extern")
        .arg(format!(
            "scanner={}",
            dir.path().join("libscanner.rlib").display()
        ))
        .arg(&driver_path)
        .arg("-o")
        .arg(dir.path().join("driver"))
        .current_dir(dir.path())
        .status()
        .map_err(|e| Error::io(format!("Failed to compile driver: {}", e)))?;
    if !status.success() {
        return Err(Error::other("Compilation of driver failed"));
    }

    let output = Command::new(dir.path().join("driver"))
        .current_dir(dir.path())
        .output()
        .map_err(|e| Error::io(format!("Failed to run driver: {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::other(format!("Driver execution failed: {}", stderr)));
    }

    println!("{}", String::from_utf8_lossy(&output.stdout));
    if !output.stderr.is_empty() {
        eprintln!("{}", String::from_utf8_lossy(&output.stderr));
    }

    Ok(())
}


fn dfa_to_dot(dfa: &DFA) -> String {
    let mut dot = String::from("digraph DFA {\n");
    dot.push_str("  rankdir=LR;\n");
    dot.push_str("  node [shape=circle];\n");
    for (i, acc) in dfa.accept.iter().enumerate() {
        if acc.is_some() {
            dot.push_str(&format!("  {} [shape=doublecircle];\n", i));
        }
    }
    dot.push_str(&format!("  start [shape=point];\n"));
    dot.push_str(&format!("  start -> {};\n", dfa.start_state));
    for (i, row) in dfa.transitions.iter().enumerate() {
        for (j, &next) in row.iter().enumerate() {
            if let Some(next_state) = next {
                let (start, end) = dfa.intervals[j];
                let label = if start + 1 == end {
                    format!("{:?}", start as u8 as char)
                } else if end == start + 2 {
                    format!("{:?}..{:?}", start as u8 as char, (end - 1) as u8 as char)
                } else {
                    format!("0x{:04X}..0x{:04X}", start, end - 1)
                };
                dot.push_str(&format!(
                    "  {} -> {} [label=\"{}\"];\n",
                    i, next_state, label
                ));
            }
        }
    }
    dot.push_str("}\n");
    dot
}
