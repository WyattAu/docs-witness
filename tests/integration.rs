//! Golden-path tests: execution, fuel, traps, assertions, determinism.

// Test-only lifts (docs-core decoder-pattern precedent): expectations
// carry context on the setup half of each test and match-arm panics are
// the failure voice. Library paths keep the denies.
#![allow(clippy::expect_used, clippy::panic)]

use docs_witness::{
    check, run, AssertionError, Assertions, WitnessConfig, WitnessError, DEFAULT_FUEL,
};

/// Convert wat text to wasm binary (the compiler's job; tests do it
/// inline with the wat crate's own converter).
fn wasm(wat: &str) -> Vec<u8> {
    wat::parse_str(wat).expect("test wat compiles")
}

fn hello_wat() -> String {
    r#"
    (module
      (import "wasi_snapshot_preview1" "fd_write"
        (func $fd_write (param i32 i32 i32 i32) (result i32)))
      (memory (export "memory") 1)
      (data (i32.const 0) "\10\00\00\00\06\00\00\00")
      (data (i32.const 16) "hello\n")
      (func (export "_start")
        i32.const 1
        i32.const 0
        i32.const 1
        i32.const 48
        call $fd_write
        drop))
    "#
    .to_string()
}

#[test]
fn hello_witness() {
    let w = run(&wasm(&hello_wat()), &WitnessConfig::default()).expect("runs");
    assert_eq!(w.exit, 0);
    assert_eq!(w.stdout, b"hello\n");
    assert!(w.fuel_consumed > 0, "fuel accounting active");
    assert!(!w.witness_version.is_empty());
    assert!(!w.wasmtime_version.is_empty());
}

#[test]
fn proc_exit_is_an_exit_not_an_error() {
    let wat = r#"
    (module
      (import "wasi_snapshot_preview1" "proc_exit" (func $exit (param i32)))
      (memory (export "memory") 1)
      (func (export "_start") i32.const 42 call $exit))
    "#;
    let w = run(&wasm(wat), &WitnessConfig::default()).expect("runs");
    assert_eq!(w.exit, 42);
}

#[test]
fn traps_are_typed() {
    let wat = "(module (func (export \"_start\") unreachable))";
    match run(&wasm(wat), &WitnessConfig::default()) {
        Err(WitnessError::Trap { code, detail }) => {
            assert!(code.contains("Unreachable"), "code={code}");
            assert!(!detail.is_empty());
        }
        other => panic!("expected Trap, got {other:?}"),
    }
}

#[test]
fn fuel_exhaustion_is_typed_and_equals_the_limit() {
    // A loop whose body burns fuel: an empty `loop br 0` is a Cranelift
    // compile error, not a fuel case (found by this very test).
    let wat = r#"(module (memory (export "memory") 1) (func $f (export "_start") (local i32) (loop (local.set 0 (i32.add (local.get 0) (i32.const 1))) (br 0))))"#;
    let cfg = WitnessConfig {
        fuel: 5_000,
        ..WitnessConfig::default()
    };
    match run(&wasm(wat), &cfg) {
        Err(WitnessError::OutOfFuel { limit, consumed }) => {
            assert_eq!(limit, 5_000);
            assert_eq!(consumed, limit, "exhaustion consumes the budget");
        }
        other => panic!("expected OutOfFuel, got {other:?}"),
    }
}

#[test]
fn missing_entrypoint_is_typed() {
    let wat = r#"(module (memory (export "memory") 1))"#;
    assert!(matches!(
        run(&wasm(wat), &WitnessConfig::default()),
        Err(WitnessError::NoEntrypoint)
    ));
}

#[test]
fn invalid_bytes_are_a_compile_error() {
    let garbage = [0u8; 32];
    assert!(matches!(
        run(&garbage, &WitnessConfig::default()),
        Err(WitnessError::Compile(_))
    ));
}

#[test]
fn runs_are_deterministic_except_wall_time() {
    let bytes = wasm(&hello_wat());
    let one = run(&bytes, &WitnessConfig::default()).expect("run 1");
    let two = run(&bytes, &WitnessConfig::default()).expect("run 2");
    assert_eq!(one.stdout, two.stdout);
    assert_eq!(one.exit, two.exit);
    assert_eq!(one.fuel_consumed, two.fuel_consumed, "fuel is stable");
    // Fresh instance per run: memory state never leaks across witnesses.
    assert_ne!(one.wall_ns, 0);
}

#[test]
fn stdout_capacity_bounds_a_chatty_guest() {
    let wat = r#"
    (module
      (import "wasi_snapshot_preview1" "fd_write"
        (func $fd_write (param i32 i32 i32 i32) (result i32)))
      (memory (export "memory") 1)
      (data (i32.const 0) "\20\00\00\00\04\00\00\00")
      (data (i32.const 32) "xxxx")
      (func (export "_start")
        i32.const 1
        i32.const 0
        i32.const 1
        i32.const 48
        call $fd_write
        drop))
    "#;
    let cfg = WitnessConfig {
        fuel: DEFAULT_FUEL,
        stdout_capacity: 2,
    };
    // The pipe rejects the write beyond capacity (the guest sees the write
    // error, the host never blocks): the run completes and stdout stays
    // within the cap.
    let w = run(&wasm(wat), &cfg).expect("bounded run completes");
    assert!(w.stdout.len() <= cfg.stdout_capacity);
}

#[test]
fn assertions_pass_and_fail_typed() {
    let w = run(&wasm(&hello_wat()), &WitnessConfig::default()).expect("runs");

    check(
        &w,
        &Assertions {
            exit: Some(0),
            stdout: Some("hello\n".into()),
        },
    )
    .expect("both assertions hold");

    match check(
        &w,
        &Assertions {
            exit: Some(1),
            stdout: None,
        },
    ) {
        Err(e @ AssertionError::Exit { .. }) => {
            assert!(e.to_string().contains("assert_exit failed"));
        }
        other => panic!("expected Exit mismatch, got {other:?}"),
    }
    match check(
        &w,
        &Assertions {
            exit: None,
            stdout: Some("nope".into()),
        },
    ) {
        Err(e @ AssertionError::Stdout { .. }) => {
            assert!(e.to_string().contains("assert_stdout failed"));
        }
        other => panic!("expected Stdout mismatch, got {other:?}"),
    }
    check(&w, &Assertions::default()).expect("no assertions, no failure");
}

#[test]
fn link_failures_are_typed_and_display_is_informative() {
    // A WASI import the preview1 surface cannot provide: linking fails.
    let wat = r#"
    (module
      (import "wasi_snapshot_preview1" "bogus" (func))
      (memory (export "memory") 1)
      (func (export "_start")))
    "#;
    let Err(WitnessError::Link(e)) = run(&wasm(wat), &WitnessConfig::default()) else {
        panic!("expected Link, got a different outcome")
    };
    assert!(!e.to_string().is_empty());
}

#[test]
fn error_display_covers_remaining_variants() {
    // Fuel-accounting faults are engine-configuration bugs, not guest
    // behavior; they cannot be triggered through the public surface, so
    // construct the variant directly.
    let fuel = WitnessError::Fuel("boom".into());
    assert!(fuel.to_string().contains("fuel accounting"));

    let garbage = [0u8; 8];
    let Err(WitnessError::Compile(c)) = run(&garbage, &WitnessConfig::default()) else {
        panic!("expected Compile for garbage bytes")
    };
    assert!(!c.to_string().is_empty());

    let no_start = WitnessError::NoEntrypoint;
    assert!(no_start.to_string().contains("_start"));
}

#[test]
fn payload_serializes_round_trip() {
    let w = run(&wasm(&hello_wat()), &WitnessConfig::default()).expect("runs");
    let json = serde_json::to_string(&w).expect("serializes");
    let back: docs_witness::Witness = serde_json::from_str(&json).expect("deserializes");
    assert_eq!(w, back);
}
