//! Compiler-less warp runtime: wasmtime without Cranelift, slim host imports, hand-rolled WASI `fd_write`.
//! Used as the prebuilt stub for `warp build --exe` (notes/aot.md): the .cwasm is appended (magic + length at EOF),
//! or loaded from `WARP_CWASM` while developing / proving the probe path (probes/aot/standalone.sh).
use std::env;
use std::fs;
use std::io::{self, Write};
use std::process;
use std::time::{SystemTime, UNIX_EPOCH};
use wasmtime::{Caller, Config, Engine, Extern, Linker, Module, Store, Val};

const FUEL: u64 = 10_000_000_000;
const GC_HEAP_INITIAL_BYTES: u64 = 1 << 30;
/// Trailer: `[.cwasm bytes][u64 little-endian length][magic]`
const TRAILER_MAGIC: &[u8; 8] = b"WRPCwasm";
const UNSUPPORTED: &str = "warp-runtime: this program needs a host word the standalone runtime does not provide";

fn main() {
	if let Err(failure) = run() {
		eprintln!("{failure}");
		process::exit(1);
	}
}

fn run() -> Result<(), String> {
	let machine_code = load_machine_code()?;
	let engine = engine()?;
	// SAFETY: the machine code is warp's own `compile --aot` / `build --exe` output for this wasmtime version
	let module = unsafe { Module::deserialize(&engine, &machine_code) }
		.map_err(|e| format!("warp-runtime: could not load machine code: {e}"))?;
	let mut store = Store::new(&engine, ());
	store
		.set_fuel(FUEL)
		.map_err(|e| format!("warp-runtime: fuel: {e}"))?;
	let mut linker = Linker::new(&engine);
	link_imports(&mut linker, &module)?;
	let instance = linker
		.instantiate(&mut store, &module)
		.map_err(|e| format!("warp-runtime: instantiate: {e}"))?;
	let main = instance
		.get_func(&mut store, "main")
		.ok_or_else(|| "warp-runtime: module exports no main".to_string())?;
	let mut results = vec![Val::AnyRef(None); main.ty(&store).results().len()];
	main
		.call(&mut store, &[], &mut results)
		.map_err(|e| format!("warp-runtime: main failed: {e:#}"))?;
	println!("{}", format_result(results.first(), &mut store, &instance));
	Ok(())
}

/// Engine settings must match `util::deterministic_config` + fuel (notes/aot.md); otherwise deserialize fails.
fn engine() -> Result<Engine, String> {
	// Match util::deterministic_config + fuel. This crate builds wasmtime without the component-model
	// feature (and without Cranelift): those settings are already off, and the compile-side engine
	// turns them off explicitly so .cwasm files load here (notes/aot.md).
	let mut config = Config::new();
	config.wasm_gc(true);
	config.wasm_function_references(true);
	config.gc_heap_initial_size(GC_HEAP_INITIAL_BYTES);
	config.consume_fuel(true);
	config.concurrency_support(false);
	Engine::new(&config).map_err(|e| format!("warp-runtime: engine: {e}"))
}

fn load_machine_code() -> Result<Vec<u8>, String> {
	if let Some(bytes) = read_appended_cwasm()? {
		return Ok(bytes);
	}
	if let Ok(path) = env::var("WARP_CWASM") {
		return fs::read(&path).map_err(|e| format!("warp-runtime: WARP_CWASM {path}: {e}"));
	}
	Err("warp-runtime: no appended .cwasm and WARP_CWASM is unset".into())
}

/// `[stub][cwasm][u64 le length][WRPCwasm]` — length is only the .cwasm bytes.
fn read_appended_cwasm() -> Result<Option<Vec<u8>>, String> {
	let path = env::current_exe().map_err(|e| format!("warp-runtime: current_exe: {e}"))?;
	let bytes = fs::read(&path).map_err(|e| format!("warp-runtime: read {}: {e}", path.display()))?;
	if bytes.len() < 8 + 8 {
		return Ok(None);
	}
	if &bytes[bytes.len() - 8..] != TRAILER_MAGIC {
		return Ok(None);
	}
	let len_bytes: [u8; 8] = bytes[bytes.len() - 16..bytes.len() - 8]
		.try_into()
		.map_err(|_| "warp-runtime: trailer length".to_string())?;
	let cwasm_len = u64::from_le_bytes(len_bytes) as usize;
	let start = bytes
		.len()
		.checked_sub(16 + cwasm_len)
		.ok_or_else(|| "warp-runtime: trailer length past start of file".to_string())?;
	Ok(Some(bytes[start..start + cwasm_len].to_vec()))
}

fn link_imports(linker: &mut Linker<()>, module: &Module) -> Result<(), String> {
	for import in module.imports() {
		let module_name = import.module().to_string();
		let name = import.name().to_string();
		match (module_name.as_str(), name.as_str()) {
			("wasi_snapshot_preview1", "fd_write") => link_fd_write(linker)?,
			("host", "fetch") | ("host", "fetch_within") | ("host", "read") => link_host_text_pair(linker, &module_name, &name)?,
			("host", "warn") => {
				linker
					.func_wrap("host", "warn", |_caller: Caller<'_, ()>, _ptr: i32, _len: i32| {})
					.map_err(|e| e.to_string())?;
			}
			("host", "sleep") => {
				linker
					.func_wrap("host", "sleep", |milliseconds: i64| {
						std::thread::sleep(std::time::Duration::from_millis(milliseconds.max(0) as u64))
					})
					.map_err(|e| e.to_string())?;
			}
			("host", "random") => {
				linker
					.func_wrap("host", "random", || (next_random() >> 11) as f64 / (1u64 << 53) as f64)
					.map_err(|e| e.to_string())?;
			}
			("host", "random_below") => {
				linker
					.func_wrap("host", "random_below", |bound: i64| {
						if bound <= 0 {
							0
						} else {
							(next_random() % bound as u64) as i64
						}
					})
					.map_err(|e| e.to_string())?;
			}
			("host", "clock") => {
				linker
					.func_wrap("host", "clock", || {
						SystemTime::now()
							.duration_since(UNIX_EPOCH)
							.map(|d| d.as_millis() as i64)
							.unwrap_or(0)
					})
					.map_err(|e| e.to_string())?;
			}
			("host", "run") | ("host", "run_block")
			| ("host", "task_spawn")
			| ("host", "task_await")
			| ("host", "task_control")
			| ("host", "task_spawn_values")
			| ("host", "task_await_value")
			| ("host", "task_join")
			| ("host", "task_failure")
			| ("host", "task_status")
			| ("host", "task_poll")
			| ("host", "task·check") => link_unsupported(linker, &module_name, &name, import.ty())?,
			(other_module, other_name) => {
				return Err(format!(
					"warp-runtime: unsupported import {other_module}::{other_name} (standalone runtime has slim host + fd_write only)"
				));
			}
		}
	}
	Ok(())
}

fn link_unsupported(
	linker: &mut Linker<()>,
	module: &str,
	name: &str,
	ty: wasmtime::ExternType,
) -> Result<(), String> {
	let message = format!("{UNSUPPORTED} ({module}::{name})");
	match ty {
		wasmtime::ExternType::Func(func_ty) => {
			// Define a matching trap for whatever signature the module imported
			linker
				.func_new(module, name, func_ty, move |_caller, _params, _results| {
					Err(wasmtime::Error::msg(message.clone()))
				})
				.map_err(|e| e.to_string())?;
			// message is returned when the program calls this import
			Ok(())
		}
		_ => Err(format!("warp-runtime: unsupported non-func import {module}::{name}")),
	}
}

/// host.fetch / fetch_within / read: (ptr,len[,timeout]) -> (ptr,len); negative length = failure text in memory.
fn link_host_text_pair(linker: &mut Linker<()>, module: &str, name: &str) -> Result<(), String> {
	let failure = |msg: &'static str| -> (i32, i32) {
		let _ = msg;
		(0, -1)
	};
	match name {
		"fetch_within" => {
			linker
				.func_wrap(module, name, move |_caller: Caller<'_, ()>, _p: i32, _l: i32, _t: i64| -> (i32, i32) {
					failure("fetch_within unavailable in warp-runtime")
				})
				.map_err(|e| e.to_string())?;
		}
		_ => {
			linker
				.func_wrap(module, name, move |_caller: Caller<'_, ()>, _p: i32, _l: i32| -> (i32, i32) {
					failure("host text import unavailable in warp-runtime")
				})
				.map_err(|e| e.to_string())?;
		}
	}
	Ok(())
}

/// wasi_snapshot_preview1::fd_write(fd, iovs, iovs_len, nwritten) -> errno
fn link_fd_write(linker: &mut Linker<()>) -> Result<(), String> {
	linker
		.func_wrap(
			"wasi_snapshot_preview1",
			"fd_write",
			|mut caller: Caller<'_, ()>, fd: i32, iovs: i32, iovs_len: i32, nwritten: i32| -> i32 {
				if fd != 1 && fd != 2 {
					return 8; // ERRNO_BADF
				}
				let Some(Extern::Memory(memory)) = caller.get_export("memory") else {
					return 29; // ERRNO_IO
				};
				let mut written: u32 = 0;
				for i in 0..iovs_len {
					let base = (iovs + i * 8) as usize;
					let mut ptr_buf = [0u8; 4];
					let mut len_buf = [0u8; 4];
					if memory.read(&caller, base, &mut ptr_buf).is_err()
						|| memory.read(&caller, base + 4, &mut len_buf).is_err()
					{
						return 29;
					}
					let ptr = u32::from_le_bytes(ptr_buf) as usize;
					let len = u32::from_le_bytes(len_buf) as usize;
					let mut chunk = vec![0u8; len];
					if memory.read(&caller, ptr, &mut chunk).is_err() {
						return 29;
					}
					let out: &mut dyn Write = if fd == 1 {
						&mut io::stdout()
					} else {
						&mut io::stderr()
					};
					if out.write_all(&chunk).is_err() {
						return 29;
					}
					written = written.saturating_add(len as u32);
				}
				let _ = memory.write(&mut caller, nwritten as usize, &written.to_le_bytes());
				0
			},
		)
		.map_err(|e| e.to_string())?;
	Ok(())
}

/// Slim read of warp's 3-field `$Node` (kind, data, value) without the compiler crate (notes/aot.md).
const KIND_MASK: i64 = 0xFF;
const KIND_INT: u8 = 1;
const KIND_FLOAT: u8 = 2;
const KIND_TEXT: u8 = 3;
const KIND_SYMBOL: u8 = 5;

fn format_result(value: Option<&Val>, store: &mut Store<()>, instance: &wasmtime::Instance) -> String {
	match value {
		Some(Val::I64(n)) => n.to_string(),
		Some(Val::I32(n)) => n.to_string(),
		Some(Val::F64(n)) => format!("{n}"),
		Some(Val::F32(n)) => format!("{n}"),
		Some(Val::AnyRef(Some(_))) => format_node(value.unwrap(), store, instance),
		Some(Val::AnyRef(None)) | None => "<none>".to_string(),
		Some(other) => format!("<{other:?}>"),
	}
}

fn format_node(value: &Val, store: &mut Store<()>, instance: &wasmtime::Instance) -> String {
	let Some(structref) = value.unwrap_anyref().and_then(|r| r.unwrap_struct(&*store).ok()) else {
		return "<node>".into();
	};
	let Ok(kind_val) = structref.field(&mut *store, 0) else { return "<node>".into() };
	let Ok(data) = structref.field(&mut *store, 1) else { return "<node>".into() };
	let tag = (kind_val.unwrap_i64() & KIND_MASK) as u8;
	match tag {
		KIND_INT => match read_int_payload(store, &data) {
			Some(n) => n.to_string(),
			None => "<int?>".into(),
		},
		KIND_FLOAT => match read_f64_payload(store, &data) {
			Some(n) => format!("{n}"),
			None => "<float?>".into(),
		},
		KIND_TEXT | KIND_SYMBOL => match (instance.get_memory(&mut *store, "memory"), read_text(store, &data)) {
			(Some(memory), Some((ptr, len))) => {
				let mut bytes = vec![0u8; len];
				if memory.read(&*store, ptr, &mut bytes).is_ok() {
					String::from_utf8_lossy(&bytes).into_owned()
				} else {
					"<text?>".into()
				}
			}
			_ => "<text?>".into(),
		},
		_ => "<node>".into(),
	}
}

fn read_int_payload(store: &mut Store<()>, data: &Val) -> Option<i64> {
	let anyref = data.unwrap_anyref()?;
	let payload = anyref.unwrap_struct(&*store).ok()?;
	match payload.field(&mut *store, 0).ok()? {
		Val::I64(value) => Some(value),
		_ => None, // BigInt / Ratio: print as <node> via caller
	}
}

fn read_f64_payload(store: &mut Store<()>, data: &Val) -> Option<f64> {
	let anyref = data.unwrap_anyref()?;
	let payload = anyref.unwrap_struct(&*store).ok()?;
	match payload.field(&mut *store, 0).ok()? {
		Val::F64(bits) => Some(f64::from_bits(bits)),
		other => {
			// wasmtime 49 may expose F64 as a typed value already
			let _ = other;
			None
		}
	}
}

fn read_text(store: &mut Store<()>, data: &Val) -> Option<(usize, usize)> {
	let anyref = data.unwrap_anyref()?;
	let string = anyref.unwrap_struct(&*store).ok()?;
	let ptr = string.field(&mut *store, 0).ok()?.unwrap_i32() as usize;
	let len = string.field(&mut *store, 1).ok()?.unwrap_i32() as usize;
	Some((ptr, len))
}

fn next_random() -> u64 {
	// xorshift-ish from the clock; enough for programs that call random once
	let mut x = SystemTime::now()
		.duration_since(UNIX_EPOCH)
		.map(|d| d.as_nanos() as u64)
		.unwrap_or(0x9E3779B97F4A7C15);
	x ^= x << 13;
	x ^= x >> 7;
	x ^= x << 17;
	x
}


