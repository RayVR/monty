use std::borrow::Cow;
use std::fmt;

use crate::exceptions::{ExceptionRaise, InternalRunError};
use crate::resource::ResourceError;

/// Errors that can occur during parsing or preparation of Python code.
///
/// Most errors are represented as `PreEvalExc` containing a Python exception,
/// allowing them to be displayed in Python's familiar exception format.
#[derive(Debug, Clone)]
pub enum ParseError {
    /// Internal parsing error (should not occur in normal usage).
    Internal(Cow<'static, str>),
    /// A Python exception raised during parsing or preparation.
    PreEvalExc(ExceptionRaise),
    /// Internal runtime error during preparation.
    PreEvalInternal(InternalRunError),
    /// Resource limit exceeded during preparation.
    PreEvalResource(ResourceError),
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Internal(s) => write!(f, "Internal parsing error: {s}"),
            Self::PreEvalExc(exc) => write!(f, "{}", exc.py_str()),
            Self::PreEvalInternal(s) => write!(f, "Internal error: {s}"),
            Self::PreEvalResource(s) => write!(f, "Resource error: {s}"),
        }
    }
}

impl From<ExceptionRaise> for ParseError {
    fn from(exc: ExceptionRaise) -> Self {
        Self::PreEvalExc(exc)
    }
}

impl From<InternalRunError> for ParseError {
    fn from(internal_run_error: InternalRunError) -> Self {
        Self::PreEvalInternal(internal_run_error)
    }
}
