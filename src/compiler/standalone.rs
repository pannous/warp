//! `warp pure <program.warp> [arguments…]` (card warp-pure, notes/standalone_compiler.md): lib/compiler/compiler.warp,
//! the warp compiler as one warp program, makes the program's module, which then runs. compiler.warp's own module is a
//! build output (decision compiler-build): compiler.wasm, built by the Rust pipeline, for `warp pure`; compiler-self.wasm,
//! compiler.warp compiled by itself through the pure lane, for `warp self`. The pair next to the binary is the delivered
//! one (a release's, or main's, stamped with the commit it was built from) and is used as it is, whatever lib/ beside
//! it says (card installed-compilers); a lib/ named by WARP_LIB that the pair is not for keeps its own in
//! <lib>/../data/, where a run on one built from another lib/ or warp build says so in one line. Only `warp rebuild`
//! builds one: a missing one is an error naming it.
use crate::compiler::self_hosted::{library, library_fingerprint, module_of_bytes, pure_bytes, WORKER_STACK};
use crate::node::{error, Node};
use crate::pipeline::CompiledModule;
use std::path::{Path, PathBuf};

/// compiler.warp's command compiling one REPL line (lib/compiler/compiler.warp repl_step)
const REPL_STEP: &str = "repl-step";

const COMPILER_FILE: &str = "compiler/compiler.warp";
/// The custom section of a build file naming what it was built from: lib/, lib/'s fingerprint, the warp build
const BUILD_SECTION: &str = "warp.build";
const BUILD_SEPARATOR: &str = "\n";
/// The build section's parts judged for staleness (judged_build's), the commit after them
const BUILD_PARTS_JUDGED: usize = 3;
/// `warp rebuild [pure|self]`, the one command that builds a compiler
const REBUILD_WORD: &str = "rebuild";
const SHOWN_COMMIT_LENGTH: usize = 10;
/// Where a lib/ other than the binary's keeps its build files, beside lib/
const OTHER_LIBRARY_FOLDER: &str = "data";
/// Names a lib/ of its own (a worktree's): only then can the compilers be other than those next to the binary
const LIBRARY_VARIABLE: &str = "WARP_LIB";
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
	let module = compiled_in(&file, folder, by).map_err(|failure| inline_positions(&failure, &file));
	let _ = std::fs::remove_file(&file);
	module
}

/// `warp pure <command> …` (card wasm-commands): compiler.warp's own command (lib/compiler/commands.warp), the command
/// line its arguments
pub fn command(arguments: Vec<String>, by: Compiler) -> Node {
	ran(compiler(by), arguments)
}

/// One line of `warp pure repl`: compiler.warp's repl-step compiles it after the lines kept in `state` and decides which
/// stay; its module runs here, where plain WASI could not run it
pub fn repl_line(line: &str, state: &Path, by: Compiler) -> Node {
	let module = state.with_extension("wasm");
	let step = [REPL_STEP, &state.display().to_string(), line, &module.display().to_string()].map(String::from);
	let stepped = command(step.to_vec(), by);
	if let Node::Error(_) = stepped.drop_meta() {
		return stepped;
	}
	match std::fs::read(&module) {
		Ok(bytes) => crate::pipeline::run_module(module_of_bytes(bytes)),
		Err(failure) => error(&format!("repl-step wrote no module {}: {failure}", module.display())),
	}
}

/// A failure of inline code: a position in the file it was written to is one in the code (`at 1:5`)
fn inline_positions(failure: &Node, file: &Path) -> Node {
	match failure.drop_meta() {
		Node::Error(message) => match message.drop_meta() {
			Node::Text(text) => error(&text.replace(&format!("{}:", file.display()), "")),
			_ => failure.clone(),
		},
		_ => failure.clone(),
	}
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
	let owner = crate::extensions::sha256_hex(&compiler.bytes);
	let said = crate::modules::std_adapters::kept_by(owner, || crate::pipeline::run_module(compiler));
	let bytes = std::fs::read(&output);
	let _ = std::fs::remove_file(&output);
	match (said.drop_meta(), bytes) {
		// a position the compiler's run names is one in compiler.warp (its `raise`), not in the program
		(Node::Error(_), _) => Err(crate::errors::diagnostic::located_in_file(&said, &library().join(COMPILER_FILE))),
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

/// The compiler's build file there, or the error naming `warp rebuild`: before many runs start at once
pub fn prepare(by: Compiler) -> Result<(), Node> {
	compiler(by).map(|_| ())
}

/// compiler.warp's module from its build file; a missing one is an error naming `warp rebuild`, never built here (card
/// installed-compilers). One line when a worktree's build in data/ was built from another lib/ or warp build; the
/// delivered pair next to the binary is used as it is
fn compiler(by: Compiler) -> Result<CompiledModule, Node> {
	let path = build_path(by);
	let Ok(bytes) = std::fs::read(&path) else {
		let rebuild = format!("warp {REBUILD_WORD} {}", by.command());
		return Err(error(&format!("no {} at {}: `{rebuild}` builds it", by.file_name(), path.display())));
	};
	if !is_delivered(&path) && built_from(&bytes)[..BUILD_PARTS_JUDGED] != judged_build()[..] {
		eprintln!("{} was built from another lib/ or warp build: `warp rebuild` builds it anew", path.display());
	}
	Ok(module_of_bytes(bytes))
}

/// Where the delivered compilers lie: next to the binary, or in an installed warp's share/warp
fn binary_folder() -> Option<PathBuf> {
	crate::modules::installed_folder()
}

/// A build file of the installed folder: the delivered compiler
fn is_delivered(path: &Path) -> bool {
	binary_folder().is_some_and(|folder| path.parent() == Some(folder.as_path()))
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
fn judged_build() -> Vec<String> {
	vec![library().display().to_string(), library_fingerprint(), env!("WARP_SOURCE").to_string()]
}

/// The build section's parts: judged_build's, then the commit of lib/'s checkout (the stamp of a delivered pair; not
/// judged: a commit of notes alone changes it)
fn current_build() -> Vec<String> {
	let mut parts = judged_build();
	parts.push(library_commit());
	parts
}

/// The commit lib/'s git checkout is at, "" outside one (an installed lib/)
fn library_commit() -> String {
	let said = std::process::Command::new("git").arg("-C").arg(library()).args(["rev-parse", "HEAD"]).output();
	said.ok()
		.filter(|said| said.status.success())
		.map(|said| String::from_utf8_lossy(&said.stdout).trim().to_string())
		.unwrap_or_default()
}

/// `warp version`'s line of the compilers in use: `compiler.wasm of <commit>, compiler-self.wasm missing`
pub fn compilers_line() -> String {
	let stamp = |by: Compiler| {
		let path = build_path(by);
		match std::fs::read(&path) {
			Ok(bytes) => match &built_from(&bytes)[BUILD_PARTS_JUDGED] {
				commit if commit.is_empty() => format!("{} of no commit", by.file_name()),
				commit => format!("{} of {}", by.file_name(), &commit[..commit.len().min(SHOWN_COMMIT_LENGTH)]),
			},
			Err(_) => format!("{} missing (warp {REBUILD_WORD} {})", by.file_name(), by.command()),
		}
	};
	Compiler::ALL.map(stamp).join(", ")
}

/// What a build file was made from, as current_build says it, its parts all there
fn built_from(bytes: &[u8]) -> Vec<String> {
	let mut parts = crate::host::host_parts::section_names(bytes, BUILD_SECTION, BUILD_SEPARATOR).unwrap_or_default();
	parts.resize(BUILD_PARTS_JUDGED + 1, String::new());
	parts
}

/// The build file: next to the binary, unless WARP_LIB names a lib/ the pair there is not for, whose build files lie
/// beside it. A missing one is judged by its siblings there (none: yes), so a worktree's `warp self` never puts its
/// compiler-self.wasm next to main's compiler.wasm (card build-path-missing)
fn build_path(by: Compiler) -> PathBuf {
	let binary_folder = binary_folder();
	if std::env::var_os(LIBRARY_VARIABLE).is_none() {
		if let Some(folder) = &binary_folder {
			return folder.join(by.file_name());
		}
	}
	let library_path = library().display().to_string();
	let for_this_library = |folder: &PathBuf| {
		let read = |file: Compiler| std::fs::read(folder.join(file.file_name())).ok();
		let judged: Vec<Vec<u8>> = match read(by) {
			Some(bytes) => vec![bytes],
			None => Compiler::ALL.into_iter().filter_map(read).collect(),
		};
		judged.iter().all(|bytes| built_from(bytes).first() == Some(&library_path))
	};
	match binary_folder.filter(for_this_library) {
		Some(folder) => folder.join(by.file_name()),
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
