#![cfg_attr(test, allow(unused))] // main() is not compiled under test
use extensions::numbers::Number;
use node::Node;
use std::env;
use std::fs;
use std::io::{self, IsTerminal, Read, Write};
use warp::node;
use warp::run::module_cache::COMPILED_EXTENSION;
use warp::util::{dirs_home, extract_after, file_exists, load_file};
use warp::{errors::diagnostic, extensions, run, run::wasm_reader, util};
use warp_runtime::host_words::HOST_LIBRARY;

mod cli;
use cli::*;

/// `warp a.wasm -- x y`: the words after it are the program's arguments (`use os; args`), for every way to run one
const PROGRAM_ARGUMENTS_SEPARATOR: &str = "--";
/// `warp compile --aot` also writes the machine code (wasmtime's .cwasm), which `warp <file>.cwasm` runs
const AOT_FLAG: &str = "--aot";
/// P103 (user): "Just giving it a file will compile it": `warp hello.warp` runs the program and leaves the standalone
/// executable `hello` next to it (`hello.exe` on Windows), the warp-runtime stub with the program's machine code appended.
/// `warp build <file>` / `warp compile <file>` (not in the help any more) only make it, and the module <file>.wasm
/// (card build-should); `--exe` only the executable, `--wasm` (and `--aot`) only the module
const EXE_FLAG: &str = "--exe";
const WASM_FLAG: &str = "--wasm";
/// `warp help list`: a standard module's words; `warp help --markdown`: all of them as wiki/standard-library.md
const HELP_PREFIX: &str = "help ";
const HELP_MARKDOWN: &str = "help --markdown";
/// `warp build --site app.warp`: the directory app-site/ a web server serves (src/web/site.rs, card web-ssr)
const SITE_FLAG: &str = "--site";
/// `warp compile --page app.warp`: app.wasm for a page of its own, its path imports named page:modules/<file> and those
/// modules written to modules/ beside it, as a site ships them (web/uniscript/build.sh, card imports-wasm)
const PAGE_FLAG: &str = "--page";
/// `warp build --wit app.warp`: the WIT world of the program's `component` declaration, app.wit (card wasm-interop-rest)
const WIT_FLAG: &str = "--wit";
/// `warp build --component app.warp`: app.component.wasm, the component of that world (src/modules/component_builder.rs)
const COMPONENT_FLAG: &str = "--component";
/// `warp build --wagi app.warp`: app.wagi.wasm, its routes answering one WAGI request (lib/wagi.warp), and app.spin.toml
/// to run it: `spin up -f app.spin.toml` (Spin's wagi executor; Akamai Functions: `spin aka deploy`)
const WAGI_FLAG: &str = "--wagi";
/// The name of an executable built from inline code (plus the platform's extension)
const DEFAULT_EXECUTABLE_NAME: &str = "out";
/// `warp run <file>`: the program runs, no executable is left (P105)
const RUN_PREFIX: &str = "run ";
const TEST_WORD: &str = "test";
const COMPILE_COMMANDS: [&str; 3] = ["compile", "build", "link"];
/// `warp tool <package> [arguments…]`: runs the package's prebuilt <package>.wasm (src/modules/package_tools.rs)
const TOOL_COMMAND: &str = "tool";
/// `warp fetch [package…]`: the packages fetched into packages/ as `use` fetches them; every package of packages.warp
/// and the default registry when none is named (web/playground/build.sh pinned, for the browser suite)
const FETCH_COMMAND: &str = "fetch";
/// `warp dev <file> [port]`: serves the file's site and builds it anew when it changes (src/web/dev_server.rs, card web-dev)
const DEV_COMMAND: &str = "dev";
/// `warp app page.warp`: the program's page in a window (src/web/webview.rs, card read-chron)
const APP_COMMAND: &str = "app";
const APP_CHECK: &str = "--check";
/// `warp deploy [--dev|--dry-run] app.warp`: the program as a Cloudflare Worker, deployed by wrangler (src/web/deploy.rs)
const DEPLOY_COMMAND: &str = "deploy";
/// `warp serve [app.warp] [port]`: serves the program, its page, server functions and routes (P222)
const SERVE_COMMAND: &str = "serve";

/// The help menu (user): never extend it without consulting through the interviewer
const HELP_COLUMN: usize = 30;
const HELP_COMMANDS: &[(&str, &str)] = &[
	("warp", "Evaluation console (REPL)"),
	("warp <code|file(.warp/.wasm)>", "Run warp code or a file"),
	("warp build <code|file.warp>", "Create native app and wasm"),
	("warp version", "Show version"),
	("warp help [<topic>]", "Show this help"),
	("warp verify <file>", "Test and prove the laws of a file"),
	("warp debug <code|file>", "Show the parse, lowering and types"),
	("warp tool <package> [args]", "Run a package's prebuilt <package>.wasm in its directory"),
	("warp serve [file] [port]", "Serve the program: its page, server functions and routes (app.warp, port 8080)"),
	("warp deploy", "Host the program on Cloudflare (see deploy.md)"),
	("warp fuel <steps> <file|code>", "Count steps"),
	("warp test", "Run tests"),
	("warp docs", "Open documentation"),
];
const HELP_OPTIONS: &[(&str, &str)] = &[
	("--no-ask", "Never prompt \"got it?\" after a warning or note"),
	("--no-hints", "Hide hints and notes (prefer ^ over **), shown by default (env WARP_HINTS=0)"),
	("--warnings=<level>", "Show diagnostics from hint (default), weak, normal or severe up"),
	("--strict[=<level>]", "Warnings from normal (or the level) up are errors, as `use strict`"),
	("--sandbox", "No C, shell or other runtime, files only in the working folder"),
];
// Out of the menu (user's notes): self, rebuild: internal; eval: redundant, gets a note; repl: redundant (plain warp);
// lower, parse, types, format: folded into `warp debug` (TODO card cli-debug-command); data: what for?; dev; fast: get
// rid; compile --wasm/--aot/--page: redundant with build (aot always with build); build --exe/--wasm: nobody uses them;
// --trim: see full.wasm / mod.wasm (decision build-three-outputs); app: ?; last.wasm kept in ~/.cache/warp
/// `warp lower --json <code>`: the lowered program as the JSON node view core reads with `node_of(json(…))`,
/// lossless where the serialized text is not
const LOWER_JSON_COMMAND: &str = "lower --json";
const LOWER_COMMAND: &str = "lower";
/// `warp runtime <out.wasm>`: runtime.wasm, the runtime words compiled once (notes/runtime_linking.md)
const RUNTIME_COMMAND: &str = "runtime";
/// `warp pure …`: plain `warp …`, the one compiler (core/compiler.warp as warp.wasm) since the Rust compiler went; kept
/// as a hidden alias that says so (user, card drop-pure-word)
const PURE_COMMAND: &str = "pure";
const PURE_NOTE: &str = "note: `warp pure` is plain `warp` now";
/// `warp self <file> [arguments…]`: as plain `warp`, with warp-self.wasm, warp.wasm promoted at a fixpoint
const SELF_COMMAND: &str = "self";
/// `warp repl`: the console with each line compiled by compiler.warp (card wasm-commands)
const REPL_COMMAND: &str = "repl";
/// `warp self run <file|code>`, `warp self eval <code>`: the same as without the word, the program run here
const PURE_RUNNING_WORDS: [&str; 2] = ["run", "eval"];
/// `warp rebuild [pure|self]`: warp.wasm, the compiler of plain `warp`, built anew from lib/ by the seed
/// (warp-seed.wasm beside the binary); `self`: warp.wasm promoted to warp-self.wasm, `warp self`'s, when it compiles compiler.warp
/// to its own bytes (decision compiler-build, card rust-bootstrap; src/compiler/standalone.rs)
const REBUILD_COMMAND: &str = "rebuild";
/// The first words the command line takes as commands, for unknown_command's suggestion
const COMMAND_WORDS: [&str; 34] = [
	"run", "pure", "self", "rebuild", "test", "eval", "lower", "parse", "verify", "format", "lint", "beautify", "fix",
	"types", "data", "tool", "fetch", "dev", "serve", "deploy", "repl", "console", "compile", "build", "link", "app",
	"help", "version", "docs", "wiki", "home", "lsp", "start", "runtime",
];
/// How far a mistyped command may be from the command it suggests (`formt` → format)
const MISTYPED_LETTERS: usize = 2;
const SERVE_PORT: u16 = 8080;
const WARP_VERSION: &str = env!("CARGO_PKG_VERSION");
/// The warnings, notes and hints the user said "got it" to, in the home folder: one `ack:<topic> = acknowledged` per
/// line. A "got it" holds wherever warp runs next (card hints-dismissed)
const ACKNOWLEDGEMENTS_FILE: &str = ".warp-acknowledged";
/// Run the program with no C, shell or other runtime (Capability::GRANTED_SANDBOXED), writing files only in its state
/// folder (src/host/sandbox.rs), as warp-lambda hosts programs
const SANDBOX_FLAG: &str = "--sandbox";
/// What the console and `warp <code>` put before a program's value
const RESULT_MARK: &str = "» ";

#[cfg(not(test))]
fn main() {
	let mut args: Vec<String> = env::args().collect();
	warp::errors::crash_card::install();
	end_quietly_on_closed_pipe(); // after the crash card hook, so a closed pipe files none
	warp::graphics::paint::allow_windows();
	warp_runtime::engine::give_main_stack_to_wasm();
	let given = given_arguments(&mut args); // before the flags: `-- --strict` is the program's
	apply_flags(&mut args);
	match args.iter().position(|arg| arg == SANDBOX_FLAG) {
		Some(flag) => {
			args.remove(flag);
			warp::host::sandbox::confine_process();
			warp::compiler::compile_context::scoped(|context| &mut context.sandboxed, true, || run_command(&args, &given))
		}
		None => run_command(&args, &given),
	}
}

/// The words after `--`, taken off the command line: the program's arguments
fn given_arguments(args: &mut Vec<String>) -> Vec<String> {
	let Some(separator) = args.iter().position(|arg| arg == PROGRAM_ARGUMENTS_SEPARATOR) else {
		return vec![];
	};
	let given = args.split_off(separator + 1);
	args.pop();
	given
}

/// A reader that closed the pipe (`warp run x.warp | head -1`) ends warp by SIGPIPE, as it ends cat: Rust ignores the
/// signal, so println! panicked on the broken pipe and filed a crash card (cards crash-show, crashes-broken). That panic
/// raises the signal now. It stays ignored for every other pipe: a closed viewer or interpreter stdin is an error warp
/// handles (paint_window.rs falls back to PNGs), not its end.
#[cfg(all(unix, not(test)))]
fn end_quietly_on_closed_pipe() {
	const SIGPIPE: i32 = 13; // the same on macOS and Linux
	const DEFAULT_ACTION: usize = 0; // SIG_DFL
	const PRINT_FAILED: &str = "failed printing to std"; // print!'s panic on stdout or stderr
	const CLOSED_PIPE: &str = "(os error 32)"; // EPIPE, the same on macOS and Linux
	unsafe extern "C" {
		fn signal(signal: i32, handler: usize) -> usize;
		fn raise(signal: i32) -> i32;
	}
	let next_hook = std::panic::take_hook();
	std::panic::set_hook(Box::new(move |info| {
		let message = info.payload_as_str().unwrap_or_default();
		if message.starts_with(PRINT_FAILED) && message.contains(CLOSED_PIPE) {
			unsafe {
				signal(SIGPIPE, DEFAULT_ACTION);
				raise(SIGPIPE);
			}
		}
		next_hook(info);
	}));
}

#[cfg(all(not(unix), not(test)))]
fn end_quietly_on_closed_pipe() {}

/// `warp [run] prog.warp a b`: whether only to run, the program file and the arguments it gets (`use os; args`)
#[cfg(not(test))]
fn program_file(args: &[String]) -> Option<(bool, &str, &[String])> {
	let only_run = args.get(1).is_some_and(|word| RUN_PREFIX.trim_end() == word);
	let file = if only_run { 2 } else { 1 };
	let path = args.get(file).filter(|path| has_warp_extension(path))?;
	Some((only_run, path.as_str(), &args[file + 1..]))
}

/// `warp [run] a.wasm x y`: the compiled module and the words after it
fn module_file(args: &[String]) -> Option<(&str, &[String])> {
	let file = if args.get(1).is_some_and(|word| RUN_PREFIX.trim_end() == word) { 2 } else { 1 };
	let path = args.get(file).filter(|path| is_module_file(path))?;
	Some((path.as_str(), &args[file + 1..]))
}

/// `a.wasm`, `a.cwasm`: a compiled module
fn is_module_file(word: &str) -> bool {
	word.ends_with(".wasm") || word.ends_with(&format!(".{COMPILED_EXTENSION}"))
}

/// `warp test a.warp b.warp …`: the program files whose tests run
fn test_files(args: &[String]) -> Option<&[String]> {
	let paths = args.get(2..).filter(|paths| !paths.is_empty() && paths.iter().all(|path| has_warp_extension(path)))?;
	args.get(1).is_some_and(|word| TEST_WORD == word).then_some(paths)
}

/// `warp test [--self] [--each] [folder…]`: the folders whose test files run through warp.wasm
/// (src/compiler/pure_tests.rs), tests/ when none is named and it exists; the words after `test`, flags kept
fn test_folders(args: &[String]) -> Option<Vec<String>> {
	use warp::compiler::pure_tests::{DEFAULT_FOLDER, FLAGS};
	if args.get(1).is_none_or(|word| TEST_WORD != word) {
		return None;
	}
	let words = args.get(2..).unwrap_or_default();
	let (flags, folders): (Vec<&String>, Vec<&String>) = words.iter().partition(|word| FLAGS.contains(&word.as_str()));
	if folders.is_empty() {
		let default = std::path::Path::new(DEFAULT_FOLDER).is_dir().then(|| DEFAULT_FOLDER.to_string())?;
		return Some(flags.into_iter().cloned().chain([default]).collect());
	}
	folders.iter().all(|folder| std::path::Path::new(folder).is_dir()).then(|| words.to_vec())
}

/// `warp test --case <pure|self> <file> <index>`: one test of a file, run by a child of `warp test [folder…]`
fn test_case(args: &[String]) -> Option<&[String; 3]> {
	match args {
		[_, word, flag, rest @ ..] if TEST_WORD == word && flag == warp::compiler::pure_tests::CASE_FLAG => {
			rest.try_into().ok()
		}
		_ => None,
	}
}

/// A program file or code, compiled by warp.wasm (the warp compiler) and run: plain `warp <file>`,
/// `warp run`, `warp eval`, piped code and the console run no Rust pipeline (card rust-bootstrap, branch pure). A file's
/// folder is in scope (D15), code's the working directory; the program's arguments are those set for it
fn eval(code: &str) -> Node {
	use warp::compiler::standalone::{run, run_code, Compiler};
	if !code.contains('\n') && has_warp_extension(code) && file_exists(code) {
		let arguments = warp::modules::std_adapters::program_arguments();
		return run(std::path::Path::new(code), arguments, Compiler::RustCompiled);
	}
	run_code(code, &env::current_dir().unwrap_or_default(), Compiler::RustCompiled)
}

// ⚠ WARP DOES THIS DIFFERENTLY: a file's `test` blocks still run through the Rust pipeline here → core (later)
/// The tests of one file run, its summary printed: (tests run, failed)
fn run_tests_of(path: &str) -> (usize, usize) {
	diagnostic::show_lines_of(&source_of(path), Some(path));
	let result = eval(path);
	let summary = match result.drop_meta() {
		Node::Text(summary) => summary.clone(),
		Node::Error(summary) if matches!(summary.drop_meta(), Node::Text(_)) => summary.drop_meta().name(),
		// a program without tests shows its value; one that fails outright counts as one failed test
		other => {
			show(other, "");
			let failed = usize::from(matches!(other, Node::Error(_)));
			return (failed, failed);
		}
	};
	for skipped in warp::compiler::pure_tests::skipped_pairs(&source_of(path)) {
		println!("⊘ skipped {skipped}");
	}
	println!("{summary}");
	test_counts(&summary).unwrap_or((1, 1))
}

/// The error of a command that compiles with the Rust compiler, which `pure` no longer has (card rust-bootstrap):
/// compiler.warp does not do it yet
fn not_in_self_lane(command: &str, target: &str) -> String {
	format!("{command} {target}: not in the self lane yet; this build has no Rust compiler (card rust-bootstrap, branch pure)")
}

/// A program answering requests, `get "/" { … }` or `serve 8080`: served while it runs. Without the Rust compiler's
/// serve analysis a line starting with a route method or `serve` counts
fn serves(code: &str) -> bool {
	code.lines().map(str::trim_start).any(|line| {
		["get \"", "post \"", "put \"", "delete \"", "patch \""].iter().any(|start| line.starts_with(start))
	})
}

/// "✓ 3 tests passed" → (3, 0), "2 of 4 failed" → (4, 2)
fn test_counts(summary: &str) -> Option<(usize, usize)> {
	let numbers: Vec<usize> = summary.split(' ').filter_map(|word| word.parse().ok()).collect();
	match numbers.as_slice() {
		[failed, run] if summary.ends_with(" failed") => Some((*run, *failed)),
		[run] if summary.ends_with(" passed") => Some((*run, 0)),
		_ => None,
	}
}

/// What the command line asks for: a file, a subcommand (`eval`, `compile`, `verify`, `tool` …) or code to evaluate
#[cfg(not(test))]
fn run_command(args: &[String], given: &[String]) {
	// Join args (skip program name)
	let arg_string: String = args.iter().skip(1).cloned().collect::<Vec<_>>().join(" ");
	warp::modules::std_adapters::set_program_arguments(given.to_vec()); // `use os; args`

	if args.len() == 1 {
		// No args, just program name
		if !io::stdin().is_terminal() {
			// Read from stdin pipe
			let mut input = String::new();
			if io::stdin().read_to_string(&mut input).is_ok() && !input.is_empty() {
				if shows_in_window(&input) {
					return show_in_webview(&input, false);
				}
				show_and_fail_on_error(&eval(&input), "");
				return;
			}
		}

		print_version();
		usage();
		console();
		return;
	}

	if args[1] == APP_COMMAND && args.len() >= 3 {
		let check = args[2] == APP_CHECK;
		show_in_webview(&args[if check { 3 } else { 2 }..].join(" "), check);
	} else if is_html_file(&arg_string) {
		show_in_webview(&arg_string, false);
	} else if is_format_command(&args[1]) {
		std::process::exit(format_command(&args[1], &args[2..]));
	} else if let Some(target) = command_argument(&arg_string, "verify") {
		fail(not_in_self_lane("warp verify", target));
	} else if let Some(target) = command_argument(&arg_string, "types") {
		fail(not_in_self_lane("warp types", target));
	} else if args[1] == FETCH_COMMAND {
		fail(not_in_self_lane("warp fetch", &args[2..].join(" ")));
	} else if args[1] == TOOL_COMMAND && args.len() >= 3 {
		fail(not_in_self_lane("warp tool", &args[2]));
	} else if args[1] == warp::graphics::paint_window::COMMAND {
		if let Err(failure) = warp::graphics::paint_window::run(
			args.get(2).is_some_and(|flag| flag == warp::graphics::paint_window::CHECK),
		) {
			fail(failure);
		}
	} else if args[1] == SERVE_COMMAND {
		match served_program(&args[2..]) {
			Ok((path, port)) => serve_file(path, port, "", false),
			Err(failure) => fail(format!("warp serve: {failure}")),
		}
	} else if args[1] == DEPLOY_COMMAND && args.len() >= 3 {
		fail(not_in_self_lane("warp deploy", &args[2..].join(" ")));
	} else if args[1] == DEV_COMMAND && args.len() >= 3 {
		fail(not_in_self_lane("warp dev", &args[2]));
	} else if let Some(target) = command_argument(&arg_string, "data") {
		// `warp data file.warp` prints the file as data
		let text = source_of(target);
		println!("{}", warp::compiler::standalone::parsed(&text).serialize());
	} else if COMPILE_COMMANDS.iter().any(|command| arg_string.starts_with(&format!("{command} "))) {
		let target = extract_after(&arg_string, " ");
		let trim = Trim::of(&target);
		let (flags, target) = leading_flags(target);
		let ahead_of_time = flags.contains(&AOT_FLAG);
		let standalone = flags.contains(&EXE_FLAG) || !(flags.contains(&WASM_FLAG) || ahead_of_time);
		let code = source_of(&target);
		if flags.contains(&SITE_FLAG) {
			return write_site(&code, &target);
		}
		if flags.contains(&PAGE_FLAG) {
			return write_page_module(&code, &target);
		}
		if flags.contains(&WIT_FLAG) {
			return write_wit(&code, &target);
		}
		if flags.contains(&COMPONENT_FLAG) {
			return write_component(&code, &target);
		}
		if flags.contains(&WAGI_FLAG) {
			return write_wagi(&code, &target);
		}
		// `warp build` writes both (card build-should), `--exe` too, `--wasm` and `--aot` only the module
		write_module(&code, &target, trim, ahead_of_time);
		if standalone {
			let written = if builds_an_app(&code) {
				write_web_view_app(&code, &target)
			} else {
				write_standalone_executable(&code, &target)
			};
			match written {
				Ok(report) => println!("{report}"),
				Err(failure) => fail(format!("warp build: {failure}")),
			}
		}
	} else if let Some([by, file, index]) = test_case(args) {
		std::process::exit(if warp::compiler::pure_tests::run_child_case(by, file, index) { 0 } else { 1 });
	} else if let Some(folders) = test_folders(args) {
		std::process::exit(if warp::compiler::pure_tests::run_folders(&folders) { 0 } else { 1 });
	} else if let Some(paths) = test_files(args) {
		// P209, P210: the files' `test` lines and blocks run, failures print ✗ lines, "m of n failed" exits nonzero;
		// several files each print their summary under their name, and the total last
		let several = paths.len() > 1;
		let (mut run, mut failed) = (0, 0);
		for path in paths {
			if several {
				println!("{path}:");
			}
			let (file_run, file_failed) = run_tests_of(path);
			(run, failed) = (run + file_run, failed + file_failed);
		}
		if several {
			let total =
				if failed > 0 { format!("{failed} of {run} failed") } else { format!("✓ {run} tests passed") };
			println!("{total} in {} files", paths.len());
		}
		std::process::exit(if failed > 0 { 1 } else { 0 });
	} else if args[1] == REBUILD_COMMAND {
		rebuild_compilers(args.get(2).map(String::as_str));
	} else if [PURE_COMMAND, SELF_COMMAND].contains(&args[1].as_str()) && args.len() > 2 {
		use warp::compiler::standalone::Compiler;
		let by = if args[1] == SELF_COMMAND { Compiler::SelfCompiled } else { Compiler::RustCompiled };
		if args[1] == PURE_COMMAND {
			eprintln!("{PURE_NOTE}");
		}
		run_by_compiler(&args[2..], by);
	} else if let Some((only_run, path, program_arguments)) = program_file(args) {
		// P105 (user): `warp run <file>` "shall do the opposite": it runs the program and writes no executable
		warp::modules::std_adapters::set_program_arguments([program_arguments, given].concat());
		if !file_exists(path) {
			eprintln!("Error: Could not read file '{}'", path);
		}
		warp_runtime::system_signals::allow_staying(); // a program with a live timer stays after main
		if !only_run && serves(&source_of(path)) {
			serve_file(path, SERVE_PORT, "routes found; ", true);
		}
		// a page shows in a window, as `warp app` shows it; piped or redirected it prints its HTML
		if shows_in_window(&source_of(path)) {
			show_in_webview(path, false);
			return;
		}
		diagnostic::show_lines_of(&source_of(path), Some(path));
		warp::run::program_cache::enable(); // its next run starts without compiling (card cache-compiled-shader)
		let result = eval(path); // a file: its folder is in scope (D15)
		if !only_run && wants_executable() && !matches!(result, Node::Error(_)) {
			leave_executable(path);
		}
		print_and_exit(result);
	} else if arg_string.ends_with(".wat") || arg_string.ends_with(".wast") {
		// Compile WAT/WAST text format to WASM binary, then execute
		let wat_code = load_file(&arg_string);
		print_and_exit(run::wasmtime_runner::run_wat(&wat_code));
	} else if let Some((module, more)) = module_file(args) {
		let words = more.join(" ");
		if !more.is_empty() && more.iter().all(|word| is_module_file(word)) {
			fail(format!(
				"Error: running several wasm files together (linking {module} {words}) is not supported yet; run one file"
			));
		}
		if !more.is_empty() {
			eprintln!(
				"note: got it, {words} are the program's arguments; write them after --: warp {module} -- {words}"
			);
			warp::modules::std_adapters::set_program_arguments([more, given].concat());
		}
		print_and_exit(run::wasmtime_runner::run(module));
	} else if arg_string == "test" || arg_string == "tests" {
		println!(
			"warp test <file.warp> runs the tests of a program, warp test [folder…] the test files under the folders"
		);
	} else if matches!(arg_string.as_str(), "home" | "wiki" | "docs" | "documentation") {
		println!("Warp documentation can be found at https://github.com/pannous/warp/wiki");
		warp::web::open_in_browser("https://github.com/pannous/warp/");
	} else if let Some(code) = command_argument(&arg_string, "eval") {
		diagnostic::show_lines_of(code, None);
		show_and_fail_on_error(&eval(code), RESULT_MARK);
	} else if let Some(code) = command_argument(&arg_string, LOWER_JSON_COMMAND) {
		fail(not_in_self_lane("warp lower --json", code));
	} else if [LOWER_COMMAND, RUNTIME_COMMAND].contains(&args[1].as_str()) && args.len() > 2 {
		// compiler.warp's own commands (core/commands.warp), as `warp self lower …` gives them
		run_by_compiler(&args[1..], warp::compiler::standalone::Compiler::RustCompiled);
	} else if let Some(code) = command_argument(&arg_string, "parse") {
		println!("{}", structure(&warp::compiler::standalone::parsed(code)));
	} else if matches!(arg_string.as_str(), "repl" | "console" | "start" | "run") {
		console();
	} else if matches!(arg_string.as_str(), "2D" | "2d" | "SDL" | "sdl") {
		println!("warp compiled without sdl/webview");
	} else if matches!(arg_string.as_str(), "app" | "webview" | "browser") {
		println!("must compile with WEBAPP support");
		std::process::exit(-1);
	} else if arg_string == "lsp" {
		println!("LSP not yet implemented");
	} else if matches!(arg_string.as_str(), "help" | "--help" | "-h") {
		usage();
		println!("{}", warp::modules::std_docs::modules_overview());
		println!("detailed documentation can be found at https://github.com/pannous/warp/wiki");
	} else if arg_string == HELP_MARKDOWN {
		print!("{}", warp::modules::std_docs::standard_library_markdown());
	} else if let Some(module) = command_argument(&arg_string, HELP_PREFIX) {
		match warp::modules::std_docs::module_help(module) {
			Some(help) => println!("{help}"),
			None => fail(format!("no standard module {module}; {}", warp::modules::std_docs::modules_overview())),
		}
	} else if arg_string == "version" || arg_string == "--version" || arg_string == "-v" {
		print_version();
	} else if let Some(word) = unknown_command(args) {
		fail(word);
	} else if arg_string.contains('/') && std::path::Path::new(&arg_string).is_dir() {
		// a folder is no code: `warp core/runtime/extended` read it as `lib / compiler / …`
		fail(format!("warp: {arg_string} is a folder; run one of its .warp files (warp test <folder> runs its tests)"));
	} else {
		// Default: eval and print; a page shows in a window
		if shows_in_window(&arg_string) {
			return show_in_webview(&arg_string, false);
		}
		show_and_fail_on_error(&eval(&arg_string), RESULT_MARK);
	}
}

/// `🌀 Warp 1.2.4` (its last word the version, tests/common checks it), then on stderr the commit and branch the build
/// was made from and the time of its binary (cards g_oMw8, g_oM-0, version-banner-version):
/// `debug build 75d67b068 on main · built 2026-10-09 14:32`
fn print_version() {
	println!("🌀 Warp {}", WARP_VERSION);
	let profile = if cfg!(debug_assertions) { "debug" } else { "release" };
	let commit = option_env!("WARP_COMMIT").unwrap_or("of an unknown commit");
	let branch = option_env!("WARP_BRANCH").map(|branch| format!(" on {branch}")).unwrap_or_default();
	let binary = env::current_exe().map(|path| path.display().to_string()).unwrap_or_default();
	let built = command_line("date", &["-r", &binary, "+%Y-%m-%d %H:%M"]);
	eprintln!("{profile} build {commit}{branch} · built {}", built.as_deref().unwrap_or("?"));
	eprintln!("{}", warp::compiler::standalone::compilers_line());
}

/// The first line a successful command prints
fn command_line(program: &str, arguments: &[&str]) -> Option<String> {
	let output =
		std::process::Command::new(program).args(arguments).output().ok().filter(|output| output.status.success())?;
	String::from_utf8_lossy(&output.stdout).lines().next().map(str::to_string)
}

/// `warp rebuild [pure|self]`: warp.wasm built anew (plain `warp rebuild` too), or warp.wasm promoted to warp-self.wasm
/// when it passes the fixpoint test (`self`), each with its machine code beside it (<file>.cwasm); status 1 when that
/// failed. `warp rebuild seed`: the seed's machine code only
fn rebuild_compilers(command: Option<&str>) {
	use warp::compiler::standalone::{rebuild, ship_seed, Compiler};
	if command == Some("seed") {
		match ship_seed() {
			Ok(path) => println!("precompiled {}", path.display()),
			Err(failure) => fail(format!("warp rebuild seed: {}", failure.serialize())),
		}
		return;
	}
	let word = command.unwrap_or(Compiler::RustCompiled.command());
	let chosen: Vec<Compiler> = Compiler::ALL.into_iter().filter(|by| word == by.command()).collect();
	if chosen.is_empty() {
		fail(format!(
			"warp rebuild: no compiler {:?}; say `warp rebuild`, `warp rebuild pure`, `warp rebuild self` or `warp rebuild seed`",
			command.unwrap_or_default()
		));
	}
	let mut failed = false;
	for by in chosen {
		match rebuild(by) {
			Ok(path) => println!("built {}", path.display()),
			Err(failure) => {
				eprintln!("warp rebuild {}: {}", by.command(), failure.serialize());
				failed = true;
			}
		}
	}
	std::process::exit(failed as i32)
}

/// The message on stderr and exit status 1
fn fail(message: impl std::fmt::Display) -> ! {
	eprintln!("{message}");
	std::process::exit(1)
}

/// `warp beautify app.warp` before beautify existed ran as the expression `beautify / app.warp` ("type error: int /
/// text", card beautify-cli): a first word warp knows neither as a command nor as a word of the language, followed by
/// an existing file, is a mistyped command. The message names the closest command
fn unknown_command(args: &[String]) -> Option<String> {
	let word = args[1].as_str();
	let plain_word = word.chars().all(|c| c.is_ascii_alphabetic() || c == '-');
	let names_a_file = args[2..].iter().any(|argument| std::path::Path::new(argument).exists());
	let known = COMMAND_WORDS.contains(&word);
	if !plain_word || !names_a_file || known {
		return None;
	}
	let closest = COMMAND_WORDS.iter().min_by_key(|command| extensions::strings::edit_distance(word, command));
	let suggestion = closest
		.filter(|command| extensions::strings::edit_distance(word, command) <= MISTYPED_LETTERS)
		.map(|command| format!(", did you mean {command}?"))
		.unwrap_or_default();
	Some(format!("warp: unknown command {word}{suggestion} (warp help lists the commands)"))
}

/// `warp self [run|eval|build|compile|link] <file.warp|code> [arguments…]` (and the hidden alias `warp pure`): as plain `warp` takes it,
/// the program compiled by compiler.warp (`by`): `run`/`eval` run it, `build` writes its module as `warp build --wasm`
/// does (out.wasm for inline code); with no command word a .warp file or inline code runs. Any other command word is
/// compiler.warp's own command (core/commands.warp: help, version, parse, data, lower, repl; the others refuse
/// naming the native warp), never compiled as code (`warp self build x` compiled `build x`)
#[cfg(not(test))]
fn run_by_compiler(words: &[String], by: warp::compiler::standalone::Compiler) -> ! {
	use warp::compiler::standalone::{compiled, compiled_code, run, run_code};
	let command = words[0].as_str();
	let rest = if words.len() > 1 { &words[1..] } else { words };
	if COMPILE_COMMANDS.contains(&command) && words.len() > 1 {
		let trim = Trim::of(&rest.join(" "));
		let (flags, target) = leading_flags(rest.join(" "));
		let module = if has_warp_extension(&target) && file_exists(&target) {
			diagnostic::show_lines_of(&source_of(&target), Some(&target));
			compiled(std::path::Path::new(&target), by)
		} else {
			diagnostic::show_lines_of(&target, None);
			compiled_code(&target, &env::current_dir().unwrap_or_default(), by)
		};
		let module = module.unwrap_or_else(|failure| print_and_exit(failure));
		write_compiled(module.bytes, &target, trim, flags.contains(&AOT_FLAG));
		if let Some(other) = flags.iter().find(|flag| ![AOT_FLAG, WASM_FLAG].contains(*flag)) {
			let pure = by.command();
			eprintln!("note: warp {pure} build {other}: compiler.warp builds only the module yet; warp build {other} makes it");
		} else if !flags.contains(&WASM_FLAG) && !flags.contains(&AOT_FLAG) {
			eprintln!(
				"note: warp {} build writes the module; warp build makes the standalone executable",
				by.command()
			);
		}
		std::process::exit(0);
	}
	let runs = PURE_RUNNING_WORDS.contains(&command) && words.len() > 1;
	// `warp self <command> …`: the command of compiler.warp itself, written in warp (card wasm-commands); the REPL's
	// lines run here, which plain WASI cannot
	if !runs && COMMAND_WORDS.contains(&command) && !file_exists(command) {
		if command == REPL_COMMAND {
			pure_console(by);
			std::process::exit(0);
		}
		print_and_exit(warp::compiler::standalone::command(words.to_vec(), by));
	}
	let program = if runs { rest } else { words };
	// a .warp file with its arguments, or inline code as plain `warp <code>` takes it, `use` reading from the current
	// folder
	if has_warp_extension(&program[0]) {
		diagnostic::show_lines_of(&source_of(&program[0]), Some(&program[0]));
		print_and_exit(run(std::path::Path::new(&program[0]), program[1..].to_vec(), by));
	}
	let code = program.join(" ");
	diagnostic::show_lines_of(&code, None);
	print_and_exit(run_code(&code, &env::current_dir().unwrap_or_default(), by));
}

/// The text of the file `target` names, else `target` itself as code
fn source_of(target: &str) -> String {
	if file_exists(target) {
		load_file(target)
	} else {
		target.to_string()
	}
}

// Never extend this menu without consulting through the interviewer
fn usage() {
	print_help_rows(HELP_COMMANDS);
	println!("  Options:");
	print_help_rows(HELP_OPTIONS);
}

fn print_help_rows(rows: &[(&str, &str)]) {
	for (usage, meaning) in rows {
		println!("  {usage:<HELP_COLUMN$} {meaning}");
	}
}
