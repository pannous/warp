#![cfg_attr(test, allow(unused))] // main() is not compiled under test
use extensions::numbers::Number;
use node::Node;
use std::env;
use std::fs;
use std::io::{self, IsTerminal, Read, Write};
use warp::node;
use warp::run::module_cache::COMPILED_EXTENSION;
use warp::util::{dirs_home, extract_after, file_exists, load_file};
use warp::{
	errors::diagnostic, extensions, law, modules::package_tools, run, run::wasm_reader, util, warp_parser,
	wasm_emitter, web::markup,
};
use warp_runtime::host_words::{FOREIGN_CALL, HOST_LIBRARY};
use wasm_emitter::eval;

mod cli;
use cli::*;

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
/// The site of inline code
const DEFAULT_SITE_NAME: &str = "site";
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
/// `warp dev <file> [port]`: serves the file's site and builds it anew when it changes (src/web/dev_server.rs, card web-dev)
const DEV_COMMAND: &str = "dev";
/// `warp app page.warp`: the program's page in a window (src/web/webview.rs, card read-chron)
const APP_COMMAND: &str = "app";
const APP_CHECK: &str = "--check";
/// `warp deploy [--dev|--dry-run] app.warp`: the program as a Cloudflare Worker, deployed by wrangler (src/web/deploy.rs)
const DEPLOY_COMMAND: &str = "deploy";
/// `warp serve [app.warp] [port]`: serves the program, its page, server functions and routes (P222)
const SERVE_COMMAND: &str = "serve";
/// The first words the command line takes as commands, for unknown_command's suggestion
const COMMAND_WORDS: [&str; 30] = [
	"run", "test", "eval", "lower", "parse", "verify", "format", "lint", "beautify", "fix", "types", "data", "tool",
	"dev", "serve", "deploy", "repl", "console", "register", "compile", "build", "link", "app", "help", "version",
	"docs", "wiki", "home", "lsp", "start",
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
	apply_flags(&mut args);
	match args.iter().position(|arg| arg == SANDBOX_FLAG) {
		Some(flag) => {
			args.remove(flag);
			warp::host::sandbox::confine_process();
			warp::pipeline::sandboxed(|| run_command(&args))
		}
		None => run_command(&args),
	}
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

/// `warp test file.warp`: the program file whose tests run
fn test_file(args: &[String]) -> Option<&str> {
	let path = args.get(2).filter(|path| has_warp_extension(path))?;
	args.get(1).is_some_and(|word| TEST_WORD == word).then_some(path.as_str())
}

/// What the command line asks for: a file, a subcommand (`eval`, `compile`, `verify`, `tool` …) or code to evaluate
#[cfg(not(test))]
fn run_command(args: &[String]) {
	// Join args (skip program name)
	let arg_string: String = args.iter().skip(1).cloned().collect::<Vec<_>>().join(" ");

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
		let code = source_of(target);
		let reports = law::verify(&code);
		reports.iter().for_each(|report| println!("{}", report));
		let all_proofs_check =
			reports.iter().all(|report| report.assurance == law::Assurance::Proved && !report.failed());
		println!("{}", if all_proofs_check { "ALL PROOFS CHECK" } else { "SOME PROOFS FAIL" });
		std::process::exit((!all_proofs_check) as i32);
	} else if let Some(target) = command_argument(&arg_string, "types") {
		println!("{}", law::type_model::report(&source_of(target)));
	} else if args[1] == TOOL_COMMAND && args.len() >= 3 {
		let arguments: Vec<&str> = args[3..].iter().map(String::as_str).collect();
		match package_tools::run_package_tool(&args[2], &arguments) {
			Ok(run) => {
				print!("{}", run.stdout);
				eprint!("{}", run.stderr);
				std::process::exit(run.status);
			}
			Err(failure) => fail(failure),
		}
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
		use warp::web::deploy::{Deployment, DEV_FLAG, DRY_RUN_FLAG, HOSTED_FLAG};
		let deployment = match args[2].as_str() {
			DEV_FLAG => Deployment::Local,
			DRY_RUN_FLAG => Deployment::DryRun,
			HOSTED_FLAG => Deployment::Hosted,
			_ => Deployment::Cloud,
		};
		let Some(program) = args.get(if deployment == Deployment::Cloud { 2 } else { 3 }) else {
			fail(format!("warp deploy: which program? warp deploy [{DEV_FLAG}|{DRY_RUN_FLAG}|{HOSTED_FLAG}] app.warp"));
		};
		if let Err(failure) = warp::web::deploy::deploy(std::path::Path::new(program), deployment) {
			fail(format!("warp deploy: {failure}"));
		}
	} else if args[1] == DEV_COMMAND && args.len() >= 3 {
		let port = args.get(3).map_or(Ok(warp::web::dev_server::DEV_PORT), |port| port.parse::<u16>());
		let served = port.map_err(|_| format!("warp dev: the port is a number, not {}", args[3])).and_then(|port| {
			warp::web::dev_server::serve_and_open(std::path::Path::new(&args[2]), port, opens_browser())
		});
		if let Err(failure) = served {
			fail(failure);
		}
	} else if let Some(target) = command_argument(&arg_string, "data") {
		// `warp data file.warp` prints the file as data
		let text = source_of(target);
		println!("{}", warp_parser::parse_data(&text).serialize());
	} else if COMPILE_COMMANDS.iter().any(|command| arg_string.starts_with(&format!("{command} "))) {
		let target = extract_after(&arg_string, " ");
		let (flags, target) = leading_flags(target);
		let ahead_of_time = flags.contains(&AOT_FLAG);
		let standalone = flags.contains(&EXE_FLAG) || !(flags.contains(&WASM_FLAG) || ahead_of_time);
		let code = source_of(&target);
		if flags.contains(&SITE_FLAG) {
			return write_site(&code, &target);
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
		write_module(&code, &target, ahead_of_time);
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
	} else if let Some(path) = test_file(args) {
		// P209, P210: the file's `test` lines and blocks run, failures print ✗ lines, "m of n failed" exits nonzero
		diagnostic::show_lines_of(&source_of(path));
		let result = warp::pipeline::for_tests(|| eval(path));
		let failed = matches!(result, Node::Error(_));
		match result.drop_meta() {
			Node::Text(summary) => println!("{summary}"),
			Node::Error(summary) if matches!(summary.drop_meta(), Node::Text(_)) => {
				println!("{}", summary.drop_meta())
			}
			other => show(other, ""),
		}
		std::process::exit(if failed { 1 } else { 0 });
	} else if let Some((only_run, path, program_arguments)) = program_file(args) {
		// P105 (user): `warp run <file>` "shall do the opposite": it runs the program and writes no executable
		warp::modules::std_adapters::set_program_arguments(program_arguments.to_vec()); // `use os; args`
		if !file_exists(path) {
			eprintln!("Error: Could not read file '{}'", path);
		}
		warp_runtime::system_signals::allow_staying(); // a program with a live timer stays after main
		if !only_run && warp::serve::serves(&source_of(path)) {
			serve_file(path, SERVE_PORT, "routes found; ", true);
		}
		// a page shows in a window, as `warp app` shows it; piped or redirected it prints its HTML
		if shows_in_window(&source_of(path)) {
			show_in_webview(path, false);
			return;
		}
		diagnostic::show_lines_of(&source_of(path));
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
	} else if arg_string.ends_with(".wasm") || arg_string.ends_with(&format!(".{COMPILED_EXTENSION}")) {
		if args.len() >= 3 {
			fail(format!(
				"Error: running several wasm files together (linking {}) is not supported yet; run one file",
				args[1..].join(" ")
			));
		} else {
			print_and_exit(run::wasmtime_runner::run(&arg_string));
		}
	} else if arg_string == "test" || arg_string == "tests" {
		println!("warp test <file.warp> runs the tests of a program; warp's own tests run with: cargo test");
	} else if matches!(arg_string.as_str(), "home" | "wiki" | "docs" | "documentation") {
		println!("Warp documentation can be found at https://github.com/pannous/warp/wiki");
		warp::web::dev_server::open_in_browser("https://github.com/pannous/warp/");
	} else if let Some(code) = command_argument(&arg_string, "eval") {
		diagnostic::show_lines_of(code);
		show_and_fail_on_error(&eval(code), RESULT_MARK);
	} else if let Some(code) = command_argument(&arg_string, "lower") {
		match wasm_emitter::lower(code) {
			Ok(lowered) => println!("{}", lowered.serialize()),
			Err(final_value) => println!("no module needed: {}", final_value.serialize()),
		}
	} else if let Some(code) = command_argument(&arg_string, "parse") {
		println!("{}", structure(&warp_parser::parse(code)));
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
}

/// The first line a successful command prints
fn command_line(program: &str, arguments: &[&str]) -> Option<String> {
	let output =
		std::process::Command::new(program).args(arguments).output().ok().filter(|output| output.status.success())?;
	String::from_utf8_lossy(&output.stdout).lines().next().map(str::to_string)
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
	let known = COMMAND_WORDS.contains(&word) || warp::lowering::library_words::is_calling_word(word);
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

/// The text of the file `target` names, else `target` itself as code
fn source_of(target: &str) -> String {
	if file_exists(target) {
		load_file(target)
	} else {
		target.to_string()
	}
}

fn usage() {
	// println!("Usage: warp [options] [file]");
	println!("  warp <file.warp>     Run a warp file (env WARP_EXECUTABLE=1: also leave its standalone executable <file> next to it)");
	println!("  warp run <file.warp> Run a warp file, never an executable");
	println!("  warp <file.wasm>     Run a wasm file (or a .cwasm from compile --aot)");
	println!("  warp eval <code>     Evaluate code");
	println!("  warp lower <code>    Show the program after the lowering passes");
	println!("  warp parse <code>    Show the parsed AST");
	println!("  warp verify <file>   Test and prove the laws of a file");
	println!("  {FORMAT_USAGE}");
	println!("  warp types <file>    The program in W0, the proved model of warp's types, and both verdicts");
	println!("  warp data <file>     Read untrusted data without evaluating it");
	println!("  warp tool <package> [args]  Run a package's prebuilt <package>.wasm in its directory");
	println!("  warp dev <file> [port]  Serve the file's page, reloaded when it changes (port 8008)");
	println!(
		"  warp serve [file] [port]  Serve the program: its page, server functions and routes (app.warp, port 8080)"
	);
	println!("  warp deploy [--dev|--dry-run|--hosted] <file>  The program's routes as a Cloudflare Worker, by wrangler (--dev: on this machine, --dry-run: only <file>-worker/, --hosted: at warp-<file>.pannous.workers.dev, by your GitHub login)");
	println!("  warp repl            Start interactive console");
	println!("  warp register        Let Finder and `open` run .warp files (macOS)");
	println!("  --fuel <steps>       Execution budget before 'out of fuel' (env WARP_FUEL)");
	println!("  warp fast <file|code> sin cos exp pow atan2 as quick wasm polynomials, as `use fast_math` (errors: lib/fast_math.warp)");
	println!("  --no-ask             Never prompt \"got it?\" after a warning or note");
	println!("  --no-hints           Hide hints and notes (prefer ^ over **), shown by default (env WARP_HINTS=0)");
	println!("  --warnings=<level>   Show diagnostics from hint (default), weak, normal or severe up");
	println!("  --strict[=<level>]   Warnings from normal (or the level) up are errors, as `use strict`");
	println!("  --sandbox            No C, shell or other runtime, files written only in the working folder (as hosted programs run)");
	println!("  The last compiled module is kept in ~/.cache/warp/last.wasm for inspection");
	println!(
		"  warp compile --wasm <file|code>  Only the module, <file>.wasm (out.wasm for inline code), without running"
	);
	println!("  warp compile --aot <file|code>   The module and its machine code for this machine, <file>.cwasm");
	println!("  warp build <file.warp>  The standalone executable <file> and <file>.wasm (--exe: only the executable, --wasm: only the module)");
	println!("  warp {APP_COMMAND} <file.warp>    Show the program's page in a window of its own (macOS web view; {APP_CHECK}: print its text)");
	println!("  warp test            Run tests");
	println!("  warp docs            Open documentation");
	println!("  warp version         Show version");
	println!("  warp help            Show this help");
	println!(
		"  warp help <module>   A standard module's words (warp help --markdown: all, as wiki/standard-library.md)"
	);
}
