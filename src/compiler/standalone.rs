//! `warp pure <program.warp> [arguments…]` (card warp-pure, notes/standalone_compiler.md): lib/compiler/compiler.warp,
//! the warp compiler as one warp program, makes the program's module, which then runs. Rust only keeps the compiler: it
//! is compiled once and kept in the module cache (~/.cache/warp/modules) under the key of the binary and lib/, by the
//! pure lane; while that lane refuses it, by the Rust pipeline, said on stderr at every run.
use crate::compiler::self_hosted::{library, module_of_bytes, pure_bytes, stage_key, WORKER_STACK};
use crate::node::{error, Node};
use crate::pipeline::CompiledModule;
use crate::run::program_cache::{keep_under, kept_under};
use std::path::{Path, PathBuf};

const COMPILER_FILE: &str = "compiler/compiler.warp";
/// The kept compiler's cache names: compiled by the pure lane, or by Rust while the pure lane refuses compiler.warp
const PURE_COMPILER: &str = "warp pure: compiler.warp, pure-compiled";
const RUST_COMPILER: &str = "warp pure: compiler.warp, Rust-compiled";

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

/// compiler.warp's module, kept; the pure lane's when it compiles compiler.warp, else Rust's, said on stderr
fn compiler() -> Result<CompiledModule, Node> {
	let file = library().join(COMPILER_FILE);
	let source = std::fs::read_to_string(&file).map_err(|failure| error(&format!("{}: {failure}", file.display())))?;
	let (pure_key, rust_key) = (stage_key(PURE_COMPILER), stage_key(RUST_COMPILER));
	if let Some(module) = pure_key.as_deref().and_then(kept_under) {
		return Ok(module);
	}
	if let Some(module) = rust_key.as_deref().and_then(kept_under) {
		eprintln!("{RUST_COMPILER} (the pure lane refuses it; its first compile said why)");
		return Ok(module);
	}
	let (module, key) = match pure_compiled(&file, source.clone()) {
		Ok(module) => (module, pure_key),
		Err(refusal) => {
			eprintln!("{RUST_COMPILER}: the pure lane refuses it: {refusal}");
			// a Node stays on its thread: the failure crosses as its text
			let compiled =
				on_big_stack(file, move || crate::pipeline::compile(&source).map_err(|failed| failed.serialize()));
			(compiled.map_err(|failed| error(&failed))?, rust_key)
		}
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
