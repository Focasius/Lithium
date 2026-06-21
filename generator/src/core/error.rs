use std::fmt;
use std::io;

#[derive(Debug, Clone)]
pub enum Error {
    Parse {
        message: String,
        line: usize,
        col: usize,
    },

    Io(String),

    Config(String),

    Internal(String),

    Other(String),
}

pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    pub fn parse(message: impl Into<String>, line: usize, col: usize) -> Self {
        Self::Parse {
            message: message.into(),
            line,
            col,
        }
    }

    pub fn io(message: impl Into<String>) -> Self {
        Self::Io(message.into())
    }

    pub fn config(message: impl Into<String>) -> Self {
        Self::Config(message.into())
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self::Internal(message.into())
    }

    pub fn other(message: impl Into<String>) -> Self {
        Self::Other(message.into())
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Parse { message, line, col } => {
                write!(f, "Parse error at line {}, col {}: {}", line, col, message)
            }
            Self::Io(msg) => write!(f, "IO error: {}", msg),
            Self::Config(msg) => write!(f, "Configuration error: {}", msg),
            Self::Internal(msg) => write!(f, "Internal error: {}", msg),
            Self::Other(msg) => write!(f, "Error: {}", msg),
        }
    }
}

impl std::error::Error for Error {}

// ---- 便捷转换 ----

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        Self::Io(err.to_string())
    }
}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Self {
        Self::Config(err.to_string())
    }
}

impl From<String> for Error {
    fn from(s: String) -> Self {
        Self::Other(s)
    }
}

impl From<&str> for Error {
    fn from(s: &str) -> Self {
        Self::Other(s.to_string())
    }
}
