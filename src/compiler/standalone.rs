//! `warp pure <program.warp> [arguments…]` (card warp-pure, notes/standalone_compiler.md): lib/compiler/compiler.warp,
//! the warp compiler as one warp program, makes the program's module, which then runs. Rust only keeps the compiler: it
//! is compiled once and kept in the module cache (~/.cache/warp/modules) under the key of the binary and lib/, by the
//! Rust pipeline, or by the pure lane with WARP_PURE_COMPILER=self.
use crate::compiler::self_hosted::{library, module_of_bytes, pure_bytes, stage_key, WORKER_STACK};
use crate::node::{error, Node};
use crate::pipeline::CompiledModule;
use crate::run::program_cache::{keep_under, kept_under};
use std::path::{Path, PathBuf};

const COMPILER_FILE: &str = "compiler/compiler.warp";
/// The kept compiler's cache names: compiled by the pure lane or by Rust
const PURE_COMPILER: &str = "warp pure: compiler.warp, pure-compiled";
const RUST_COMPILER: &str = "warp pure: compiler.warp, Rust-compiled";
/// `WARP_PURE_COMPILER=self`: the compiler compiled by the pure lane, itself
const COMPILER_SWITCH: &str = "WARP_PURE_COMPILER";
const SELF_COMPILED: &str = "self";

/// The program's value, run from the module compiler.warp made of it, with these arguments (`use os; args`)
pub fn run(program: &Path, arguments: Vec<String>) -> Node {
	match compiled(program) {
		Ok(module) => {
			crate::modules::std_adapters::set_program_arguments(arguments);
			crate::pipeline::run_module(module)
		}
		Err(failure) => failure,
	}
}

/// The module compiler.warp makes of the program file: `compiler.warp <source> <wasm> <folder> <lib>`
pub fn compiled(program: &Path) -> Result<CompiledModule, Node> {
	let program = program.canonicalize().map_err(|failure| error(&format!("{}: {failure}", program.display())))?;
	let folder = program.parent().unwrap_or(Path::new("/")).to_path_buf();
	let output = std::env::temp_dir().join(format!("warp-pure-{}.wasm", std::process::id()));
	let _ = std::fs::remove_file(&output);
	let arguments = [program.as_path(), output.as_path(), folder.as_path(), library()];
	crate::modules::std_adapters::set_program_arguments(
		arguments.iter().map(|path| path.display().to_string()).collect(),
	);
	let compiler = compiler()?;
	let said = crate::pipeline::run_module(compiler);
	let bytes = std::fs::read(&output);
	let _ = std::fs::remove_file(&output);
	match (said.drop_meta(), bytes) {
		(Node::Error(_), _) => Err(said),
		(_, Ok(bytes)) => Ok(module_of_bytes(bytes)),
		(_, Err(_)) => Err(error(&format!("compiler.warp wrote no module, it gave {}", said.serialize()))),
	}
}

/// compiler.warp's module, kept: the Rust pipeline's, or the pure lane's with WARP_PURE_COMPILER=self, which stays a
/// choice until it behaves as the Rust-compiled one on all tests (user 2026-10-21, card self-compiled-parity)
fn compiler() -> Result<CompiledModule, Node> {
	let file = library().join(COMPILER_FILE);
	let source = std::fs::read_to_string(&file).map_err(|failure| error(&format!("{}: {failure}", file.display())))?;
	let self_compiled = crate::setting(COMPILER_SWITCH).as_deref() == Some(SELF_COMPILED);
	let key = stage_key(if self_compiled { PURE_COMPILER } else { RUST_COMPILER });
	if let Some(module) = key.as_deref().and_then(kept_under) {
		return Ok(module);
	}
	let module = if self_compiled {
		pure_compiled(&file, source).map_err(|refusal| error(&format!("{PURE_COMPILER}: the pure lane refuses it: {refusal}")))?
	} else {
		// a Node stays on its thread: the failure crosses as its text
		let compiled = on_big_stack(file, move || crate::pipeline::compile(&source).map_err(|failed| failed.serialize()));
		compiled.map_err(|failed| error(&failed))?
	};
	if let Some(key) = key {
		keep_under(&key, &module);
	}
	Ok(module)
}

/// compiler.warp compiled by the pure lane (parser.warp, the warp passes, analyzer.warp, emitter.warp), else the refusal
fn pure_compiled(file: &Path, source: String) -> Result<CompiledModule, String> {
	on_big_stack(file.to_path_buf(), move || match pure_bytes(&source) {
		Ok(Ok(bytes)) => Ok(bytes),
		Ok(Err(diagnosed)) => Err(diagnosed.serialize()),
		Err(refusal) => Err(refusal),
	})
	.map(module_of_bytes)
}

/// `compile` as the program of `file`, in a thread with a stack deep enough for a compile of the compiler
fn on_big_stack<T: Send + 'static>(file: PathBuf, compile: impl FnOnce() -> T + Send + 'static) -> T {
	let work = move || crate::modules::with_program_file(&file, compile);
	std::thread::Builder::new()
		.stack_size(WORKER_STACK)
		.spawn(work)
		.expect("a compiler thread")
		.join()
		.expect("the compile")
}
