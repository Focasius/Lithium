use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(author, about = "Lithium - Lexer/Parser Generator", long_about = None)]
pub struct Args {
    #[command(subcommand)]
    pub command: Option<Commands>,

    
    #[arg(value_name = "CONFIG_FILE")]
    pub config_file: Option<PathBuf>,

    
    #[arg(
        short,
        long,
        value_name = "FILE",
        env = "LITHIUM_OUTPUT",
        global = true
    )]
    pub output_file: Option<String>,

    
    #[arg(short, long, value_name = "NAME", env = "LITHIUM_NAME", global = true)]
    pub scanner_name: Option<String>,

    
    #[arg(
        short,
        long,
        value_name = "FILE",
        env = "LITHIUM_TEMPLATE",
        global = true
    )]
    pub template_file: Option<String>,

    
    #[arg(long, env = "LITHIUM_COMPRESS", global = true)]
    pub compress: bool,

    
    #[arg(long, env = "LITHIUM_PARALLEL_MINIMIZE", global = true)]
    pub parallel_minimize: bool,

    
    #[arg(short, long, action = clap::ArgAction::Count, global = true)]
    pub verbose: u8,

    
    #[arg(long, value_name = "INPUT", global = true)]
    pub test_input: Option<String>,

    
    #[arg(long, value_name = "FILE")]
    pub dump_dfa: Option<PathBuf>,

    #[arg(long, hide = true)]
    pub meow: bool

}

#[derive(Debug, Subcommand)]
pub enum Commands {
    
    Generate {
        #[arg(value_name = "CONFIG_FILE")]
        config_file: Option<PathBuf>,
    },
    
    Test {
        #[arg(value_name = "CONFIG_FILE")]
        config_file: Option<PathBuf>,
        #[arg(short, long, value_name = "INPUT")]
        input: String,
    },
    
    DumpDfa {
        #[arg(value_name = "CONFIG_FILE")]
        config_file: Option<PathBuf>,
        #[arg(short, long, value_name = "FILE")]
        output: Option<PathBuf>,
    },
    
    Validate {
        #[arg(value_name = "CONFIG_FILE")]
        config_file: Option<PathBuf>,
    },
}

impl Args {
    pub fn from_env() -> Self {
        Self::parse()
    }

    pub fn get_config_file(&self) -> Option<&PathBuf> {
        match &self.command {
            Some(Commands::Generate { config_file }) => {
                config_file.as_ref().or(self.config_file.as_ref())
            }
            Some(Commands::Test { config_file, .. }) => {
                config_file.as_ref().or(self.config_file.as_ref())
            }
            Some(Commands::DumpDfa { config_file, .. }) => {
                config_file.as_ref().or(self.config_file.as_ref())
            }
            Some(Commands::Validate { config_file }) => {
                config_file.as_ref().or(self.config_file.as_ref())
            }
            None => self.config_file.as_ref(),
        }
    }

    pub fn is_test_mode(&self) -> bool {
        matches!(self.command, Some(Commands::Test { .. })) || self.test_input.is_some()
    }

    pub fn get_test_input(&self) -> Option<&str> {
        match &self.command {
            Some(Commands::Test { input, .. }) => Some(input.as_str()),
            _ => self.test_input.as_deref(),
        }
    }

    pub fn is_dump_dfa(&self) -> bool {
        matches!(self.command, Some(Commands::DumpDfa { .. }))
    }

    pub fn is_validate(&self) -> bool {
        matches!(self.command, Some(Commands::Validate { .. }))
    }

    pub fn get_dump_dfa_output(&self) -> Option<&PathBuf> {
        match &self.command {
            Some(Commands::DumpDfa { output, .. }) => output.as_ref(),
            _ => None,
        }
    }
}
