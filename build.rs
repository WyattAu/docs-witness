//! Build-time provenance: the wasmtime dependency version is read from
//! Cargo.lock and embedded so every witness records the engine version it
//! ran under (SPEC.md REQ-W4). Std-only parsing; "unknown" on any failure.

use std::fs;

fn main() {
    println!("cargo:rerun-if-changed=Cargo.lock");
    let version = dep_version("wasmtime").unwrap_or_else(|| "unknown".to_string());
    println!("cargo:rustc-env=DOCS_WITNESS_WASMTIME_VERSION={version}");
}

/// The `version` of the first `[[package]]` block named `name`.
fn dep_version(name: &str) -> Option<String> {
    let lock = fs::read_to_string("Cargo.lock").ok()?;
    let mut in_block = false;
    let mut matched_name = false;
    for line in lock.lines() {
        if line.starts_with("[[package]]") {
            // A new block ends the previous one before a version was seen.
            if matched_name {
                return None;
            }
            in_block = true;
            matched_name = false;
            continue;
        }
        if !in_block {
            continue;
        }
        if let Some(rest) = line.strip_prefix("name = ") {
            matched_name = strip_quotes(rest) == name;
        } else if matched_name {
            if let Some(rest) = line.strip_prefix("version = ") {
                return Some(strip_quotes(rest).to_string());
            }
        }
    }
    None
}

fn strip_quotes(s: &str) -> &str {
    s.trim().trim_start_matches('"').trim_end_matches('"')
}
