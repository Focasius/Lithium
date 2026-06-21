

use colored::*;

pub fn print_version() {
    let version = env!("CARGO_PKG_VERSION");
    let name = "Lithium Generator";
    let target = std::env::consts::ARCH;
    let os = std::env::consts::OS;
    let build_time = option_env!("BUILD_TIME").unwrap_or("unknown");
    let rustc_version = option_env!("RUSTC_VERSION").unwrap_or("unknown");

    let art = r#"
  ██╗     ██╗████████╗██╗  ██╗██╗██╗   ██╗███╗   ███╗
  ██║     ██║╚══██╔══╝██║  ██║██║██║   ██║████╗ ████║
  ██║     ██║   ██║   ███████║██║██║   ██║██╔████╔██║
  ██║     ██║   ██║   ██╔══██║██║██║   ██║██║╚██╔╝██║
  ███████╗██║   ██║   ██║  ██║██║╚██████╔╝██║ ╚═╝ ██║
  ╚══════╝╚═╝   ╚═╝   ╚═╝  ╚═╝╚═╝ ╚═════╝ ╚═╝     ╚═╝
"#;

    println!();
    for line in art.lines() {
        println!("{}", line.bright_cyan());
    }
    println!();

    println!("  {}  {}", "⚡".bright_yellow(), name.bright_cyan().bold());
    println!("  {}  {}", "Version".dimmed(), version.bright_green());
    println!("  {}  {} / {}", "Platform".dimmed(), os.bright_blue(), target.dimmed());
    println!("  {}  {}", "Rust".dimmed(), rustc_version.dimmed());
    println!("  {}  {}", "Built".dimmed(), build_time.dimmed());
    println!();
}