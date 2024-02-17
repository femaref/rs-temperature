
use esp_idf_hal::i2c::I2cError;

use std::{error, fmt, num::Wrapping};

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[allow(deprecated)]
#[non_exhaustive]
pub enum ErrorKind {
    CalibrationLengthError,
    Other,
}

#[derive(Debug, thiserror::Error)]
pub struct Error {
    kind: ErrorKind,
    repr: Option<Box<dyn error::Error + Send + Sync>>,
}

impl Error {
    pub fn new<E>(kind: ErrorKind, error: E) -> Error
    where
        E: Into<Box<dyn error::Error + Send + Sync>>,
    {
        Self::_new(kind, error.into())
    }

    fn _new(kind: ErrorKind, error: Box<dyn error::Error + Send + Sync>) -> Error {
        Error {
            repr: error.into(),
            kind: kind,
        }
    }

    pub fn other(error: Box<dyn error::Error + Send + Sync>) -> Error {
        Self::_new(ErrorKind::Other, error)
    }
}

impl From<I2cError> for Error {
    fn from(value: I2cError) -> Self {
        Error::other(value.into())
    }
}

impl From<ErrorKind> for Error {
    fn from(value: ErrorKind) -> Self {
        Error {
            kind: value,
            repr: None,
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.kind {
            ErrorKind::Other => match &self.repr {
                Some(e) => write!(f, "bme280 error: {}", e),
                None => f.write_str(""),
            },
            _ => f.write_str(self.kind.as_str()),
        }
    }
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl ErrorKind {
    pub(crate) fn as_str(&self) -> &'static str {
        use ErrorKind::*;
        // tidy-alphabetical-start
        match *self {
            CalibrationLengthError => "provided vector is not 42 bytes long",
            Other => "other",
        }
    }
}