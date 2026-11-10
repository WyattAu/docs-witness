# docs-witness

Deterministic WASI snippet execution witnesses for documentation sites:
one sandboxed `wasm32-wasip1` execution per run in a fresh wasmtime
engine, fuel-bounded compute, capacity-bounded stdout capture, no
filesystem, no sockets. The serializable witness payload is the static
evidence a docs compiler embeds into a page; the assertion half turns
declared expectations into build-time failures. Part of the docs-core
extension program (`docs-core/SPEC.md` REQ-W).

## Module map

| Module | Contents |
|---|---|
| `run` | execute one module in a fresh engine/store/WASI context |
| `Witness` | the payload: stdout bytes, exit code, fuel spend, wall time, provenance versions; serde-serializable |
| `WitnessConfig` | fuel budget (default 20M) and stdout capacity (default 4 MiB) |
| `check` / `Assertions` | `assert_exit` / `assert_stdout` evaluation, typed mismatches |
| `WitnessError` | compile / link / trap / out-of-fuel / no-entrypoint / fuel-accounting |

## Guarantees (0.1, honest)

- **Fresh per run.** No state crosses witnesses: new engine, store, and
  WASI context every call; stdout/exit/fuel are reproducible on the same
  host (wall time is reporting only).
- **Bounded, not hanging.** Fuel bounds compute; the stdout pipe rejects
  writes beyond its capacity (the guest sees the write error; the host
  never blocks) so stdout stays within the cap. The v0.1 WASI surface
  adds no filesystem, no sockets, and no blocking calls, so a
  wall-clock timeout is unnecessary here; it arrives with that surface's
  extension, not silently absent.
- **Total.** Traps, fuel exhaustion, missing `_start`, and invalid bytes
  are typed errors; `proc_exit(n)` and a normal `_start` return are
  exits, not errors. A compiled snippet is untrusted input per the
  estate decoder pattern.
- **Provenance in the payload.** Every witness records the docs-witness
  version and the wasmtime engine version (read from Cargo.lock at build
  time).

## Out of scope for 0.1

Filesystem/socket WASI, wall-clock timeouts, stdin injection,
snapshotting/time travel. Each is a later increment with its own
requirement in `docs-core/SPEC.md`; none is silently missing.

## Layer

L1 - substrate: a WASM execution harness. Zero estate-internal
dependencies; the external floor is the wasmtime stack (pinned to the
newest release whose MSRV is the estate's 1.85 floor; currently
wasmtime 34).

## Tests

`cargo test` under clippy `-D warnings` with the estate decoder-pattern
denies (`unwrap`/`expect`/`panic`/`indexing_slicing` lifted in test
targets only). Determinism, fuel exhaustion, trap typing, capacity
bounding, missing entrypoints, assertion pass/fail, and payload
round-trip are all covered.

Licensed under MIT OR Apache-2.0.
