// Take a look at the license at the top of the repository in the LICENSE file.

use std::{error, fmt, io};

/// Error type covering the main errors which can happen when querying system information.
#[derive(Debug)]
pub enum Error {
    /// This feature is not supported on this platform.
    Unsupported,
    /// An IO error happened when querying the information.
    Io(io::Error),
    /// Any non-IO error on supported platforms.
    Other(Box<dyn error::Error + Send + Sync + 'static>),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::Unsupported => f.write_str("unsupported"),
            Error::Io(e) => fmt::Display::fmt(e, f),
            Error::Other(e) => fmt::Display::fmt(e, f),
        }
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Error::Io(e) => Some(e),
            Error::Other(e) => Some(&**e),
            _ => None,
        }
    }
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Error::Io(e)
    }
}

impl From<&'static str> for Error {
    fn from(e: &'static str) -> Self {
        Error::Other(e.into())
    }
}
