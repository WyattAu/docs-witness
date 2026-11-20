# Pattern adoption

docs-witness is the L1 deterministic WASI snippet execution crate: fresh
wasmtime instance per run, fuel-bounded, captured stdout, serializable
payload. Declared per references/adoption-format.md in pattern-standards.

| Pattern | Status | Evidence |
|---|---|---|
| T50 | partial | src/witness.rs:62 (fresh wasmtime instance per run, preview1-only linker, capped stdout pipe, fuel bound enabled at src/witness.rs:72; the sandbox mechanism is applied to docs snippets, not agent tools) |
