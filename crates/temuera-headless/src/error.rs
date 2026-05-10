use std::error::Error;
use std::fmt;
use std::io;
use std::path::PathBuf;

pub type Result<T> = std::result::Result<T, HeadlessError>;

#[derive(Debug)]
pub enum HeadlessError {
    Usage(String),
    InvalidGameDir(PathBuf),
    Io { action: String, source: io::Error },
}

impl HeadlessError {
    pub fn io(action: impl Into<String>, source: io::Error) -> Self {
        Self::Io {
            action: action.into(),
            source,
        }
    }

    pub fn exit_code(&self) -> u8 {
        match self {
            Self::Usage(_) | Self::InvalidGameDir(_) => 2,
            Self::Io { .. } => 1,
        }
    }
}

impl fmt::Display for HeadlessError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Usage(message) => write!(f, "{message}"),
            Self::InvalidGameDir(path) => {
                write!(f, "game directory does not exist: {}", path.display())
            }
            Self::Io { action, source } => write!(f, "{action}: {source}"),
        }
    }
}

impl Error for HeadlessError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            _ => None,
        }
    }
}
