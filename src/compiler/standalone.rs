//! `warp pure <program.warp> [arguments…]` (card warp-pure, notes/standalone_compiler.md): lib/compiler/compiler.warp,
//! the warp compiler as one warp program, makes the program's module, which then runs. compiler.warp's own module is a
//! build output (decision compiler-build): compiler.wasm, built by the Rust pipeline, for `warp pure`; compiler-self.wasm,
//! compiler.warp compiled by itself through the pure lane, for `warp self`. Both lie next to the binary for the lib/ they
//! record, for any other lib/ in <lib>/../data/; a missing one is built at its first run, an existing one only by
//! `warp rebuild`, and a run on one built from another lib/ or warp build says so in one line.
use crate::compiler::self_hosted::{library, library_fingerprint, module_of_bytes, pure_bytes, WORKER_STACK};
use crate::node::{error, Node};
use crate::pipeline::CompiledModule;
use std::path::{Path, PathBuf};

const COMPILER_FILE: &str = "compiler/compiler.warp";
/// The custom section of a build file naming what it was built from: lib/, lib/'s fingerprint, the warp build
const BUILD_SECTION: &str = "warp.build";
const BUILD_SEPARATOR: &str = "\n";
/// Where a lib/ other than the binary's keeps its build files, beside lib/
const OTHER_LIBRARY_FOLDER: &str = "data";
/// What a new build compiles once to compute its kept values
const WARMING_PROGRAM: &str = "0";
/// Set to 1: the warnings and hints of compiling compiler.warp itself are shown
const COMPILER_WARNINGS: &str = "WARP_COMPILER_WARNINGS";

/// Who compiles compiler.warp: `warp pure` the Rust pipeline, `warp self` the pure lane, compiler.warp itself
#[derive(Clone, Copy)]
pub enum Compiler {
	RustCompiled,
	SelfCompiled,
}

impl Compiler {
	pub const ALL: [Compiler; 2] = [Compiler::RustCompiled, Compiler::SelfCompiled];

	/// The build file's name
	fn file_name(self) -> &'static str {
		match self {
			Compiler::RustCompiled => "compiler.wasm",
			Compiler::SelfCompiled => "compiler-self.wasm",
		}
	}

	/// The command that runs programs with it
	pub fn command(self) -> &'static str {
		match self {
			Compiler::RustCompiled => "pure",
			Compiler::SelfCompiled => "self",
		}
	}
}

/// The program's value, run from the module compiler.warp made of it, with these arguments (`use os; args`)
pub fn run(program: &Path, arguments: Vec<String>, by: Compiler) -> Node {
	ran(compiled(program, by), arguments)
}

/// The value of `code` (`warp pure "x=√72.;x^2"`, a test case), compiled as a program file of `folder` would be
pub fn run_code(code: &str, folder: &Path, by: Compiler) -> Node {
	ran(compiled_code(code, folder, by), Vec::new())
}

/// The module compiler.warp makes of `code`, compiled as a program file of `folder` would be
fn compiled_code(code: &str, folder: &Path, by: Compiler) -> Result<CompiledModule, Node> {
	let file = std::env::temp_dir().join(format!("warp-inline-{}.warp", std::process::id()));
	std::fs::write(&file, format!("{code}\n")).map_err(|failure| error(&format!("{}: {failure}", file.display())))?;
	let module = compiled_in(&file, folder, by);
	let _ = std::fs::remove_file(&file);
	module
}

/// The value of the compiled module run with these arguments, or why it was not compiled
fn ran(module: Result<CompiledModule, Node>, arguments: Vec<String>) -> Node {
	match module {
		Ok(module) => {
			crate::modules::std_adapters::set_program_arguments(arguments);
			crate::pipeline::run_module(module)
		}
		Err(failure) => failure,
	}
}

/// The module compiler.warp makes of the program file, its folder in scope
pub fn compiled(program: &Path, by: Compiler) -> Result<CompiledModule, Node> {
	let program = program.canonicalize().map_err(|failure| error(&format!("{}: {failure}", program.display())))?;
	let folder = program.parent().unwrap_or(Path::new("/")).to_path_buf();
	compiled_in(&program, &folder, by)
}

/// The module compiler.warp makes of the program file with `folder` in scope: `compiler.warp <source> <wasm> <folder> <lib>`
fn compiled_in(program: &Path, folder: &Path, by: Compiler) -> Result<CompiledModule, Node> {
	let output = std::env::temp_dir().join(format!("warp-pure-{}.wasm", std::process::id()));
	let _ = std::fs::remove_file(&output);
	let arguments = [program, output.as_path(), folder, library()];
	let compiler = compiler(by)?;
	crate::modules::std_adapters::set_program_arguments(
		arguments.iter().map(|path| path.display().to_string()).collect(),
	);
	let said = crate::pipeline::run_module(compiler);
	let bytes = std::fs::read(&output);
	let _ = std::fs::remove_file(&output);
	match (said.drop_meta(), bytes) {
		(Node::Error(_), _) => Err(said),
		(_, Ok(bytes)) => Ok(module_of_bytes(bytes)),
		(_, Err(_)) => Err(error(&format!("compiler.warp wrote no module, it gave {}", said.serialize()))),
	}
}

/// `warp rebuild`: the build file of each compiler built anew from the current lib/, its path, or why it failed
pub fn rebuild(by: Compiler) -> Result<PathBuf, Node> {
	let path = build_path(by);
	build(by, &path)?;
	Ok(path)
}

/// The compiler's build file there, built first when missing: before many runs start at once, so none waits on a build
pub fn prepare(by: Compiler) -> Result<(), Node> {
	compiler(by).map(|_| ())
}

/// compiler.warp's module from its build file, built first when missing; one line when it was built from another lib/
/// or warp build
fn compiler(by: Compiler) -> Result<CompiledModule, Node> {
	let path = build_path(by);
	let Ok(bytes) = std::fs::read(&path) else {
		eprintln!("building {} (later: warp rebuild)", path.display());
		return build(by, &path);
	};
	if built_from(&bytes) != current_build() {
		eprintln!("{} was built from another lib/ or warp build: `warp rebuild` builds it anew", path.display());
	}
	Ok(module_of_bytes(bytes))
}

/// compiler.warp compiled by `by`, written to `path` with the build section
fn build(by: Compiler, path: &Path) -> Result<CompiledModule, Node> {
	let file = library().join(COMPILER_FILE);
	let source = std::fs::read_to_string(&file).map_err(|failure| error(&format!("{}: {failure}", file.display())))?;
	let module = match by {
		Compiler::SelfCompiled => pure_compiled(&file, source).map_err(|refusal| {
			error(&format!("compiler.warp compiled by itself: the pure lane refuses it: {refusal}"))
		})?,
		Compiler::RustCompiled => {
			// a Node stays on its thread: the failure crosses as its text
			let compiled =
				on_big_stack(file, move || crate::pipeline::compile(&source).map_err(|failed| failed.serialize()));
			compiled.map_err(|failed| error(&failed))?
		}
	};
	let mut bytes = module.bytes.clone();
	let section = current_build().join(BUILD_SEPARATOR);
	wasm_encoder::Section::append_to(
		&wasm_encoder::CustomSection { name: BUILD_SECTION.into(), data: section.as_bytes().into() },
		&mut bytes,
	);
	let written = path.parent().map_or(Ok(()), std::fs::create_dir_all).and_then(|_| write_new(path, &bytes));
	written.map_err(|failure| error(&format!("{}: {failure}", path.display())))?;
	warm(by)?;
	Ok(module)
}

/// The new build run once on a small program: the values compiler.warp keeps for every run (`kept(…) ?? keep(…)`, its
/// lowered runtime words: ~50 s on the Mac) are computed by the build, not by the first program's run within its time
/// limit (card cloud-lane-narrow: the first `warp self` after `warp rebuild self` ran out of time on a cloud VM)
fn warm(by: Compiler) -> Result<(), Node> {
	compiled_code(WARMING_PROGRAM, library(), by).map(|_| ())
}

/// A new file moved into place: writing over a module another warp run reads would hand it half a module
fn write_new(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
	let new = path.with_extension(format!("new-{}", std::process::id()));
	std::fs::write(&new, bytes)?;
	std::fs::rename(&new, path)
}

/// What a build of now is made from: lib/, its fingerprint, the sources of this warp (WARP_SOURCE, build.rs)
fn current_build() -> Vec<String> {
	vec![library().display().to_string(), library_fingerprint(), env!("WARP_SOURCE").to_string()]
}

/// What a build file was made from, as current_build says it
fn built_from(bytes: &[u8]) -> Vec<String> {
	crate::host::host_parts::section_names(bytes, BUILD_SECTION, BUILD_SEPARATOR).unwrap_or_default()
}

/// The build file: next to the binary when that one is for this lib/ (or none is there yet), else beside lib/
fn build_path(by: Compiler) -> PathBuf {
	let beside_binary = std::env::current_exe()
		.ok()
		.and_then(|binary| binary.canonicalize().ok())
		.and_then(|binary| binary.parent().map(|folder| folder.join(by.file_name())));
	let library_path = library().display().to_string();
	let for_this_library = |path: &PathBuf| match std::fs::read(path) {
		Ok(bytes) => built_from(&bytes).first() == Some(&library_path),
		Err(_) => true,
	};
	match beside_binary.filter(for_this_library) {
		Some(path) => path,
		None => library().parent().unwrap_or(library()).join(OTHER_LIBRARY_FOLDER).join(by.file_name()),
	}
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

/// `compile` as the program of `file`, in a thread with a stack deep enough for a compile of the compiler; quietly: the
/// compiler's own warnings and hints are no news to the user of `warp pure` (card prefix-count-warning), but they are
/// to whoever works on lib/compiler: WARP_COMPILER_WARNINGS=1 shows them (the Integrator's pure-chain check)
fn on_big_stack<T: Send + 'static>(file: PathBuf, compile: impl FnOnce() -> T + Send + 'static) -> T {
	let work = move || {
		let compile = || crate::modules::with_program_file(&file, compile);
		if std::env::var(COMPILER_WARNINGS).is_ok_and(|shown| shown == "1") {
			compile()
		} else {
			crate::errors::diagnostic::quietly(compile)
		}
	};
	std::thread::Builder::new()
		.stack_size(WORKER_STACK)
		.spawn(work)
		.expect("a compiler thread")
		.join()
		.expect("the compile")
}
