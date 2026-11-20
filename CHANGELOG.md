# Changelog

All notable changes to this project are documented in this file.

## [0.1.0] - 2026-11-10

### Added

- `run`: fresh-instance WASI execution (wasmtime 34, preview1 sync
  linker), fuel-bounded via `Config::consume_fuel`, stdout captured
  through a capacity-bounded `MemoryOutputPipe` (the pipe rejects
  over-capacity writes; the host never blocks).
- `Witness` payload (serde): stdout, exit code, fuel consumed, wall time,
  docs-witness version, and the wasmtime engine version read from
  Cargo.lock at build time (REQ-W4 provenance).
- `Assertions` / `check`: `assert_exit` and exact `assert_stdout`
  checking with typed mismatches (REQ-W3).
- Typed errors for compile/link failures, guest traps, fuel exhaustion,
  missing `_start`, and fuel-accounting faults.
- Tests: determinism, fuel exhaustion, trap typing, capacity bounding,
  missing entrypoints, invalid bytes, assertion pass/fail, payload
  round-trip.

## [0.1.1] - 2026-11-18

### Security

- wasmtime / wasmtime-wasi bumped to 36.0.23: RUSTSEC-2026-0566/0567/
  0569/0571 (fd_renumber panic, shared-memory unsoundness, unbounded
  host memory growth) are patched in >=36.0.21 on the 36 line — the
  pinned major for the RUSTSEC-2025-0118 constraint stays satisfied.
