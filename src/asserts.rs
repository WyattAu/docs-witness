//! The assertion half: `assert_stdout` / `assert_exit` checking
//! (docs-core extension program, `SPEC.md` REQ-W3).

use crate::error::AssertionError;
use crate::witness::Witness;

/// The assertions a `witness` fence declares. Every `Some` field must
/// hold; `None` fields are unchecked.
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Assertions {
    /// Expected WASI exit code (`assert_exit`).
    pub exit: Option<u32>,
    /// Expected stdout, exact byte match after lossless capture
    /// (`assert_stdout`; compare as UTF-8 with lossy decoding for
    /// diagnostics).
    pub stdout: Option<String>,
}

/// Check a witness against declared assertions. A `None` assertion is
/// unchecked; a `Some` assertion must match exactly.
pub fn check(w: &Witness, a: &Assertions) -> Result<(), AssertionError> {
    if let Some(expected) = a.exit {
        if w.exit != expected {
            return Err(AssertionError::Exit {
                expected,
                actual: w.exit,
            });
        }
    }
    if let Some(expected) = &a.stdout {
        let actual = String::from_utf8_lossy(&w.stdout);
        if actual != expected.as_str() {
            return Err(AssertionError::Stdout {
                expected: expected.clone(),
                actual: actual.into_owned(),
            });
        }
    }
    Ok(())
}
