# Security Policy

## Reporting a vulnerability

Email wyatt_au@protonmail.com with the repo name in the subject. Please
include reproduction steps or a failing module; do not open a public
issue for an unfixed vulnerability.

## Content discipline

A compiled snippet is untrusted input (the estate decoder pattern).
Execution is sandboxed by construction: a fresh wasmtime engine per run,
no filesystem, no sockets, no host functions beyond WASI preview1,
fuel-bounded compute, and a capacity-bounded stdout pipe that traps
rather than blocks. Malformed modules and hostile guests produce typed
errors (`WitnessError`), never panics. If you find a module that panics
the host, hangs the runner, escapes the sandbox, or allocates
unboundedly, that is a security bug.

## Dependency posture

wasmtime is pinned to the newest release whose MSRV matches the estate's
1.85 floor and is updated through the weekly dependabot wave plus the
estate vet gate. `cargo vet` and `cargo deny` run in CI; advisories
against wasmtime are treated as blocking.

## Supported versions

Only the latest released `0.1.x` is supported with fixes.
