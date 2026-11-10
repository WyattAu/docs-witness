//! docs-witness: deterministic WASI snippet execution witnesses
//! (docs-core extension program, `SPEC.md` REQ-W).
//!
//! A witness is one sandboxed execution of a `wasm32-wasip1` module in a
//! fresh wasmtime engine: fuel-bounded compute, a capacity-bounded stdout
//! pipe, no filesystem, no sockets. The recorded payload (stdout, exit
//! code, fuel spend, wall time, provenance versions) is the static
//! evidence a docs compiler embeds into the page; the assertion half
//! turns declared expectations into build-time failures.
//!
//! ```
//! use docs_witness::{run, WitnessConfig};
//!
//! // A tiny WASI module that prints via fd_write; real callers feed
//! // modules the compiler produced from ```witness fences.
//! let wat = r#"
//!     (module
//!       (import "wasi_snapshot_preview1" "fd_write"
//!         (func $fd_write (param i32 i32 i32 i32) (result i32)))
//!       (memory (export "memory") 1)
//!       (data (i32.const 0) "\10\00\00\00\05\00\00\00")
//!       (data (i32.const 16) "hello")
//!       (func (export "_start")
//!         i32.const 1
//!         i32.const 0
//!         i32.const 1
//!         i32.const 48
//!         call $fd_write
//!         drop))
//! "#;
//! let wasm = wat::parse_str(wat).unwrap();
//! let witness = run(&wasm, &WitnessConfig::default()).unwrap();
//! assert_eq!(witness.exit, 0);
//! assert_eq!(witness.stdout, b"hello");
//! ```
//!
//! # Scope (0.1, honest)
//!
//! In: fresh-instance execution, fuel bounds, stdout capture with
//! capacity bounds, typed errors for traps/fuel/missing entrypoints,
//! exit-code and stdout assertions, serialized payload, deterministic
//! runs on the same host (stdout/exit/fuel are reproducible; wall time is
//! reporting only).
//! Out: filesystem/socket WASI (the pipe-trap is the deliberate bound),
//! wall-clock timeouts via epoch interruption (they arrive with that
//! surface; fuel is the compute bound here), stdin injection, and
//! snapshotting/time travel. Each absence is a later increment, not
//! silent.
//!
//! # Layer
//!
//! L1 - substrate: a WASM execution harness. Zero estate-internal
//! dependencies; the external floor is the wasmtime stack.

#![deny(missing_docs)]
#![forbid(unsafe_code)]

mod asserts;
mod error;
mod witness;

pub use asserts::{check, Assertions};
pub use error::{AssertionError, WitnessError};
pub use witness::{run, Witness, WitnessConfig, DEFAULT_FUEL, DEFAULT_STDOUT_CAPACITY};

/// The crate's version, for callers that report it.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
