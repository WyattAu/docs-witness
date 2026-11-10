//! The execution half: fresh-instance WASI runner and witness payload.

use std::time::Instant;

use wasmtime::{Config, Engine, Linker, Module, Store, Trap};
use wasmtime_wasi::p2::pipe::MemoryOutputPipe;
use wasmtime_wasi::preview1::{self, WasiP1Ctx};
use wasmtime_wasi::{I32Exit, WasiCtxBuilder};

use crate::error::WitnessError;

/// The default fuel budget (wasmtime fuel units; most instructions cost 1).
pub const DEFAULT_FUEL: u64 = 20_000_000;

/// The default stdout pipe capacity. A guest writing beyond it traps (the
/// pipe errors rather than blocking), so the runner cannot hang.
pub const DEFAULT_STDOUT_CAPACITY: usize = 4 * 1024 * 1024;

/// Execution budget for one witness run.
///
/// There is deliberately no wall-clock timeout: the v0.1 WASI surface has
/// no filesystem, no sockets, and no blocking calls, so compute is bounded
/// by fuel and I/O by the pipe capacity. A wall timeout via epoch
/// interruption lands with that surface's extension, not silently absent.
#[derive(Debug, Clone)]
pub struct WitnessConfig {
    /// Fuel budget for `_start`.
    pub fuel: u64,
    /// Pipe capacity for stdout in bytes. A guest writing beyond it sees
    /// a write error (the pipe rejects; the host never blocks).
    pub stdout_capacity: usize,
}

impl Default for WitnessConfig {
    fn default() -> Self {
        Self {
            fuel: DEFAULT_FUEL,
            stdout_capacity: DEFAULT_STDOUT_CAPACITY,
        }
    }
}

/// The recorded result of one execution: everything REQ-W2 embeds into the
/// page as static payload.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Witness {
    /// Captured guest stdout, lossless bytes.
    pub stdout: Vec<u8>,
    /// WASI exit code (0 when `_start` returned normally).
    pub exit: u32,
    /// Fuel consumed by the run.
    pub fuel_consumed: u64,
    /// Wall-clock duration in nanoseconds (reporting only; not part of
    /// the determinism contract).
    pub wall_ns: u64,
    /// This crate's version.
    pub witness_version: String,
    /// The wasmtime engine version the run executed under.
    pub wasmtime_version: String,
}

/// Run `wasm` (a `wasm32-wasip1` module) exactly once inside a fresh
/// engine, store, and WASI context: no filesystem, no sockets, no clock
/// injection beyond the guest's own reads. Fuel-bounded, stdout-captured.
///
/// Guest `proc_exit(n)` and a normal `_start` return are exits, not errors;
/// traps and fuel exhaustion are [`WitnessError`] variants.
pub fn run(wasm: &[u8], cfg: &WitnessConfig) -> Result<Witness, WitnessError> {
    let started = Instant::now();

    let mut engine_config = Config::new();
    engine_config.consume_fuel(true);
    let engine = Engine::new(&engine_config).map_err(WitnessError::Compile)?;
    let module = Module::new(&engine, wasm).map_err(WitnessError::Compile)?;

    let stdout_pipe = MemoryOutputPipe::new(cfg.stdout_capacity);
    let ctx = WasiCtxBuilder::new().stdout(stdout_pipe.clone()).build_p1();
    let mut store = Store::new(&engine, ctx);
    store
        .set_fuel(cfg.fuel)
        .map_err(|e| WitnessError::Fuel(e.to_string()))?;

    let mut linker: Linker<WasiP1Ctx> = Linker::new(&engine);
    preview1::add_to_linker_sync(&mut linker, |t: &mut WasiP1Ctx| t).map_err(WitnessError::Link)?;

    let instance = linker
        .instantiate(&mut store, &module)
        .map_err(WitnessError::Link)?;
    let start = instance
        .get_typed_func::<(), ()>(&mut store, "_start")
        .map_err(|_| WitnessError::NoEntrypoint)?;

    let outcome = start.call(&mut store, ());
    let fuel_remaining = store
        .get_fuel()
        .map_err(|e| WitnessError::Fuel(e.to_string()))?;

    let exit = match outcome {
        Ok(()) => 0u32,
        Err(e) => {
            if let Some(code) = e.downcast_ref::<I32Exit>() {
                // WASI exit codes are u32; the bit-preserving cast keeps
                // guests that passed a large u32 intact.
                code.0 as u32
            } else if let Some(trap) = e.downcast_ref::<Trap>() {
                if *trap == Trap::OutOfFuel {
                    return Err(WitnessError::OutOfFuel {
                        limit: cfg.fuel,
                        consumed: cfg.fuel.saturating_sub(fuel_remaining),
                    });
                }
                return Err(WitnessError::Trap {
                    code: format!("{trap:?}"),
                    detail: e.to_string(),
                });
            } else {
                return Err(WitnessError::Link(e));
            }
        }
    };

    Ok(Witness {
        stdout: stdout_pipe.contents().to_vec(),
        exit,
        fuel_consumed: cfg.fuel.saturating_sub(fuel_remaining),
        wall_ns: u64::try_from(started.elapsed().as_nanos()).unwrap_or(u64::MAX),
        witness_version: crate::VERSION.to_string(),
        wasmtime_version: env!("DOCS_WITNESS_WASMTIME_VERSION").to_string(),
    })
}
