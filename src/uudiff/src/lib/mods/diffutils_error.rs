// This file is part of the uutils diffutils package.
//
// For the full copyright and license information, please view the LICENSE-*
// files that was distributed with this source code.

//! Common errors for all diffutils utilities.

use std::ffi::OsString;

use crate::{error::UError, translate};

/// Contains common DiffUtils errors and their text messages.
#[derive(Debug)]
pub enum DiffUtilsError {
    /// When a util does not handle directories (e.g. cmp).
    ///
    /// Param: wrong operand (dir name)
    DirectoryNotAllowed(OsString),

    /// Generic IO error, Display handled by [crate::error::UIoError]
    Io(Box<dyn UError>),
    IoDouble(Box<dyn UError>, Box<dyn UError>),
}

impl std::error::Error for DiffUtilsError {}

impl UError for DiffUtilsError {
    fn code(&self) -> i32 {
        2
    }
}

impl From<std::io::Error> for DiffUtilsError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e.into())
    }
}

impl std::fmt::Display for DiffUtilsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let msg = match self {
            Self::DirectoryNotAllowed(dir) => {
                translate!("error-is-a-directory", "file" => dir.to_string_lossy())
            }
            Self::Io(e) => {
                // dbg!("Io");
                return e.fmt(f);
            }
            Self::IoDouble(e1, e2) => {
                format!("{e1}\n{}: {e2}", uucore::util_name())
            }
        };

        write!(f, "{msg}")
    }
}
