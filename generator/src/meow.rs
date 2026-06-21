

use colored::*;

pub fn print_meow() {
    let cat = r#"
       /\_/\
      ( o.o )
       > ^ <
"#;

    let slogan = "This Generator has super cat powers! ✨";

    println!("\n{}", cat.bright_yellow());
    println!("  {}", slogan.bright_cyan().bold());
    println!();
}