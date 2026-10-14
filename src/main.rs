#![cfg_attr(test, allow(unused))] // main() is not compiled under test
use extensions::numbers::Number;
use node::Node;
use std::env;
use std::fs;
use std::io::{self, IsTerminal, Read, Write};
use warp::node;
use warp::util::{dirs_home, extract_after, file_exists, load_file};
use warp::{
	errors::diagnostic, extensions, law, modules::package_tools, run, run::wasm_reader, util, warp_parser,
	wasm_emitter, web::markup,
};
use warp_runtime::host_words::{FOREIGN_CALL, HOST_LIBRARY};
use wasm_emitter::eval;

const DEFAULT_COMPILED_NAME: &str = "out.wasm";
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
const SITE_SUFFIX: &str = "-site";
/// The site of inline code
const DEFAULT_SITE_NAME: &str = "site";
/// `warp build --wit app.warp`: the WIT world of the program's `component` declaration, app.wit (card wasm-interop-rest)
const WIT_FLAG: &str = "--wit";
const WIT_EXTENSION: &str = "wit";
/// `warp build --component app.warp`: app.component.wasm, the component of that world (src/modules/component_builder.rs)
const COMPONENT_FLAG: &str = "--component";
const COMPONENT_EXTENSION: &str = "component.wasm";
/// `warp build --wagi app.warp`: app.wagi.wasm, its routes answering one WAGI request (lib/wagi.warp), and app.spin.toml
/// to run it: `spin up -f app.spin.toml` (Spin's wagi executor; Akamai Functions: `spin aka deploy`)
const WAGI_FLAG: &str = "--wagi";
const WAGI_EXTENSION: &str = "wagi.wasm";
const SPIN_MANIFEST_EXTENSION: &str = "spin.toml";
const COMPILE_FLAGS: [&str; 7] = [
	EXE_FLAG,
	WASM_FLAG,
	AOT_FLAG,
	SITE_FLAG,
	WIT_FLAG,
	COMPONENT_FLAG,
	WAGI_FLAG,
];
const MACHINE_CODE_EXTENSION: &str = "cwasm";
/// The name of an executable built from inline code (plus the platform's extension)
const DEFAULT_EXECUTABLE_NAME: &str = "out";
/// `warp run <file>`: the program runs, no executable is left (P105)
const RUN_PREFIX: &str = "run ";
const TEST_WORD: &str = "test";
/// The prebuilt runtimes executables are made of: a stub's binary, the variable naming another, its cargo features
struct RuntimeStub {
	name: &'static str,
	variable: &'static str,
	features: &'static str,
}
const PROGRAM_STUB: RuntimeStub = RuntimeStub {
	name: "warp-runtime",
	variable: "WARP_RUNTIME_STUB",
	features: "",
};
// a page app's stub carries the web view (wry, winit), which plain executables do without (card build-links)
const PAGE_STUB: RuntimeStub = RuntimeStub {
	name: "warp-page-runtime",
	variable: "WARP_PAGE_RUNTIME_STUB",
	features: "webview",
};
/// WARP_EXECUTABLE=1 (or true/yes/on): `warp <file>` also leaves the standalone executable next to the file, off by
/// default (card executable-flag, user 2026-10-12, replaces P103's default); `warp build` always makes it
const EXECUTABLE_VARIABLE: &str = "WARP_EXECUTABLE";
/// The crate of the stub in warp's source checkout
const RUNTIME_STUB_CRATE: &str = "crates/warp-runtime";
#[cfg(unix)]
const EXECUTABLE_MODE: u32 = 0o755;
/// Where a failed standalone build is remembered (one file per program path), so its note shows once per version of
/// the program and of warp, not on every run
const STANDALONE_NOTES_CACHE: &str = ".cache/warp/standalone_notes";
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
const SERVE_PORT: u16 = 8080;
/// The program `warp serve` serves when it is given none: the first of these in the current folder
const DEFAULT_PROGRAMS: [&str; 2] = ["app.warp", "main.warp"];
const WARP_VERSION: &str = env!("CARGO_PKG_VERSION");
/// The warnings, notes and hints the user said "got it" to, in the home folder: one `ack:<topic> = acknowledged` per
/// line. A "got it" holds wherever warp runs next (card hints-dismissed)
const ACKNOWLEDGEMENTS_FILE: &str = ".warp-acknowledged";
/// Where earlier versions remembered "got it", in the current folder: their `ack:` lines are adopted
const LOCAL_ACKNOWLEDGEMENTS_FILES: [&str; 2] = [".warp-answers", ACKNOWLEDGEMENTS_FILE];
/// Never prompt "got it?" after a warning or note (as in CI or a pipe)
const NO_ASK_FLAG: &str = "--no-ask";
/// Hints and notes (`prefer ^ over **`) are shown by default; `--no-hints` or WARP_HINTS=0 hide them (card hints-toggle)
const NO_HINTS_FLAG: &str = "--no-hints";
/// Run the program with no C, shell or other runtime (Capability::GRANTED_SANDBOXED), writing files only in its state
/// folder (src/host/sandbox.rs), as warp-lambda hosts programs
const SANDBOX_FLAG: &str = "--sandbox";
const HINTS_VARIABLE: &str = "WARP_HINTS";
/// `--warnings=<level>`: print diagnostics from that level up (hint, weak, normal, severe); `--no-hints` is weak
const WARNINGS_FLAG: &str = "--warnings";
/// `--strict` (normal and severe warnings are errors, as `use strict`), `--strict=<level>` from another level
const STRICT_FLAG: &str = "--strict";
/// The last line of a run that showed hints while nobody chose to see them (neither the flag nor WARP_HINTS was given)
const HIDE_HINTS_TIP: &str = "hide hints with: warp --no-hints (or WARP_HINTS=0)";
/// What the console and `warp <code>` put before a program's value
const RESULT_MARK: &str = "» ";
/// The exit status of a run that ends in an uncaught error (card cli-error-exit)
const ERROR_STATUS: i32 = 1;

/// A program file's exit status: its Int result; a float (a measurement such as elapsed seconds) is no status
fn node_to_i32(node: &Node) -> i32 {
	match node.drop_meta() {
		Node::Number(Number::Int(n)) => *n as i32,
		Node::Error(_) => ERROR_STATUS,
		_ => 0,
	}
}

/// The argument of `warp <command> <argument>`; an assigned command word (`warp 'lower = "abc"; lower'`) starts a
/// program instead (card top-level)
fn command_argument<'a>(arg_string: &'a str, command: &str) -> Option<&'a str> {
	let argument = arg_string.strip_prefix(command.trim_end())?.strip_prefix(' ')?;
	let assigned = ["=", ":="]
		.iter()
		.any(|operator| argument.trim_start().starts_with(operator));
	(!assigned).then_some(argument)
}

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

/// `--fuel <steps>`, `--strict`, `--no-ask`, `--no-hints` take effect and leave the arguments; the answers file is read
#[cfg(not(test))]
fn apply_flags(args: &mut Vec<String>) {
	// `--fuel <steps>`: execution budget of every run (default util::DEFAULT_FUEL, env WARP_FUEL)
	if let Some(flag) = args.iter().position(|arg| arg == "--fuel") {
		match args
			.get(flag + 1)
			.and_then(|steps| steps.replace('_', "").parse::<u64>().ok())
		{
			Some(steps) => util::set_fuel_budget(steps),
			None => {
				eprintln!("--fuel needs a number of steps, e.g. --fuel 100000000000");
				std::process::exit(2);
			}
		}
		args.drain(flag..flag + 2);
	}

	// `--strict[=level]`: warnings of that level and up are errors (as `use strict` in the program)
	if let Some(level) = level_flag(args, STRICT_FLAG, diagnostic::STRICT_LEVEL) {
		diagnostic::set_warning_mode(diagnostic::WarningMode::Strict(level));
	}

	let no_hints = args
		.iter()
		.position(|arg| arg == NO_HINTS_FLAG)
		.map(|flag| args.remove(flag))
		.is_some();
	let hints_variable = env::var(HINTS_VARIABLE).ok();
	let hints_wanted = hints_variable
		.as_deref()
		.is_none_or(|value| !matches!(value, "0" | "false" | "no" | "off"));
	let hints_shown = hints_wanted && !no_hints;
	let fewest_shown = if hints_shown {
		diagnostic::Level::Hint
	} else {
		diagnostic::Level::Weak
	};
	let shown = level_flag(args, WARNINGS_FLAG, fewest_shown).unwrap_or(fewest_shown);
	diagnostic::show_from(shown);
	if shown == diagnostic::Level::Hint && hints_variable.is_none() {
		tip_hiding_hints_at_exit();
	}

	// "got it?" is asked on the terminal after a warning or note unless nobody is there to answer
	let no_ask = args
		.iter()
		.position(|arg| arg == NO_ASK_FLAG)
		.map(|flag| args.remove(flag))
		.is_some();
	if !no_ask && env::var_os("CI").is_none() && io::stdin().is_terminal() && io::stderr().is_terminal() {
		diagnostic::set_acknowledger(Some(std::rc::Rc::new(diagnostic::TerminalAcknowledger)));
	}
	let acknowledgements = dirs_home().join(ACKNOWLEDGEMENTS_FILE);
	for local_file in LOCAL_ACKNOWLEDGEMENTS_FILES {
		diagnostic::adopt_acknowledgements(local_file, &acknowledgements);
	}
	diagnostic::use_acknowledgements_file(acknowledgements);
}

/// The level of `--flag=<level>` or `--flag <level>`, `bare` for the flag alone, taken out of the arguments; exits on an
/// unknown level
#[cfg(not(test))]
fn level_flag(args: &mut Vec<String>, flag: &str, bare: diagnostic::Level) -> Option<diagnostic::Level> {
	let at = args
		.iter()
		.position(|arg| arg == flag || arg.starts_with(&format!("{flag}=")))?;
	let given = match args.remove(at).split_once('=') {
		Some((_, name)) => Some(name.to_string()),
		None => args
			.get(at)
			.filter(|name| diagnostic::Level::named(name).is_some())
			.cloned()
			.inspect(|_| {
				args.remove(at);
			}),
	};
	let Some(name) = given else {
		return Some(bare);
	};
	diagnostic::Level::named(&name).or_else(|| {
		let names: Vec<&str> = diagnostic::Level::ALL.iter().map(|level| level.name()).collect();
		eprintln!("{flag} takes a level: {}", names.join(", "));
		std::process::exit(2);
	})
}

/// After the run (also one ending in process::exit), one line telling how to hide the hints it showed
#[cfg(not(test))]
fn tip_hiding_hints_at_exit() {
	extern "C" fn tip() {
		if diagnostic::any_hint_printed() {
			eprintln!("{}", diagnostic::paint(diagnostic::Color::Gray, HIDE_HINTS_TIP));
		}
	}
	extern "C" {
		fn atexit(callback: extern "C" fn()) -> i32;
	}
	// SAFETY: atexit only stores the function pointer; tip touches no state torn down before exit handlers run
	unsafe { atexit(tip) };
}

/// `warp [run] prog.warp a b`: whether only to run, the program file and the arguments it gets (`use os; args`)
#[cfg(not(test))]
fn program_file(args: &[String]) -> Option<(bool, &str, &[String])> {
	let only_run = args.get(1).is_some_and(|word| RUN_PREFIX.trim_end() == word);
	let file = if only_run { 2 } else { 1 };
	let path = args
		.get(file)
		.filter(|path| path.ends_with(".wasp") || path.ends_with(".warp"))?;
	Some((only_run, path.as_str(), &args[file + 1..]))
}

/// `warp serve [file] [port]`: the program file (app.warp or main.warp of the current folder when none is given) and
/// the port (8080 when none is given)
#[cfg(not(test))]
fn served_program(arguments: &[String]) -> Result<(&str, u16), String> {
	let (ports, files): (Vec<&String>, Vec<&String>) = arguments
		.iter()
		.partition(|argument| argument.chars().all(|digit| digit.is_ascii_digit()));
	let port = match ports.as_slice() {
		[] => SERVE_PORT,
		[port] => port.parse().map_err(|_| format!("{port} is no port"))?,
		several => return Err(format!("one port, not {}", several.len())),
	};
	let file = match files.as_slice() {
		[] => DEFAULT_PROGRAMS
			.iter()
			.find(|file| file_exists(file))
			.copied()
			.ok_or_else(|| format!("no program given and no {} here", DEFAULT_PROGRAMS.join(" or ")))?,
		[file] => file.as_str(),
		several => return Err(format!("one program, not {}", several.len())),
	};
	Ok((file, port))
}

/// The program served at the port until it is stopped; `reason` says why a plain `warp app.warp` serves, which opens
/// its page in the browser from a terminal (`warp serve` doesn't)
#[cfg(not(test))]
fn serve_file(path: &str, port: u16, reason: &str, open: bool) -> ! {
	let code = source_of(path);
	if !warp::serve::serves_itself(&code) {
		println!("serving http://localhost:{port} ({reason}`warp run {path}` runs it once without serving)");
	}
	diagnostic::show_lines_of(&code);
	if open && opens_browser() {
		warp::web::web_server::when_listening(|address| {
			warp::web::dev_server::open_in_browser(address);
		});
	}
	print_and_exit(warp::pipeline::serving_at(port, || eval(path)))
}

/// `warp test file.warp`: the program file whose tests run
fn test_file(args: &[String]) -> Option<&str> {
	let path = args
		.get(2)
		.filter(|path| path.ends_with(".wasp") || path.ends_with(".warp"))?;
	args.get(1)
		.is_some_and(|word| TEST_WORD == word)
		.then_some(path.as_str())
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
	} else if let Some(target) = command_argument(&arg_string, "verify") {
		let code = source_of(target);
		let reports = law::verify(&code);
		reports.iter().for_each(|report| println!("{}", report));
		let all_proofs_check = reports
			.iter()
			.all(|report| report.assurance == law::Assurance::Proved && !report.failed());
		println!(
			"{}",
			if all_proofs_check {
				"ALL PROOFS CHECK"
			} else {
				"SOME PROOFS FAIL"
			}
		);
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
			args.get(2)
				.is_some_and(|flag| flag == warp::graphics::paint_window::CHECK),
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
			fail(format!(
				"warp deploy: which program? warp deploy [{DEV_FLAG}|{DRY_RUN_FLAG}|{HOSTED_FLAG}] app.warp"
			));
		};
		if let Err(failure) = warp::web::deploy::deploy(std::path::Path::new(program), deployment) {
			fail(format!("warp deploy: {failure}"));
		}
	} else if args[1] == DEV_COMMAND && args.len() >= 3 {
		let port = args
			.get(3)
			.map_or(Ok(warp::web::dev_server::DEV_PORT), |port| port.parse::<u16>());
		let served = port
			.map_err(|_| format!("warp dev: the port is a number, not {}", args[3]))
			.and_then(|port| {
				warp::web::dev_server::serve_and_open(std::path::Path::new(&args[2]), port, opens_browser())
			});
		if let Err(failure) = served {
			fail(failure);
		}
	} else if let Some(target) = command_argument(&arg_string, "data") {
		// `warp data file.warp` prints the file as data
		let text = source_of(target);
		println!("{}", warp_parser::parse_data(&text).serialize());
	} else if COMPILE_COMMANDS
		.iter()
		.any(|command| arg_string.starts_with(&format!("{command} ")))
	{
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
		let result = eval(path); // a file: its folder is in scope (D15)
		if !only_run && wants_executable() && !matches!(result, Node::Error(_)) {
			leave_executable(path);
		}
		if let Some(code) = warp_runtime::system_signals::take_exit_code() {
			let _ = io::stdout().flush();
			std::process::exit(code); // `exit(code)` (P121)
		}
		print_and_exit(result);
	} else if arg_string.ends_with(".wat") || arg_string.ends_with(".wast") {
		// Compile WAT/WAST text format to WASM binary, then execute
		let wat_code = load_file(&arg_string);
		print_and_exit(run::wasmtime_runner::run_wat(&wat_code));
	} else if arg_string.ends_with(".wasm") || arg_string.ends_with(&format!(".{MACHINE_CODE_EXTENSION}")) {
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
			None => fail(format!(
				"no standard module {module}; {}",
				warp::modules::std_docs::modules_overview()
			)),
		}
	} else if arg_string == "version" || arg_string == "--version" || arg_string == "-v" {
		print_version();
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
	let branch = option_env!("WARP_BRANCH")
		.map(|branch| format!(" on {branch}"))
		.unwrap_or_default();
	let binary = env::current_exe()
		.map(|path| path.display().to_string())
		.unwrap_or_default();
	let built = command_line("date", &["-r", &binary, "+%Y-%m-%d %H:%M"]);
	eprintln!(
		"{profile} build {commit}{branch} · built {}",
		built.as_deref().unwrap_or("?")
	);
}

/// The first line a successful command prints
fn command_line(program: &str, arguments: &[&str]) -> Option<String> {
	let output = std::process::Command::new(program)
		.args(arguments)
		.output()
		.ok()
		.filter(|output| output.status.success())?;
	String::from_utf8_lossy(&output.stdout)
		.lines()
		.next()
		.map(str::to_string)
}

/// The message on stderr and exit status 1
fn fail(message: impl std::fmt::Display) -> ! {
	eprintln!("{message}");
	std::process::exit(1)
}

/// The program's value printed, its Int the exit status, an uncaught error status 1
fn print_and_exit(result: Node) -> ! {
	show(&result, "");
	ended_well(&result);
	std::process::exit(node_to_i32(&result));
}

/// The value of inline code printed; an uncaught error exits with status 1, so scripts and CI see it failed
fn show_and_fail_on_error(result: &Node, mark: &str) {
	show(result, mark);
	if matches!(result.drop_meta(), Node::Error(_)) {
		std::process::exit(ERROR_STATUS);
	}
	ended_well(result);
}

/// A program that ended well keeps its painted window up until the user closes it; exit() or a failure closes it
fn ended_well(result: &Node) {
	if !matches!(result.drop_meta(), Node::Error(_)) {
		warp::graphics::paint_window::wait_until_closed();
	}
}

/// A program's value after what it printed, behind `mark`; nothing for ø, the value of `print` (issue #18) and of a
/// program that only acts, as Python's console shows nothing for None
fn show(result: &Node, mark: &str) {
	if !result.is_empty_node() {
		// markup (`html{ body{ … } }`) is printed as HTML: `warp run page.warp > page.html` (card web-dom)
		let shown = if markup::is_markup(result) {
			markup::to_html(result)
		} else {
			result.serialize()
		};
		println!("{mark}{shown}");
	}
	if let Some(excerpt) =
		diagnostic::error_position(result).and_then(|(line, column)| diagnostic::shown_excerpt(line, column))
	{
		let _ = io::stdout().flush();
		eprintln!("{excerpt}");
	}
}

/// The text of the file `target` names, else `target` itself as code
fn source_of(target: &str) -> String {
	if file_exists(target) {
		load_file(target)
	} else {
		target.to_string()
	}
}

/// `dir/program.warp` compiles to `dir/program.wasm`; inline code compiles to `out.wasm`
/// `warp build`: the WebAssembly module next to the program file (out.wasm for inline code), with `--aot` its machine
/// code too
fn write_module(code: &str, target: &str, ahead_of_time: bool) {
	match warp::pipeline::for_any_host(|| wasm_emitter::compile(code)) {
		Ok(module) => {
			let output_path = compiled_output_path(target);
			fs::write(&output_path, &module.bytes).expect("could not write the compiled module");
			println!("compiled {} bytes to {}", module.bytes.len(), output_path);
			if ahead_of_time {
				write_machine_code(&module.bytes, &output_path);
			}
		}
		Err(final_value) => fail(format!("nothing to compile: {}", final_value.serialize())),
	}
}

fn compiled_output_path(target: &str) -> String {
	if file_exists(target) {
		std::path::Path::new(target)
			.with_extension("wasm")
			.to_string_lossy()
			.into_owned()
	} else {
		DEFAULT_COMPILED_NAME.to_string()
	}
}

/// `warp compile --aot`: the module compiled to machine code for this machine and wasmtime version, next to the .wasm
fn write_machine_code(bytes: &[u8], wasm_path: &str) {
	let path = std::path::Path::new(wasm_path).with_extension(MACHINE_CODE_EXTENSION);
	let (engine, _) = wasm_reader::engine_for(bytes);
	match engine.precompile_module(bytes) {
		Ok(machine_code) => {
			fs::write(&path, &machine_code).expect("could not write the machine code");
			println!(
				"compiled {} bytes of machine code to {}",
				machine_code.len(),
				path.display()
			);
		}
		Err(failure) => fail(format!("could not compile to machine code: {failure}")),
	}
}

/// The compile flags `arguments` start with, in any order, and the arguments after them
fn leading_flags(mut arguments: String) -> (Vec<&'static str>, String) {
	let mut flags = vec![];
	while let Some(flag) = COMPILE_FLAGS
		.into_iter()
		.find(|flag| arguments.split_whitespace().next() == Some(flag))
	{
		flags.push(flag);
		arguments = arguments[flag.len()..].trim_start().to_string();
	}
	(flags, arguments)
}

fn wants_executable() -> bool {
	env::var(EXECUTABLE_VARIABLE)
		.is_ok_and(|value| matches!(value.to_lowercase().as_str(), "1" | "true" | "yes" | "on"))
}

/// P103 (with WARP_EXECUTABLE): the executable next to a program file that ran, unless one newer than the file is there already; a program
/// the runtime cannot carry (it fetches, say) gets a note on stderr instead
fn leave_executable(path: &str) {
	let modified = |file: &std::path::Path| fs::metadata(file).and_then(|metadata| metadata.modified()).ok();
	let executable = standalone_output_path(path);
	if modified(&executable)
		.zip(modified(std::path::Path::new(path)))
		.is_some_and(|(built, written)| built >= written)
	{
		return;
	}
	let program_file = std::path::Path::new(path);
	let marker = standalone_note_marker(program_file);
	let noted = marker.as_deref().and_then(modified);
	let warp_binary = env::current_exe().ok().and_then(|binary| modified(&binary));
	if noted.is_some_and(|noted| Some(noted) >= modified(program_file) && Some(noted) >= warp_binary) {
		return;
	}
	let written = diagnostic::quietly(|| {
		warp::modules::with_program_file(program_file, || write_standalone_executable(&load_file(path), path))
	});
	if let Err(failure) = written {
		// (said once until the file or warp changes)
		eprintln!("note: no executable {}: {failure}", executable.display());
		if let Some(marker) = marker {
			let _ = marker.parent().map(fs::create_dir_all);
			let _ = fs::write(&marker, failure);
		}
	}
}

/// The file remembering that the standalone build of `program` failed: ~/.cache/warp/standalone_notes/<hash of its path>
fn standalone_note_marker(program: &std::path::Path) -> Option<std::path::PathBuf> {
	use std::hash::{Hash, Hasher};
	let mut hasher = std::collections::hash_map::DefaultHasher::new();
	fs::canonicalize(program).ok()?.hash(&mut hasher);
	Some(
		std::path::PathBuf::from(env::var_os("HOME")?)
			.join(STANDALONE_NOTES_CACHE)
			.join(format!("{:016x}", hasher.finish())),
	)
}

/// The program, printing its value, compiled to machine code and appended to a copy of the compiler-less `warp-runtime`
/// stub (crates/warp-runtime, notes/aot.md); Ok is the report of what was written
fn write_standalone_executable(code: &str, target: &str) -> Result<String, String> {
	let program = wasm_emitter::compile_printing_result(code)
		.map_err(|value| format!("nothing to compile: {}", value.serialize()))?;
	let foreign_call = (HOST_LIBRARY.to_string(), FOREIGN_CALL.to_string());
	let kept_exports = if warp::host::host_parts::imports_of(&program.bytes)?.contains(&foreign_call) {
		warp_runtime::standalone::stub_calls_export_with_foreign_calls
	} else {
		warp_runtime::standalone::stub_calls_export
	};
	let bytes = warp::compiler::dead_functions::keeping_exports(&program.bytes, kept_exports)?;
	let engine = warp_runtime::standalone::standalone_engine().map_err(|failure| failure.to_string())?;
	let module = run::module_cache::compiled_module(&engine, &bytes).map_err(|failure| failure.to_string())?;
	let provided: Vec<(&str, &str)> = warp_runtime::standalone::provided_imports().collect();
	let mut missing: Vec<String> = vec![];
	for import in module
		.imports()
		.filter(|import| !provided.contains(&(import.module(), import.name())))
	{
		let feature = feature_of(import.module(), import.name());
		if !missing.contains(&feature) {
			missing.push(feature);
		}
	}
	if !missing.is_empty() {
		return Err(format!("{} used runtime.", missing.join(", ")));
	}
	let machine_code = module.serialize().map_err(|failure| failure.to_string())?;
	let (stub, runtime) = runtime_stub(&PROGRAM_STUB)?;
	let output = standalone_output_path(target);
	let executable = warp_runtime::standalone::with_machine_code(&runtime, &machine_code);
	write_executable(&executable, &output)?;
	Ok(format!(
		"wrote {} ({} bytes: runtime {}, machine code {})",
		output.display(),
		executable.len(),
		stub.display(),
		machine_code.len()
	))
}

/// `warp build page.warp` of a program showing a page, as `warp page.warp` opens it in a window: an app, the stub
/// carrying the page's site, which it shows in the system's web view (card build-links); macOS only for now
fn builds_an_app(code: &str) -> bool {
	cfg!(target_os = "macos") && warp::serve::shows_page(code)
}

fn write_web_view_app(code: &str, target: &str) -> Result<String, String> {
	let output = standalone_output_path(target);
	let title = output
		.file_stem()
		.map(|stem| stem.to_string_lossy().into_owned())
		.unwrap_or_default();
	let site = warp::web::site::files(code, &title, false)?;
	let (stub, runtime) = runtime_stub(&PAGE_STUB)?;
	let executable = warp_runtime::standalone::with_site(&runtime, &site);
	write_executable(&executable, &output)?;
	Ok(format!(
		"wrote {} ({} bytes: runtime {}, the page in a web view window)",
		output.display(),
		executable.len(),
		stub.display()
	))
}

/// The runtime stub's path and bytes
fn runtime_stub(kind: &RuntimeStub) -> Result<(std::path::PathBuf, Vec<u8>), String> {
	let stub = runtime_stub_path(kind)?;
	let runtime =
		fs::read(&stub).map_err(|failure| format!("cannot read the runtime {}: {failure}", stub.display()))?;
	Ok((stub, runtime))
}

/// The executable written, runnable and (macOS) signed
fn write_executable(executable: &[u8], output: &std::path::Path) -> Result<(), String> {
	fs::write(output, executable).map_err(|failure| format!("cannot write {}: {failure}", output.display()))?;
	#[cfg(unix)]
	{
		use std::os::unix::fs::PermissionsExt;
		fs::set_permissions(output, fs::Permissions::from_mode(EXECUTABLE_MODE))
			.map_err(|failure| failure.to_string())?;
	}
	// macOS starts only signed executables: an ad-hoc signature, which `codesign --verify --strict` accepts
	#[cfg(target_os = "macos")]
	match std::process::Command::new("codesign")
		.args(["--sign", "-", "--force"])
		.arg(output)
		.output()
	{
		Ok(signed) if signed.status.success() => {}
		Ok(signed) => {
			return Err(format!(
				"codesign {} failed: {}",
				output.display(),
				String::from_utf8_lossy(&signed.stderr).trim()
			))
		}
		Err(failure) => return Err(format!("cannot run codesign for {}: {failure}", output.display())),
	}
	Ok(())
}

/// What an import the standalone runtime lacks is for, in the user's words (card g-13Vk: "tasks need runtime."): the
/// host words of tasks, shared arrays and run-time blocks by their feature, any other import as itself. fetch,
/// fetch_within, read, warn and run are imported together (import_manager emit_host_imports): one feature.
fn feature_of(module: &str, name: &str) -> String {
	let feature = match name {
		"fetch" | "fetch_within" | "read" | "warn" | "run" if module == "host" => "fetch and files",
		_ if name.starts_with("task_") || name.starts_with("task·") => "tasks",
		_ if name.starts_with("shared_") => "shared arrays",
		"fetch_start" | "fetch_reply" => "async fetch",
		"run_block" | "block·value" => "run-time blocks",
		"parse_code" => "run-time parse",
		"foreign_call" => "foreign calls",
		_ => return format!("{module}.{name}"),
	};
	feature.to_string()
}

/// `warp build --wit`: the component's WIT world next to the program file (out.wit for inline code)
fn write_wit(code: &str, target: &str) {
	let path = std::path::Path::new(&compiled_output_path(target)).with_extension(WIT_EXTENSION);
	match warp::component_worlds::world_wit(&warp_parser::parse(code)) {
		Ok(wit) => {
			fs::write(&path, wit).expect("could not write the WIT world");
			println!("wrote {}", path.display());
		}
		Err(failure) => fail(format!("warp build --wit: {failure}")),
	}
}

/// `warp build --component`: the component next to the program file (out.component.wasm for inline code)
fn write_component(code: &str, target: &str) {
	let path = std::path::Path::new(&compiled_output_path(target)).with_extension(COMPONENT_EXTENSION);
	match warp::modules::component_builder::build(code) {
		Ok(component) => {
			fs::write(&path, &component).expect("could not write the component");
			println!("wrote {} ({} bytes)", path.display(), component.len());
		}
		Err(failure) => fail(format!("warp build --component: {failure}")),
	}
}

/// `warp build --wagi`: the WAGI module and its Spin manifest next to the program file
fn write_wagi(code: &str, target: &str) {
	let module_path = std::path::Path::new(&compiled_output_path(target)).with_extension(WAGI_EXTENSION);
	let manifest_path = module_path.with_extension("").with_extension(SPIN_MANIFEST_EXTENSION);
	let name = module_path
		.file_stem()
		.and_then(|stem| stem.to_str())
		.and_then(|stem| stem.split('.').next())
		.unwrap_or(DEFAULT_EXECUTABLE_NAME)
		.to_string();
	match warp::pipeline::for_wagi(|| wasm_emitter::compile(code)) {
		Ok(module) => {
			fs::write(&module_path, &module.bytes).expect("could not write the WAGI module");
			let source = module_path.file_name().unwrap_or_default().to_string_lossy();
			fs::write(&manifest_path, warp::web::deploy::spin_manifest(&name, &source))
				.expect("could not write the Spin manifest");
			println!(
				"wrote {} ({} bytes) and {}",
				module_path.display(),
				module.bytes.len(),
				manifest_path.display()
			);
		}
		Err(failure) => {
			eprintln!("warp build --wagi: {failure}");
			std::process::exit(1);
		}
	}
}

/// A program ending in markup (`html{…}`, `div{…}`, card div-opens-app) run with a terminal for its output: shown in a
/// window as `warp app` shows it; piped, redirected or under WARP_NO_WINDOW it prints its HTML
fn shows_in_window(code: &str) -> bool {
	io::stdout().is_terminal() && warp::graphics::paint::shows_windows() && warp::serve::shows_page(code)
}

/// `warp dev` and a served program run from a terminal open their page (cards dev-opens-browser, samples-serving); piped, under WARP_NO_WINDOW or CI it doesn't
fn opens_browser() -> bool {
	io::stdout().is_terminal() && warp::graphics::paint::shows_windows()
}

fn is_html_file(target: &str) -> bool {
	target.ends_with(".html") || target.ends_with(".htm")
}

/// `warp page.html`, `warp app [--check] page.warp`: the page in a web view window (src/web/webview.rs); --check prints
/// its text from a hidden window instead
fn show_in_webview(target: &str, check: bool) {
	if !check && !warp::graphics::paint::shows_windows() {
		fail(format!("warp {APP_COMMAND}: no window under {} or CI; `warp {APP_COMMAND} {APP_CHECK} {target}` prints the page's text", warp::graphics::paint::NO_WINDOW_VARIABLE));
	}
	#[cfg(target_os = "macos")]
	let shown = if is_html_file(target) {
		warp::web::webview::show_file(std::path::Path::new(target), check)
	} else {
		let title = std::path::Path::new(target)
			.file_stem()
			.filter(|_| file_exists(target))
			.map_or(DEFAULT_SITE_NAME.into(), |stem| stem.to_string_lossy());
		warp::web::webview::show_program(&source_of(target), &title, check)
	};
	#[cfg(not(target_os = "macos"))]
	let shown: Result<(), String> = Err(format!(
		"warp is compiled without a web view here (macOS only for now); `warp build --site {target}` writes the page"
	));
	if let Err(failure) = shown {
		fail(format!("warp {APP_COMMAND}: {failure}"));
	}
}

/// `warp build --site`: the site next to the program file, its report or failure
fn write_site(code: &str, target: &str) {
	let file = std::path::Path::new(target);
	let (name, directory) = match file.file_stem().filter(|_| file_exists(target)) {
		Some(stem) => (
			stem.to_string_lossy().into_owned(),
			file.with_file_name(format!("{}{SITE_SUFFIX}", stem.to_string_lossy())),
		),
		None => (
			DEFAULT_SITE_NAME.to_string(),
			std::path::PathBuf::from(DEFAULT_SITE_NAME),
		),
	};
	match warp::web::site::build(code, &name, &directory) {
		Ok(site) => println!(
			"built the site {} ({})",
			site.directory.display(),
			site.files.join(", ")
		),
		Err(failure) => fail(format!("warp build --site: {failure}")),
	}
}

fn standalone_output_path(target: &str) -> std::path::PathBuf {
	if file_exists(target) {
		std::path::Path::new(target).with_extension(env::consts::EXE_EXTENSION)
	} else {
		std::path::Path::new(DEFAULT_EXECUTABLE_NAME).with_extension(env::consts::EXE_EXTENSION)
	}
}

/// The runtime an executable is built from: `WARP_RUNTIME_STUB`, else `warp-runtime` next to this warp (or next to the
/// file a link to warp points to), else built once from warp's own source checkout (issue #10: it just works). P104
/// (user): never a copy of warp itself (~120 MB); without a stub nothing is written
fn runtime_stub_path(kind: &RuntimeStub) -> Result<std::path::PathBuf, String> {
	if let Ok(path) = env::var(kind.variable) {
		return Ok(std::path::PathBuf::from(path));
	}
	let warp = env::current_exe().map_err(|failure| failure.to_string())?;
	let resolved = fs::canonicalize(&warp).unwrap_or_else(|_| warp.clone());
	let next_to_warp = [&warp, &resolved].map(|binary| binary.with_file_name(kind.name));
	let neighbour = next_to_warp.iter().find(|stub| stub.is_file());
	if !in_cargo_target(&resolved) {
		// an installed warp (brew, a release tarball, ~/dev/bin) uses the prebuilt stub shipped next to it and never
		// builds one (user 2026-10-09)
		return neighbour.cloned().ok_or_else(|| {
			format!(
				"no runtime stub {}: {} ships next to warp (brew, the release tarball); put it there or name one in {}",
				next_to_warp[0].display(),
				kind.name,
				kind.variable
			)
		});
	}
	// a debug warp's neighbour in a cargo target is a debug stub (5 MB, it was 21 MB, card g_gFs8): build the release one
	match neighbour.filter(|_| !cfg!(debug_assertions)) {
		Some(stub) => Ok(stub.clone()),
		None => build_runtime_stub(kind, &next_to_warp[0]),
	}
}

/// Whether `binary` lies in a cargo profile folder (debug/, release/, deps/ under them; cargo keeps its .fingerprint
/// there): a development build, which may build its runtime stub from its source checkout
fn in_cargo_target(binary: &std::path::Path) -> bool {
	binary
		.ancestors()
		.skip(1)
		.take(2)
		.any(|folder| folder.join(".fingerprint").is_dir())
}

/// `cargo build --release -p warp-runtime` in warp's source checkout, whatever warp's own profile: the stub lands next
/// to a release warp built there; its path as cargo reports it
fn build_runtime_stub(kind: &RuntimeStub, expected: &std::path::Path) -> Result<std::path::PathBuf, String> {
	let source = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
	if !source.join(RUNTIME_STUB_CRATE).is_dir() {
		return Err(format!("no runtime stub {}: put warp-runtime there or name one in {} (the warp source {} that would build it is gone)", expected.display(), kind.variable, source.display()));
	}
	// cargo runs every time (it keeps the stub current, quickly once built); the note only before the first build
	let release_stub = expected
		.parent()
		.and_then(std::path::Path::parent)
		.map(|target| target.join("release").join(kind.name));
	if !release_stub.is_some_and(|stub| stub.is_file()) {
		eprintln!(
			"note: building the runtime stub {} once (cargo build -p {})",
			kind.name, PROGRAM_STUB.name
		);
	}
	let mut build = std::process::Command::new(env::var("CARGO").unwrap_or_else(|_| "cargo".to_string()));
	build
		.args([
			"build",
			"--release",
			"--quiet",
			"--message-format=json",
			"-p",
			PROGRAM_STUB.name,
			"--bin",
			kind.name,
		])
		.current_dir(source);
	if !kind.features.is_empty() {
		build.args(["--features", kind.features]);
	}
	let output = build
		.output()
		.map_err(|failure| format!("cannot run cargo to build the runtime stub: {failure}"))?;
	if !output.status.success() {
		return Err(format!(
			"building the runtime stub {} failed (cargo build -p {} in {}): {}",
			kind.name,
			PROGRAM_STUB.name,
			source.display(),
			String::from_utf8_lossy(&output.stderr).trim()
		));
	}
	String::from_utf8_lossy(&output.stdout)
		.lines()
		.filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
		.find_map(|message| {
			message["executable"]
				.as_str()
				.filter(|path| path.ends_with(kind.name))
				.map(std::path::PathBuf::from)
		})
		.ok_or_else(|| format!("cargo built {} but named no executable", kind.name))
}

/// Parse tree as s-expression: `(op left right)` for keys, `[items]` for lists
fn structure(node: &Node) -> String {
	match node.drop_meta() {
		Node::Key(left, op, right) => format!("({} {} {})", op, structure(left), structure(right)),
		Node::List(items, bracket, separator) => {
			let inner: Vec<String> = items.iter().map(structure).collect();
			let joiner = match separator.to_char() {
				Some(' ') | None => " ".to_string(),
				Some('\n') => "⏎ ".to_string(),
				Some(other) => format!("{other} "),
			};
			format!("{}{}{}", bracket.opening(), inner.join(&joiner), bracket.closing())
		}
		other => other.serialize(),
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

#[derive(Default)]
struct ConsoleSession {
	state: Vec<String>,
}

impl ConsoleSession {
	fn source_with(&self, input: &str) -> String {
		self.state
			.iter()
			.map(String::as_str)
			.chain([input])
			.collect::<Vec<_>>()
			.join("\n")
	}

	fn remember(&mut self, input: &str, value: &Node) {
		if !matches!(value.drop_meta(), Node::Error(_)) && console_state_line(&warp_parser::parse(input)) {
			self.state.push(input.to_string());
		}
	}
}

fn console_state_line(node: &Node) -> bool {
	match node.drop_meta() {
		Node::Key(_, op, _) => {
			matches!(
				op,
				warp::operators::Op::Assign
					| warp::operators::Op::Define
					| warp::operators::Op::Inc
					| warp::operators::Op::Dec
			) || op.is_compound_assign()
		}
		Node::List(items, node::Bracket::None, separator) if separator.separates_statements() => {
			items.iter().all(console_state_line)
		}
		Node::List(items, node::Bracket::None, node::Separator::Space) => {
			matches!(
				items.first().and_then(Node::symbol_name),
				Some("let" | "var" | "const" | "global")
			) && items.iter().skip(1).any(console_state_line)
		}
		_ => false,
	}
}

fn console() {
	use rustyline::error::ReadlineError;
	use rustyline::DefaultEditor;

	println!("Interactive console (Ctrl+D to exit)");

	let mut rl = match DefaultEditor::new() {
		Ok(editor) => editor,
		Err(e) => {
			eprintln!("Failed to initialize readline: {}", e);
			return;
		}
	};

	// Load history if exists
	let history_path = dirs_home().join(".warp_history");
	let _ = rl.load_history(&history_path);

	let mut interrupted = false; // a ctrl-c on an empty line twice in a row ends the console
	let mut session = ConsoleSession::default();
	loop {
		let read = rl.readline("🌀"); // warp language symbol for prompt
		let interrupted_before = interrupted;
		interrupted = matches!(read, Err(ReadlineError::Interrupted));
		match read {
			Err(ReadlineError::Interrupted) if interrupted_before => break,
			Err(ReadlineError::Interrupted) => {
				println!("^C (again to exit)");
				continue;
			}
			Ok(line) => {
				let input = line.trim();
				if input.is_empty() {
					continue;
				}
				if input == "exit" || input == "quit" {
					break;
				}
				let _ = rl.add_history_entry(input);
				let value = eval(&session.source_with(input));
				session.remember(input, &value);
				show(&value, RESULT_MARK);
			}
			Err(ReadlineError::Eof) => break,
			Err(e) => {
				eprintln!("Error: {}", e);
				break;
			}
		}
	}

	// Save history
	let _ = rl.save_history(&history_path);
}
