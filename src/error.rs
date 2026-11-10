//! Typed, exhaustive errors for witness execution and assertion checks.

/// Everything that can go wrong running a witness.
#[derive(Debug)]
#[non_exhaustive]
pub enum WitnessError {
    /// The bytes are not a valid module (or the engine could not be built).
    Compile(wasmtime::Error),
    /// Instantiation or linking failed (e.g. missing WASI imports that the
    /// preview1 surface cannot provide).
    Link(wasmtime::Error),
    /// The guest trapped at runtime (unreachable, division by zero, or a
    /// WASI call that failed hard, such as stdout beyond the pipe capacity).
    Trap {
        /// The wasmtime trap variant, for diagnosis.
        code: String,
        /// The full engine error chain, for diagnosis.
        detail: String,
    },
    /// The guest exceeded its fuel budget.
    OutOfFuel {
        /// The configured budget.
        limit: u64,
        /// Fuel actually consumed when the trap fired (equals the limit).
        consumed: u64,
    },
    /// The module has no `_start` export to run.
    NoEntrypoint,
    /// The fuel counters themselves failed (an engine configuration bug,
    /// not guest behavior).
    Fuel(String),
}

impl std::fmt::Display for WitnessError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Compile(e) => write!(f, "module compilation failed: {e}"),
            Self::Link(e) => write!(f, "instantiation failed: {e}"),
            Self::Trap { code, detail } => write!(f, "guest trapped ({code}): {detail}"),
            Self::OutOfFuel { limit, consumed } => {
                write!(f, "out of fuel: consumed {consumed} of {limit}")
            }
            Self::NoEntrypoint => write!(f, "module exports no _start entrypoint"),
            Self::Fuel(e) => write!(f, "fuel accounting failed: {e}"),
        }
    }
}

impl std::error::Error for WitnessError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Compile(e) | Self::Link(e) => Some(e.as_ref()),
            _ => None,
        }
    }
}

/// A witness assertion that did not hold.
#[derive(Debug)]
#[non_exhaustive]
pub enum AssertionError {
    /// The exit code differed from `assert_exit`.
    Exit {
        /// The expected code.
        expected: u32,
        /// The observed code.
        actual: u32,
    },
    /// The stdout differed from `assert_stdout` (exact match).
    Stdout {
        /// The expected stdout text.
        expected: String,
        /// The observed stdout, lossily decoded.
        actual: String,
    },
}

impl std::fmt::Display for AssertionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Exit { expected, actual } => {
                write!(f, "assert_exit failed: expected {expected}, got {actual}")
            }
            Self::Stdout { expected, actual } => {
                write!(
                    f,
                    "assert_stdout failed:\n  expected: {expected:?}\n  actual:   {actual:?}"
                )
            }
        }
    }
}

impl std::error::Error for AssertionError {}
