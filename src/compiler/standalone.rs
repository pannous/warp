//! `warp pure <program.warp> [arguments…]` (card warp-pure, notes/standalone_compiler.md): core/compiler.warp,
//! the warp compiler as one warp program, makes the program's module, which then runs. compiler.warp's own module is a
//! build output (decision compiler-build): compiler.wasm, compiler.warp compiled by the seed (seed/compiler-self.wasm, a
//! fixpoint compiler checked in; card rust-bootstrap), for `warp pure`; compiler-self.wasm, compiler.warp compiled by
//! compiler.wasm, that is by itself, for `warp self`. No Rust-compiled stage takes part in either build. The pair next to the binary is the delivered
//! one (a release's, or main's, stamped with the commit it was built from) and is used as it is, whatever lib/ beside
//! it says (card installed-compilers); a lib/ named by WARP_LIB that the pair is not for keeps its own in
//! <lib>/../data/, where a run on one built from another lib/ or warp build says so in one line. Only `warp rebuild`
//! builds one: a missing one is an error naming it.
use crate::compiler::self_hosted::{compiler_inputs_hash, library, module_of_bytes};
use crate::node::{error, Node};
use crate::pipeline::CompiledModule;
use std::path::{Path, PathBuf};

/// compiler.warp's command compiling one REPL line (core/compiler.warp repl_step)
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
/// The seed compiler, beside lib/ in a checkout (or in the installed folder): compiler.warp compiled by itself at a
/// fixpoint, what every build starts from (seed/README.md, scripts/refresh-seed.sh)
const SEED_FILE: &str = "seed/compiler-self.wasm";
/// Names another seed file (a candidate seed tried before it is checked in)
const SEED_VARIABLE: &str = "WARP_SEED";
/// The ending of an output compiler.warp writes parser.warp's view to, as JSON, instead of a module (its VIEW_SUFFIX)
const VIEW_EXTENSION: &str = "view.json";

/// Who builds compiler.warp's module: for `warp pure` the seed, for `warp self` compiler.wasm, compiler.warp itself
// ⚠ RustCompiled keeps its name until the Rust compiler leaves main (card rust-bootstrap): the seed builds it now
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

/// The node parser.warp reads `code` as: compiler.wasm, the compiler of `warp pure`, writes its view as JSON
/// (compiler.warp's view mode), read back with node_views (card rust-bootstrap: the host reads warp text without a
/// Rust parser: a channel's message, a console line, a test file, `warp data`, `warp parse`). An error when there is
/// no compiler.wasm or it fails
pub fn parsed(code: &str) -> Node {
	match parsed_view(code) {
		Ok(json) => match serde_json::from_str(&json) {
			Ok(view) => crate::compiler::node_views::node_of_view(&crate::compiler::self_hosted::view_of_json(view)),
			Err(failure) => error(&format!("compiler.warp's view of the code is no JSON: {failure}")),
		},
		Err(failure) => failure,
	}
}

/// The view JSON compiler.wasm writes of `code`, kept by the code and the compiler: the children of `warp test` read
/// their file's view, not their file again
fn parsed_view(code: &str) -> Result<String, Node> {
	use crate::run::module_cache::{keep_bytes, kept_bytes};
	let compiler = compiler(Compiler::RustCompiled)?;
	let owner = crate::extensions::sha256_hex(&compiler.bytes);
	let key = crate::extensions::sha256_hex(format!("{VIEW_EXTENSION}\n{owner}\n{code}").as_bytes());
	if let Some(json) = kept_bytes(&key, VIEW_EXTENSION).and_then(|bytes| String::from_utf8(bytes).ok()) {
		return Ok(json);
	}
	static PARSES: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
	let parse = PARSES.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
	let folder = std::env::temp_dir();
	let source = folder.join(format!("warp-parse-{}-{parse}.warp", std::process::id()));
	let output = source.with_extension(VIEW_EXTENSION);
	std::fs::write(&source, code).map_err(|failure| error(&format!("{}: {failure}", source.display())))?;
	let said = run_compiler(&source, &output, &folder, compiler, kept_scope(Compiler::RustCompiled));
	let json = std::fs::read_to_string(&output);
	let _ = std::fs::remove_file(&source);
	let _ = std::fs::remove_file(&output);
	match (said.drop_meta(), json) {
		(Node::Error(_), _) => Err(inline_positions(&said, &source)),
		(_, Ok(json)) => {
			keep_bytes(&key, VIEW_EXTENSION, json.as_bytes());
			Ok(json)
		}
		(_, Err(_)) => Err(error(&format!("compiler.warp wrote no view, it gave {}", said.serialize()))),
	}
}

/// The module compiler.warp makes of `code`, compiled as a program file of `folder` would be
pub fn compiled_code(code: &str, folder: &Path, by: Compiler) -> Result<CompiledModule, Node> {
	let file = std::env::temp_dir().join(format!("warp-inline-{}.warp", std::process::id()));
	std::fs::write(&file, format!("{code}\n")).map_err(|failure| error(&format!("{}: {failure}", file.display())))?;
	let module = compiled_in(&file, folder, by).map_err(|failure| inline_positions(&failure, &file));
	let _ = std::fs::remove_file(&file);
	module
}

/// `warp pure <command> …` (card wasm-commands): compiler.warp's own command (core/commands.warp), the command
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
	compiled_with(program, folder, compiler(by)?, kept_scope(by))
}

/// The module `compiler` (a module of compiler.warp) makes of the program file with `folder` in scope
fn compiled_with(
	program: &Path,
	folder: &Path,
	compiler: CompiledModule,
	scope: Option<String>,
) -> Result<CompiledModule, Node> {
	let output = std::env::temp_dir().join(format!("warp-pure-{}.wasm", std::process::id()));
	let _ = std::fs::remove_file(&output);
	let said = run_compiler(program, &output, folder, compiler, scope);
	let bytes = std::fs::read(&output);
	let _ = std::fs::remove_file(&output);
	match (said.drop_meta(), bytes) {
		// a position the compiler's run names is one in compiler.warp (its `raise`), not in the program
		(Node::Error(_), _) => Err(crate::errors::diagnostic::located_in_file(&said, &library().join(COMPILER_FILE))),
		(_, Ok(bytes)) => Ok(module_of_bytes(bytes)),
		(_, Err(_)) => Err(error(&format!("compiler.warp wrote no module, it gave {}", said.serialize()))),
	}
}

/// What the run of `compiler` (a module of compiler.warp) said, writing `output` of the program file with `folder` in
/// scope: `compiler.warp <source> <output> <folder> <lib>`, a module, or parser.warp's view when `output` ends in
/// VIEW_EXTENSION, its kept values in `scope` (kept_scope). The arguments of the program running, if any (a host word
/// parses in its run), stay its own
fn run_compiler(program: &Path, output: &Path, folder: &Path, compiler: CompiledModule, scope: Option<String>) -> Node {
	let running = crate::modules::std_adapters::program_arguments();
	let arguments = [program, output, folder, library()];
	crate::modules::std_adapters::set_program_arguments(
		arguments.iter().map(|path| path.display().to_string()).collect(),
	);
	let owner = crate::extensions::sha256_hex(&compiler.bytes);
	let said = crate::modules::std_adapters::kept_by(owner, scope, || crate::pipeline::run_module(compiler));
	crate::modules::std_adapters::set_program_arguments(running);
	said
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
/// installed-compilers: no run rebuilds a compiler, a stale one is at most a hint). A worktree without its own build in
/// data/ uses the delivered one when that was built from the same compiler inputs (card compiler-cache-keys). One line
/// when a worktree's build in data/ was built from other compiler inputs or warp build; the delivered pair next to the
/// binary is used as it is
fn compiler(by: Compiler) -> Result<CompiledModule, Node> {
	let path = build_path(by);
	let delivered = binary_folder().map(|folder| folder.join(by.file_name())).filter(|file| *file != path);
	let same_inputs = |bytes: &Vec<u8>| built_from(bytes)[1..BUILD_PARTS_JUDGED] == judged_build()[1..];
	let found = std::fs::read(&path).ok().map(|bytes| (path.clone(), bytes)).or_else(|| {
		let file = delivered?;
		std::fs::read(&file).ok().filter(same_inputs).map(|bytes| (file, bytes))
	});
	let Some((path, bytes)) = found else {
		let rebuild = format!("warp {REBUILD_WORD} {}", by.command());
		return Err(error(&format!(
			"no {} at {}: `{rebuild}` builds it, scripts/rebuild.sh copies the shared pair for its inputs",
			by.file_name(),
			path.display()
		)));
	};
	if !is_delivered(&path) && !same_inputs(&bytes) {
		eprintln!(
			"{} was built from other compiler inputs or warp build: `warp rebuild` builds it anew",
			path.display()
		);
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

/// compiler.warp compiled for `by` (by the seed, or by compiler.wasm: builder), written to `path` with the build section
fn build(by: Compiler, path: &Path) -> Result<CompiledModule, Node> {
	let file = library().join(COMPILER_FILE);
	let file = file.canonicalize().map_err(|failure| error(&format!("{}: {failure}", file.display())))?;
	let folder = file.parent().unwrap_or(library()).to_path_buf();
	let (builder, compiler, scope) = builder(by)?;
	let module = compiled_with(&file, &folder, compiler, scope)
		.map_err(|failure| error(&format!("compiler.warp compiled by {builder}: {}", failure.serialize())))?;
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

/// Who builds compiler.warp for `by`, named, and its module: compiler.wasm for `warp self` when there is one of these
/// compiler inputs (`warp rebuild` builds compiler.wasm first), else the seed; with the scope of its kept values
/// (kept_scope). No Rust-compiled stage (card rust-bootstrap)
fn builder(by: Compiler) -> Result<(String, CompiledModule, Option<String>), Node> {
	if let Compiler::SelfCompiled = by {
		if let Some((path, bytes)) = current_build_file(Compiler::RustCompiled) {
			return Ok((path.display().to_string(), module_of_bytes(bytes), kept_scope(Compiler::RustCompiled)));
		}
		eprintln!(
			"no compiler.wasm of these compiler inputs: the seed builds compiler-self.wasm (`warp {REBUILD_WORD}` builds both, compiler-self.wasm by compiler.wasm)"
		);
	}
	let seed = seed_file();
	let bytes = std::fs::read(&seed).map_err(|failure| {
		error(&format!(
			"no seed compiler at {} ({failure}): {SEED_VARIABLE} names one, scripts/refresh-seed.sh makes one",
			seed.display()
		))
	})?;
	Ok((format!("the seed {}", seed.display()), module_of_bytes(bytes), None))
}

/// Where a compiler's run keeps the values its makers made (os.keep_of; std_adapters kept_by): compiler.wasm, which the
/// seed compiled, under the makers' files of core and the seed, so an edit of core outside them keeps
/// them (card speed-module-cache: a used file's parse survives a compiler edit beyond the parser); None for
/// compiler-self.wasm and the seed, which all of core compiled (a seed of its time), whose values stay their own
fn kept_scope(by: Compiler) -> Option<String> {
	match by {
		Compiler::RustCompiled => std::fs::read(seed_file()).ok().map(|seed| crate::extensions::sha256_hex(&seed)),
		Compiler::SelfCompiled => None,
	}
}

/// The seed: WARP_SEED, else seed/compiler-self.wasm beside lib/ (a checkout's), else in the installed folder
fn seed_file() -> PathBuf {
	if let Some(file) = std::env::var_os(SEED_VARIABLE) {
		return PathBuf::from(file);
	}
	let beside_library = library().parent().unwrap_or(library()).join(SEED_FILE);
	let installed = binary_folder().map(|folder| folder.join(SEED_FILE)).filter(|file| file.exists());
	if beside_library.exists() {
		beside_library
	} else {
		installed.unwrap_or(beside_library)
	}
}

/// The build file of `by` for these compiler inputs, and its bytes: the build path's, else the delivered one; None when
/// neither was built from them
fn current_build_file(by: Compiler) -> Option<(PathBuf, Vec<u8>)> {
	let path = build_path(by);
	let delivered = binary_folder().map(|folder| folder.join(by.file_name()));
	let same_inputs = |bytes: &Vec<u8>| built_from(bytes)[1..BUILD_PARTS_JUDGED] == judged_build()[1..];
	std::iter::once(path)
		.chain(delivered)
		.find_map(|file| std::fs::read(&file).ok().filter(same_inputs).map(|bytes| (file, bytes)))
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

/// What a build of now is made from: lib/ (where, not judged), the compiler inputs hash (core and the modules it
/// embeds, with WARP_SOURCE: scripts/compiler-inputs.sh's), the sources of this warp (WARP_SOURCE, build.rs). An edit of
/// lib/ outside the compiler's inputs leaves a build current (card compiler-cache-keys)
fn judged_build() -> Vec<String> {
	vec![library().display().to_string(), compiler_inputs_hash(), env!("WARP_SOURCE").to_string()]
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
