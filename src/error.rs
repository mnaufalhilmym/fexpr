#[derive(Debug)]
pub enum Error {
    Buffer(String),
    Unexpected(String),
    Invalid(String),
    Empty(String),
    Incomplete(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Buffer(err) => write!(f, "Buffer: {err}"),
            Error::Unexpected(err) => write!(f, "Unexpected: {err}"),
            Error::Invalid(err) => write!(f, "Invalid: {err}"),
            Error::Empty(err) => write!(f, "Empty: {err}"),
            Error::Incomplete(err) => write!(f, "Incomplete: {err}"),
        }
    }
}

impl std::error::Error for Error {}
