//! `warp <program.warp> [arguments…]` (card warp-pure, notes/standalone_compiler.md): core/compiler.warp,
//! the warp compiler as one warp program, makes the program's module, which then runs. compiler.warp's own module is a
//! build output (decision compiler-build): warp.wasm, compiler.warp compiled by the seed (warp-seed.wasm beside the
//! binary, published on GitHub releases seed-<date>; card rust-bootstrap), for plain `warp`; warp-self.wasm, for `warp
//! self`, is no build of its own: a copy of warp.wasm, promoted when warp.wasm compiles compiler.warp once more (user
//! 2026-10-26: no fixpoint needed; `warp rebuild self`, scripts/promote-self.sh). The old names compiler.wasm and
//! compiler-self.wasm are still read when
//! the new ones are missing (transition). No Rust-compiled stage takes part. The pair next to the binary is the delivered
//! one (a release's, or main's, stamped with the commit it was built from) and is used as it is, whatever lib/ beside
//! it says (card installed-compilers); a lib/ named by WARP_LIB that the pair is not for keeps its own in
//! that lib/ folder itself (user 2026-10-26), where a run on one built from another lib/ or warp build says so in one line. Only `warp rebuild`
//! builds one: a missing one is an error naming it.
use crate::compiler::compiler_files::{compiler_inputs_hash, library, module_of_bytes};
use crate::node::{error, Node};
use crate::compiled_module::CompiledModule;
use std::path::{Path, PathBuf};

/// compiler.warp's command compiling one REPL line (core/compiler.warp repl_step)
const REPL_STEP: &str = "repl-step";
/// compiler.warp's command writing runtime.wasm, and the file it is beside warp.wasm
const RUNTIME_COMMAND: &str = "runtime";
const RUNTIME_FILE: &str = "runtime.wasm";
/// compiler.warp's command writing every standard module's <name>.wasm, and their folder beside warp.wasm (card
/// modules-prebuilt)
const MODULES_COMMAND: &str = "modules";
const MODULES_FOLDER: &str = "modules";
/// A seed's runtime.wasm, its own beside it (warp-seed.runtime.wasm): what the seed compiles compiler.warp behind,
/// written by the seed itself (ship_seed), never the runtime of the warp.wasm that shares the seed's folder
const SEED_RUNTIME_EXTENSION: &str = "runtime.wasm";

const COMPILER_FILE: &str = "compiler/compiler.warp";
/// The custom section of a build file naming what it was built from: lib/, lib/'s fingerprint, the warp build
const BUILD_SECTION: &str = "warp.build";
const BUILD_SEPARATOR: &str = "\n";
/// The build section's parts judged for staleness (judged_build's), the commit after them
const BUILD_PARTS_JUDGED: usize = 3;
/// `warp rebuild [pure|self]`, the one command that builds a compiler
const REBUILD_WORD: &str = "rebuild";
const SHOWN_COMMIT_LENGTH: usize = 10;
/// Names a lib/ of its own (a worktree's): only then can the compilers be other than those next to the binary
const LIBRARY_VARIABLE: &str = "WARP_LIB";
/// What a new build compiles once to compute its kept values
const WARMING_PROGRAM: &str = "0";
/// The seed compiler, beside the binary (no file of git: scripts/refresh-seed.sh publishes it on a GitHub release
/// seed-<date>): compiler.warp compiled by a stage that compiles itself once more, what every build starts from
const SEED_FILE: &str = "warp-seed.wasm";
/// Names another seed file (a candidate seed tried before it is checked in)
const SEED_VARIABLE: &str = "WARP_SEED";
/// The ending of an output compiler.warp writes parser.warp's view to, as JSON, instead of a module (its VIEW_SUFFIX)
const VIEW_EXTENSION: &str = "view.json";
/// Computing time of the compiler runs of `warp rebuild` (compiling compiler.warp, then warming the new build): the seed
/// compiling compiler.warp takes minutes on a small VM, no runaway, so a program's 60 s (WARP_TIME_LIMIT) does not apply
const REBUILD_TIME_LIMIT: std::time::Duration = std::time::Duration::from_secs(2 * 60 * 60);
/// Overrides REBUILD_TIME_LIMIT, in seconds
const REBUILD_TIME_LIMIT_VARIABLE: &str = "WARP_REBUILD_TIME_LIMIT";

/// Who builds compiler.warp's module: for plain `warp` the seed; `warp self`'s is warp.wasm itself, promoted when it compiles itself
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
			Compiler::RustCompiled => "warp.wasm",
			Compiler::SelfCompiled => "warp-self.wasm",
		}
	}

	/// The build file's old name, read when there is no file of the new name (transition: installs and data/ folders
	/// of before the rename, e.g. ~/dev/bin/main-builds/*/compiler.wasm)
	fn legacy_file_name(self) -> &'static str {
		match self {
			Compiler::RustCompiled => "compiler.wasm",
			Compiler::SelfCompiled => "compiler-self.wasm",
		}
	}

	/// The build file in `folder`: of the new name, else of the old one when only that is there
	fn file_in(self, folder: &Path) -> PathBuf {
		let file = folder.join(self.file_name());
		let legacy = folder.join(self.legacy_file_name());
		if !file.exists() && legacy.exists() {
			legacy
		} else {
			file
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

/// The value of `code` (`warp "x=√72.;x^2"`, a test case), compiled as a program file of `folder` would be
pub fn run_code(code: &str, folder: &Path, by: Compiler) -> Node {
	ran(compiled_code(code, folder, by), Vec::new())
}

/// The node parser.warp reads `code` as: warp.wasm, the compiler of plain `warp`, writes its view as JSON
/// (compiler.warp's view mode), read back with node_views (card rust-bootstrap: the host reads warp text without a
/// Rust parser: a channel's message, a console line, a test file, `warp data`, `warp parse`). An error when there is
/// no warp.wasm or it fails
pub fn parsed(code: &str) -> Node {
	match parsed_view(code) {
		Ok(json) => match serde_json::from_str(&json) {
			Ok(view) => crate::compiler::node_views::node_of_view(&crate::compiler::compiler_files::view_of_json(view)),
			Err(failure) => error(&format!("compiler.warp's view of the code is no JSON: {failure}")),
		},
		Err(failure) => failure,
	}
}

/// The view JSON warp.wasm writes of `code`, kept by the code and the compiler: the children of `warp test` read
/// their file's view, not their file again
fn parsed_view(code: &str) -> Result<String, Node> {
	use crate::run::module_cache::{keep_bytes, kept_bytes};
	let compiler = compiler(Compiler::RustCompiled)?;
	let owner = crate::extensions::hex_text(&crate::run::module_cache::module_digest(&compiler.bytes));
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
	let said = run_compiler(&source, &output, &folder, compiler, kept_scope(Compiler::RustCompiled), None);
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
	compiled_code_and_separate(code, folder, by, None)
}

/// The module of `code` behind runtime.wasm, and from the same compile, written to `separate`, the module importing
/// runtime.wasm's instance instead (notes/runtime_linking.md "Separate modules")
pub fn compiled_code_and_separate(
	code: &str,
	folder: &Path,
	by: Compiler,
	separate: Option<&Path>,
) -> Result<CompiledModule, Node> {
	let file = std::env::temp_dir().join(format!("warp-inline-{}.warp", std::process::id()));
	std::fs::write(&file, format!("{code}\n")).map_err(|failure| error(&format!("{}: {failure}", file.display())))?;
	let module = compiled_in(&file, folder, by, separate).map_err(|failure| inline_positions(&failure, &file));
	let _ = std::fs::remove_file(&file);
	module
}

/// `warp <command> …` (card wasm-commands): compiler.warp's own command (core/commands.warp), the command
/// line its arguments
pub fn command(arguments: Vec<String>, by: Compiler) -> Node {
	ran(compiler(by), arguments)
}

/// One line of `warp repl`: compiler.warp's repl-step compiles it after the lines kept in `state` and decides which
/// stay; its module runs here, where plain WASI could not run it
pub fn repl_line(line: &str, state: &Path, by: Compiler) -> Node {
	let module = state.with_extension("wasm");
	let compiler = match compiler(by) {
		Ok(compiler) => compiler,
		Err(failure) => return failure,
	};
	// a line compiles as a program does, behind the compiler's runtime, its fifth argument too
	let runtime = runtime_of(&compiler);
	let step = [REPL_STEP, &state.display().to_string(), line, &module.display().to_string(), &runtime.display().to_string()];
	let stepped = ran(Ok(compiler), step.map(String::from).to_vec());
	if let Node::Error(_) = stepped.drop_meta() {
		return stepped;
	}
	match std::fs::read(&module) {
		Ok(bytes) => crate::compiled_module::run_module(module_of_bytes(bytes)),
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
			crate::compiled_module::run_module(module)
		}
		Err(failure) => failure,
	}
}

/// The module compiler.warp makes of the program file, its folder in scope
pub fn compiled(program: &Path, by: Compiler) -> Result<CompiledModule, Node> {
	let program = program.canonicalize().map_err(|failure| error(&format!("{}: {failure}", program.display())))?;
	let folder = program.parent().unwrap_or(Path::new("/")).to_path_buf();
	compiled_in(&program, &folder, by, None)
}

/// The module compiler.warp makes of the program file with `folder` in scope: `compiler.warp <source> <wasm> <folder> <lib>
/// <runtime> [<separate>]`
fn compiled_in(program: &Path, folder: &Path, by: Compiler, separate: Option<&Path>) -> Result<CompiledModule, Node> {
	compiled_with(program, folder, compiler(by)?, kept_scope(by), separate)
}

/// The module `compiler` (a module of compiler.warp) makes of the program file with `folder` in scope
fn compiled_with(
	program: &Path,
	folder: &Path,
	compiler: CompiledModule,
	scope: Option<String>,
	separate: Option<&Path>,
) -> Result<CompiledModule, Node> {
	let output = std::env::temp_dir().join(format!("warp-pure-{}.wasm", std::process::id()));
	let _ = std::fs::remove_file(&output);
	let said = run_compiler(program, &output, folder, compiler, scope, separate);
	let bytes = std::fs::read(&output);
	let _ = std::fs::remove_file(&output);
	match (said.drop_meta(), bytes) {
		// a position the compiler's run names is no place of the program, nor one in compiler.warp: its line and column
		// may be in any of core's files, so it is left out (a refusal reads as `outside the core: the call zork(…)`)
		(Node::Error(_), _) => Err(crate::errors::diagnostic::without_position(&said)),
		(_, Ok(bytes)) => Ok(module_of_bytes(bytes)),
		(_, Err(_)) => Err(error(&format!("compiler.warp wrote no module, it gave {}", said.serialize()))),
	}
}

/// What the run of `compiler` (a module of compiler.warp) said, writing `output` of the program file with `folder` in
/// scope: `compiler.warp <source> <output> <folder> <lib>`, a module, or parser.warp's view when `output` ends in
/// VIEW_EXTENSION, its kept values in `scope` (kept_scope). The arguments of the program running, if any (a host word
/// parses in its run), stay its own
fn run_compiler(
	program: &Path,
	output: &Path,
	folder: &Path,
	compiler: CompiledModule,
	scope: Option<String>,
	separate: Option<&Path>,
) -> Node {
	let running = crate::modules::std_adapters::program_arguments();
	let runtime = runtime_of(&compiler);
	let arguments = [program, output, folder, library(), &runtime].into_iter().chain(separate);
	crate::modules::std_adapters::set_program_arguments(arguments.map(|path| path.display().to_string()).collect());
	let owner = crate::extensions::hex_text(&crate::run::module_cache::module_digest(&compiler.bytes));
	// its kept values shipped beside its build file serve a cold cache; a new build's warming run writes them there
	let shipped = crate::run::module_cache::shipped_kept_folder(&compiler.bytes);
	let warming = crate::run::module_cache::warming();
	let said = crate::host::sandbox::as_toolchain(|| {
		crate::run::module_cache::with_shipped_kept(shipped, warming, || {
			crate::modules::std_adapters::kept_by(owner, scope, || crate::compiled_module::run_module(compiler))
		})
	});
	crate::modules::std_adapters::set_program_arguments(running);
	said
}

/// `warp rebuild [pure]`: warp.wasm built anew from the current lib/, its path, or why it failed. `warp rebuild self`:
/// warp.wasm (built first when there is none of these compiler inputs) promoted to warp-self.wasm when it compiles
/// compiler.warp once more, else why not, warp-self.wasm left as it was
pub fn rebuild(by: Compiler) -> Result<PathBuf, Node> {
	if let Compiler::RustCompiled = by {
		ship_seed()?;
	}
	let path = build_path(by);
	crate::util::with_time_limit(rebuild_time_limit(), || match by {
		Compiler::RustCompiled => build(by, &path),
		Compiler::SelfCompiled => promote(&path),
	})?;
	Ok(path)
}

/// `warp rebuild modules`: the standard modules beside warp.wasm, compiled behind its runtime.wasm (~25 s); a word of
/// its own until a link appends them, then part of every rebuild (card modules-prebuilt)
pub fn rebuild_modules() -> Result<PathBuf, Node> {
	let path = build_path(Compiler::RustCompiled);
	let folder = path.with_file_name(MODULES_FOLDER);
	write_modules(Compiler::RustCompiled, &folder, &path.with_file_name(RUNTIME_FILE))?;
	Ok(folder)
}

/// The time limit of a rebuild's compiler runs: WARP_REBUILD_TIME_LIMIT seconds when positive, else REBUILD_TIME_LIMIT
fn rebuild_time_limit() -> std::time::Duration {
	let seconds =
		std::env::var(REBUILD_TIME_LIMIT_VARIABLE).ok().and_then(|seconds| seconds.trim().parse::<f64>().ok());
	seconds.filter(|seconds| *seconds > 0.0).map_or(REBUILD_TIME_LIMIT, std::time::Duration::from_secs_f64)
}

/// The self test of warp.wasm (stage N): it compiles core/compiler.warp again (stage N+1), and when it does, warp.wasm is
/// copied to `path`, warp-self.wasm; whether stage N+1 is byte-identical to its own module (the fixpoint) is said.
/// A warp.wasm of other compiler inputs is built anew first
fn promote(path: &Path) -> Result<CompiledModule, Node> {
	let pure = Compiler::RustCompiled;
	let (warp_file, warp_bytes) = match current_build_file(pure) {
		Some(found) => found,
		None => {
			eprintln!("no {} of these compiler inputs: building it first", pure.file_name());
			let built = build_path(pure);
			build(pure, &built)?;
			let bytes = std::fs::read(&built).map_err(|failure| error(&format!("{}: {failure}", built.display())))?;
			(built, bytes)
		}
	};
	let file = library().join(COMPILER_FILE);
	let file = file.canonicalize().map_err(|failure| error(&format!("{}: {failure}", file.display())))?;
	let folder = file.parent().unwrap_or(library()).to_path_buf();
	let next =
		compiled_with(&file, &folder, shipped(&warp_file, warp_bytes.clone()), kept_scope(pure), None).map_err(|failure| {
			error(&format!("compiler.warp compiled by {}: {}", warp_file.display(), failure.serialize()))
		})?;
	// byte-identical output (the fixpoint) is reported, not required (user 2026-10-26: "If the compiler can compile
	// itself once more, we don't need the fixed point")
	let fixpoint = is_module_with_build_section(&warp_bytes, &next.bytes);
	let written = path.parent().map_or(Ok(()), std::fs::create_dir_all).and_then(|_| write_new(path, &warp_bytes));
	written.map_err(|failure| error(&format!("{}: {failure}", path.display())))?;
	// the same bytes, so the same machine code: warp.wasm's .cwasm copied, made only when it is not current
	let (engine, _) = crate::run::wasm_reader::engine_for(&warp_bytes);
	let shipped_warp = crate::run::module_cache::shipped_path(&warp_file);
	let copied = crate::run::module_cache::shipped_current(&engine, &warp_file, &warp_bytes)
		&& std::fs::copy(&shipped_warp, crate::run::module_cache::shipped_path(path)).is_ok();
	if !copied {
		ship(path, &warp_bytes)?;
	}
	eprintln!(
		"{} compiles compiler.warp once more ({}), promoted to {}",
		warp_file.display(),
		if fixpoint { "to its own bytes, a fixpoint" } else { "to other bytes, no fixpoint" },
		path.display()
	);
	warm(Compiler::SelfCompiled)?;
	Ok(next)
}

/// Whether `file` is `module` followed by one custom section (the build section `build` appends) and nothing else
fn is_module_with_build_section(file: &[u8], module: &[u8]) -> bool {
	let Some(rest) = file.strip_prefix(module) else { return false };
	let Some((&0, sized)) = rest.split_first() else { return rest.is_empty() };
	// the section's size, unsigned LEB128, then exactly that many bytes
	let (mut size, mut shift, mut read) = (0usize, 0u32, 0usize);
	for &byte in sized.iter().take(5) {
		size |= ((byte & 0x7f) as usize) << shift;
		read += 1;
		shift += 7;
		if byte & 0x80 == 0 {
			return sized.len() == read + size;
		}
	}
	false
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
	let path = found_path(by);
	let delivered = binary_folder().map(|folder| by.file_in(&folder)).filter(|file| *file != path);
	let same_inputs = |bytes: &Vec<u8>| built_from(bytes)[1..BUILD_PARTS_JUDGED] == judged_build()[1..];
	let found = std::fs::read(&path).ok().map(|bytes| (path.clone(), bytes)).or_else(|| {
		let file = delivered?;
		std::fs::read(&file).ok().filter(same_inputs).map(|bytes| (file, bytes))
	});
	let Some((path, bytes)) = found else {
		let rebuild = match by {
			Compiler::RustCompiled => format!("`warp {REBUILD_WORD} pure` builds it"),
			Compiler::SelfCompiled => format!(
				"`warp {REBUILD_WORD} self` (scripts/promote-self.sh) promotes warp.wasm to it when warp.wasm compiles compiler.warp once more"
			),
		};
		return Err(error(&format!(
			"no {} at {}: {rebuild}, scripts/rebuild.sh copies the shared pair for its inputs",
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
	Ok(shipped(&path, bytes))
}

/// A build file's module, run from the machine code shipped beside it (module_cache shipped_with): a compiler is never
/// compiled at run time (card precompiled-compilers)
fn shipped(path: &Path, bytes: Vec<u8>) -> CompiledModule {
	crate::run::module_cache::shipped_with(path, &bytes);
	module_of_bytes(bytes)
}

/// The build file's machine code written beside it, for the engine its runs use (`<file>.cwasm`)
fn ship(path: &Path, bytes: &[u8]) -> Result<(), Node> {
	let (engine, _) = crate::run::wasm_reader::engine_for(bytes);
	crate::run::module_cache::write_shipped(&engine, path, bytes)
		.map(|_| ())
		.map_err(|failure| error(&format!("{}: cannot precompile: {failure:#}", path.display())))
}

/// Where the delivered compilers lie: next to the binary, or in an installed warp's share/warp
fn binary_folder() -> Option<PathBuf> {
	crate::modules::installed_folder()
}

/// A build file of the installed folder: the delivered compiler
fn is_delivered(path: &Path) -> bool {
	binary_folder().is_some_and(|folder| path.parent() == Some(folder.as_path()))
}

/// compiler.warp compiled for `by` (by the seed: builder), written to `path` with the build section
fn build(by: Compiler, path: &Path) -> Result<CompiledModule, Node> {
	let file = library().join(COMPILER_FILE);
	let file = file.canonicalize().map_err(|failure| error(&format!("{}: {failure}", file.display())))?;
	let folder = file.parent().unwrap_or(library()).to_path_buf();
	let (builder, compiler, scope) = builder(by)?;
	let module = compiled_with(&file, &folder, compiler, scope, None)
		.map_err(|failure| error(&format!("compiler.warp compiled by {builder}: {}", failure.serialize())))?;
	let mut bytes = module.bytes.clone();
	let section = current_build().join(BUILD_SEPARATOR);
	wasm_encoder::Section::append_to(
		&wasm_encoder::CustomSection { name: BUILD_SECTION.into(), data: section.as_bytes().into() },
		&mut bytes,
	);
	let written = path.parent().map_or(Ok(()), std::fs::create_dir_all).and_then(|_| write_new(path, &bytes));
	written.map_err(|failure| error(&format!("{}: {failure}", path.display())))?;
	ship(path, &bytes)?;
	write_runtime(by, &path.with_file_name(RUNTIME_FILE))?;
	warm(by)?;
	Ok(module)
}

/// runtime.wasm beside warp.wasm: the runtime words compiled once by compiler.warp's `runtime` command, for linking
/// programs to (notes/runtime_linking.md "runtime.wasm format")
fn write_runtime(by: Compiler, path: &Path) -> Result<(), Node> {
	written_by_command(vec![RUNTIME_COMMAND.into(), path.display().to_string()], path, by)
}

/// the standard modules beside runtime.wasm, each compiled once behind it, for linking programs to (notes/runtime_linking.md
/// "Library modules"); compiler.warp names them and those it could not prebuild
fn write_modules(by: Compiler, folder: &Path, runtime: &Path) -> Result<(), Node> {
	std::fs::create_dir_all(folder).map_err(|failure| error(&format!("{}: {failure}", folder.display())))?;
	let words = vec![MODULES_COMMAND.into(), folder.display().to_string(), runtime.display().to_string()];
	written_by_command(words, folder, by)
}

fn written_by_command(words: Vec<String>, path: &Path, by: Compiler) -> Result<(), Node> {
	let written = command(words, by);
	match written.drop_meta() {
		Node::Error(_) => Err(error(&format!("{}: {}", path.display(), written.serialize()))),
		_ => Ok(()),
	}
}

/// The runtime.wasm a compiler compiles a program behind (card runtime-prepended): a seed's own, else the one beside
/// it (warp.wasm's and warp-self.wasm's); none (an empty path, which compiler.warp refuses loudly) when it is missing
fn runtime_of(compiler: &CompiledModule) -> PathBuf {
	crate::run::module_cache::shipped_file(&compiler.bytes)
		.map(|file| if file == seed_file() { seed_runtime(&file) } else { file.with_file_name(RUNTIME_FILE) })
		.filter(|file| file.exists())
		.unwrap_or_default()
}

fn seed_runtime(seed: &Path) -> PathBuf {
	seed.with_extension(SEED_RUNTIME_EXTENSION)
}

/// Who builds compiler.warp's warp.wasm, named, and its module: the seed, with the scope of its kept values (kept_scope:
/// its own). No Rust-compiled stage (card rust-bootstrap); warp-self.wasm is never built, only promoted (promote)
fn builder(_by: Compiler) -> Result<(String, CompiledModule, Option<String>), Node> {
	let seed = seed_file();
	let bytes = std::fs::read(&seed).map_err(|failure| {
		error(&format!(
			"no seed compiler at {} ({failure}): {SEED_VARIABLE} names one, `gh release download` of the newest seed-* release fetches one",
			seed.display()
		))
	})?;
	Ok((format!("the seed {}", seed.display()), shipped(&seed, bytes), None))
}

/// `warp rebuild seed`, and before `warp rebuild pure`: the seed's machine code beside it (warp-seed.wasm.cwasm),
/// made when it is missing or stale (a refreshed seed: scripts/refresh-seed.sh), so no build compiles the seed at run
/// time (card precompiled-compilers); its path
pub fn ship_seed() -> Result<PathBuf, Node> {
	let seed = seed_file();
	let bytes =
		std::fs::read(&seed).map_err(|failure| error(&format!("no seed compiler at {}: {failure}", seed.display())))?;
	let (engine, _) = crate::run::wasm_reader::engine_for(&bytes);
	if !crate::run::module_cache::shipped_current(&engine, &seed, &bytes) {
		eprintln!("precompiling the seed {} (once per seed and engine)", seed.display());
		ship(&seed, &bytes)?;
	}
	let runtime = seed_runtime(&seed);
	if !is_newer(&runtime, &seed) {
		write_seed_runtime(&seed, bytes, &runtime);
	}
	Ok(crate::run::module_cache::shipped_path(&seed))
}

/// The seed's runtime.wasm, written by the seed's `runtime` command (once per seed); a seed of before the command
/// writes none and needs none, said once
fn write_seed_runtime(seed: &Path, bytes: Vec<u8>, runtime: &Path) {
	eprintln!("writing the seed's runtime {} (once per seed)", runtime.display());
	let arguments = vec![RUNTIME_COMMAND.into(), runtime.display().to_string()];
	let written = ran(Ok(shipped(seed, bytes)), arguments);
	if !runtime.exists() {
		eprintln!("note: the seed wrote no runtime ({}): it compiles without one", written.serialize());
	}
}

fn is_newer(file: &Path, than: &Path) -> bool {
	let modified = |path: &Path| std::fs::metadata(path).and_then(|metadata| metadata.modified()).ok();
	matches!((modified(file), modified(than)), (Some(file), Some(than)) if file >= than)
}

/// Where a compiler's run keeps the values its makers made (os.keep_of; std_adapters kept_by): warp.wasm, which the
/// seed compiled, under the makers' files of core and the seed, so an edit of core outside them keeps
/// them (card speed-module-cache: a used file's parse survives a compiler edit beyond the parser); None for
/// warp-self.wasm and the seed, which all of core compiled (a seed of its time), whose values stay their own
fn kept_scope(by: Compiler) -> Option<String> {
	match by {
		Compiler::RustCompiled => std::fs::read(seed_file()).ok().map(|seed| crate::extensions::sha256_hex(&seed)),
		Compiler::SelfCompiled => None,
	}
}

/// The seed: WARP_SEED, else warp-seed.wasm beside the binary, else beside the link it was started by (~/dev/bin/warp
/// links to a build folder)
fn seed_file() -> PathBuf {
	if let Some(file) = std::env::var_os(SEED_VARIABLE) {
		return PathBuf::from(file);
	}
	let started_by = std::env::current_exe().ok().and_then(|link| link.parent().map(Path::to_path_buf));
	let places: Vec<PathBuf> = binary_folder().into_iter().chain(started_by).collect();
	let found = places.iter().map(|folder| folder.join(SEED_FILE)).find(|file| file.exists());
	found.unwrap_or_else(|| places.first().cloned().unwrap_or_default().join(SEED_FILE))
}

/// The build file of `by` for these compiler inputs, and its bytes: the build path's, else the delivered one; None when
/// neither was built from them
fn current_build_file(by: Compiler) -> Option<(PathBuf, Vec<u8>)> {
	let path = found_path(by);
	let delivered = binary_folder().map(|folder| by.file_in(&folder));
	let same_inputs = |bytes: &Vec<u8>| built_from(bytes)[1..BUILD_PARTS_JUDGED] == judged_build()[1..];
	std::iter::once(path)
		.chain(delivered)
		.find_map(|file| std::fs::read(&file).ok().filter(same_inputs).map(|bytes| (file, bytes)))
}

/// The new build run once on a small program: the values compiler.warp keeps for every run (`kept(…) ?? keep(…)`, its
/// lowered runtime words: ~50 s on the Mac) are computed by the build, not by the first program's run within its time
/// limit (card cloud-lane-narrow: the first `warp self` after `warp rebuild self` ran out of time on a cloud VM)
fn warm(by: Compiler) -> Result<(), Node> {
	let _ = std::fs::remove_dir_all(crate::run::module_cache::shipped_kept_folder_of(&build_path(by)));
	crate::run::module_cache::warming_with(|| compiled_code(WARMING_PROGRAM, library(), by)).map(|_| ())
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

/// `warp version`'s line of the compilers in use: `warp.wasm of <commit>, warp-self.wasm missing`
pub fn compilers_line() -> String {
	let stamp = |by: Compiler| {
		let path = found_path(by);
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

/// runtime.wasm beside the compiler, which a separately linked program imports its runtime words from
pub fn runtime_path() -> PathBuf {
	found_path(Compiler::RustCompiled).with_file_name(RUNTIME_FILE)
}

/// The build file as build_path names it, or the file of the old name in its folder when only that is there
fn found_path(by: Compiler) -> PathBuf {
	let path = build_path(by);
	path.parent().map_or(path.clone(), |folder| by.file_in(folder))
}

/// The build file: in the lib/ WARP_LIB names, which keeps its compilers in itself (user 2026-10-26), else next to
/// the binary. A build with WARP_LIB never writes beside the binary: that judged the binary's folder by the pair in it
/// and took it when warp.wasm was missing there, so a worktree's rebuild replaced the installed compiler (card
/// rebuild-writes-main). The pair beside the binary is still read as the delivered one (compiler, same inputs)
fn build_path(by: Compiler) -> PathBuf {
	match (std::env::var_os(LIBRARY_VARIABLE), binary_folder()) {
		(None, Some(folder)) => folder.join(by.file_name()),
		_ => library().join(by.file_name()),
	}
}
