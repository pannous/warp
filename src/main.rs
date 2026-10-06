#![cfg_attr(test, allow(unused))] // main() is not compiled under test
use warp::{diagnostic, extensions, law, package_tools, run, util, wasm_emitter, wasm_reader, wasp_parser};
use warp::node;
use std::env;
use std::fs;
use std::io::{self, Read, IsTerminal};
use node::Node;
use wasm_emitter::eval;
use extensions::numbers::Number;

const DEFAULT_COMPILED_NAME: &str = "out.wasm";
/// `warp compile --aot` also writes the machine code (wasmtime's .cwasm), which `warp <file>.cwasm` runs
const AOT_FLAG: &str = "--aot";
/// P103 (user): "Just giving it a file will compile it": `warp hello.wasp` runs the program and leaves the standalone
/// executable `hello` next to it (`hello.exe` on Windows), the warp-runtime stub with the program's machine code appended.
/// `warp build <file>` / `warp compile <file>` (not in the help any more) only make it; `--exe` still says so, `--wasm`
/// (and `--aot`) write the module instead
const EXE_FLAG: &str = "--exe";
const WASM_FLAG: &str = "--wasm";
const COMPILE_FLAGS: [&str; 3] = [EXE_FLAG, WASM_FLAG, AOT_FLAG];
const MACHINE_CODE_EXTENSION: &str = "cwasm";
/// The name of an executable built from inline code (plus the platform's extension)
const DEFAULT_EXECUTABLE_NAME: &str = "out";
/// `warp run <file>`: the program runs, no executable is left (P105)
const RUN_PREFIX: &str = "run ";
const RUNTIME_STUB_NAME: &str = "warp-runtime";
const RUNTIME_STUB_VARIABLE: &str = "WARP_RUNTIME_STUB";
#[cfg(unix)]
const EXECUTABLE_MODE: u32 = 0o755;
const COMPILE_COMMANDS: [&str; 3] = ["compile", "build", "link"];
/// `warp tool <package> [arguments…]`: runs the package's prebuilt <package>.wasm (src/package_tools.rs)
const TOOL_COMMAND: &str = "tool";
const WARP_VERSION: &str = env!("CARGO_PKG_VERSION");
/// The warnings and notes the user said "got it" to, remembered per project: one `ack:<topic> = acknowledged` per line
const ACKNOWLEDGEMENTS_FILE: &str = ".wasp-acknowledged";
/// What earlier versions called the acknowledgements file: its `ack:` lines are adopted
const OLD_ANSWERS_FILE: &str = ".wasp-answers";
/// Never prompt "got it?" after a warning or note (as in CI or a pipe)
const NO_ASK_FLAG: &str = "--no-ask";

fn node_to_i32(node: &Node) -> i32 {
    match node {
        Node::Number(Number::Int(n)) => *n as i32,
        Node::Number(Number::Float(f)) => *f as i32,
        _ => 0,
    }
}

#[cfg(not(test))]
fn main() {
    let mut args: Vec<String> = env::args().collect();
    apply_flags(&mut args);
    run_command(&args);
}

/// `--fuel <steps>`, `--strict`, `--no-ask` take effect and leave the arguments; the answers file is read
#[cfg(not(test))]
fn apply_flags(args: &mut Vec<String>) {
    // `--fuel <steps>`: execution budget of every run (default util::DEFAULT_FUEL, env WARP_FUEL)
    if let Some(flag) = args.iter().position(|arg| arg == "--fuel") {
        match args.get(flag + 1).and_then(|steps| steps.replace('_', "").parse::<u64>().ok()) {
            Some(steps) => util::set_fuel_budget(steps),
            None => {
                eprintln!("--fuel needs a number of steps, e.g. --fuel 100000000000");
                std::process::exit(2);
            }
        }
        args.drain(flag..flag + 2);
    }

    // `--strict`: warnings are errors (as `use strict` in the program)
    if let Some(flag) = args.iter().position(|arg| arg == "--strict") {
        diagnostic::set_warning_mode(diagnostic::WarningMode::Error);
        args.remove(flag);
    }

    // "got it?" is asked on the terminal after a warning or note unless nobody is there to answer
    let no_ask = args.iter().position(|arg| arg == NO_ASK_FLAG).map(|flag| args.remove(flag)).is_some();
    if !no_ask && env::var_os("CI").is_none() && io::stdin().is_terminal() && io::stderr().is_terminal() {
        diagnostic::set_acknowledger(Some(std::rc::Rc::new(diagnostic::TerminalAcknowledger)));
    }
    diagnostic::adopt_acknowledgements(OLD_ANSWERS_FILE, ACKNOWLEDGEMENTS_FILE);
    diagnostic::use_acknowledgements_file(ACKNOWLEDGEMENTS_FILE);
}

/// What the command line asks for: a file, a subcommand (`eval`, `compile`, `verify`, `tool` …) or code to evaluate
#[cfg(not(test))]
fn run_command(args: &[String]) {
    // CGI mode detection
    if env::var("SERVER_SOFTWARE").is_ok() {
        println!("Content-Type: text/plain\n");
    }

    // Join args (skip program name)
    let arg_string: String = args.iter().skip(1).cloned().collect::<Vec<_>>().join(" ");

    if args.len() == 1 {
        // No args, just program name
        if !io::stdin().is_terminal() {
            // Read from stdin pipe
            let mut input = String::new();
            if io::stdin().read_to_string(&mut input).is_ok() && !input.is_empty() {
                let result = eval(&input);
                println!("{}", result.serialize());
                return;
            }
        }

        println!("Warp 🐝 {}", WARP_VERSION);
        usage();
        console();
        return;
    }

    if arg_string.ends_with(".html") || arg_string.ends_with(".htm") {
        println!("warp compiled without webview");
    } else if let Some(target) = arg_string.strip_prefix("verify ") {
        let code = source_of(target);
        let reports = law::verify(&code);
        reports.iter().for_each(|report| println!("{}", report));
        std::process::exit(reports.iter().any(|report| report.failed()) as i32);
    } else if args[1] == TOOL_COMMAND && args.len() >= 3 {
        let arguments: Vec<&str> = args[3..].iter().map(String::as_str).collect();
        match package_tools::run_package_tool(&args[2], &arguments) {
            Ok(run) => {
                print!("{}", run.stdout);
                eprint!("{}", run.stderr);
                std::process::exit(run.status);
            }
            Err(failure) => {
                eprintln!("{failure}");
                std::process::exit(1);
            }
        }
    } else if let Some(target) = arg_string.strip_prefix("data ") {
        let text = source_of(target);
        println!("{}", wasp_parser::parse_data(&text).serialize());
    } else if COMPILE_COMMANDS.iter().any(|command| arg_string.starts_with(&format!("{command} "))) {
        let target = extract_after(&arg_string, " ");
        let (flags, target) = leading_flags(target);
        let ahead_of_time = flags.contains(&AOT_FLAG);
        let standalone = flags.contains(&EXE_FLAG) || !(flags.contains(&WASM_FLAG) || ahead_of_time);
        let code = source_of(&target);
        if standalone {
            match write_standalone_executable(&code, &target) {
                Ok(report) => println!("{report}"),
                Err(failure) => {
                    eprintln!("warp build: {failure}");
                    std::process::exit(1);
                }
            }
            return;
        }
        match wasm_emitter::compile(&code) {
            Ok(module) => {
                let output_path = compiled_output_path(&target);
                fs::write(&output_path, &module.bytes).expect("could not write the compiled module");
                println!("compiled {} bytes to {}", module.bytes.len(), output_path);
                if ahead_of_time {
                    write_machine_code(&module.bytes, &output_path);
                }
            }
            Err(final_value) => {
                eprintln!("nothing to compile: {}", final_value.serialize());
                std::process::exit(1);
            }
        }
    } else if arg_string.ends_with(".wasp") || arg_string.ends_with(".warp") {
        // P105 (user): `warp run <file>` "shall do the opposite": it runs the program and writes no executable
        let (only_run, path) = match arg_string.strip_prefix(RUN_PREFIX) {
            Some(path) => (true, path),
            None => (false, arg_string.as_str()),
        };
        if !file_exists(path) {
            eprintln!("Error: Could not read file '{}'", path);
        }
        let result = eval(path); // a file: its folder is in scope (D15)
        if !only_run && !matches!(result, Node::Error(_)) {
            leave_executable(path);
        }
        print_and_exit(result);
    } else if arg_string.ends_with(".wat") || arg_string.ends_with(".wast") {
        // Compile WAT/WAST text format to WASM binary, then execute
        let wat_code = load_file(&arg_string);
        print_and_exit(run::wasmtime_runner::run_wat(&wat_code));
    } else if arg_string.ends_with(".wasm") || arg_string.ends_with(&format!(".{MACHINE_CODE_EXTENSION}")) {
        if args.len() >= 3 {
            eprintln!("Error: running several wasm files together (linking {}) is not supported yet; run one file", args[1..].join(" "));
            std::process::exit(1);
        } else {
            print_and_exit(run::wasmtime_runner::run(&arg_string));
        }
    } else if arg_string == "test" || arg_string == "tests" {
        {
            println!("Run tests with: cargo test");
        }
    } else if matches!(arg_string.as_str(), "home" | "wiki" | "docs" | "documentation") {
        println!("Wasp documentation can be found at https://github.com/pannous/warp/wiki");
        {
            let _ = std::process::Command::new("open")
                .arg("https://github.com/pannous/warp/")
                .spawn();
        }
    } else if arg_string.starts_with("eval ") {
        let code = arg_string.strip_prefix("eval ").unwrap_or("");
        let result = eval(code);
        println!("» {}", result.serialize());
    } else if let Some(code) = arg_string.strip_prefix("lower ") {
        match wasm_emitter::lower(code) {
            Ok(lowered) => println!("{}", lowered.serialize()),
            Err(final_value) => println!("no module needed: {}", final_value.serialize()),
        }
    } else if let Some(code) = arg_string.strip_prefix("parse ") {
        println!("{}", structure(&wasp_parser::parse(code)));
    } else if matches!(arg_string.as_str(), "repl"| "console" | "start" | "run") {
        console();
    } else if matches!(arg_string.as_str(), "2D" | "2d" | "SDL" | "sdl") {
        println!("warp compiled without sdl/webview");
    } else if matches!(arg_string.as_str(), "app" | "webview" | "browser") {
        {
            println!("must compile with WEBAPP support");
            std::process::exit(-1);
        }
    } else if arg_string.starts_with("serv") || arg_string == "server" {
        // CGI/server mode
        println!("Content-Type: text/plain\n");
        let prog = arg_string.strip_prefix("server ").or(arg_string.strip_prefix("serv ")).unwrap_or("");
        let prog = source_of(prog);
        if !prog.is_empty() {
            let result = eval(&prog);
            println!("{}", result.serialize());
        } else {
            println!("Wasp compiled without server OR no program given!");
        }
    } else if arg_string == "lsp" {
        {
            // lsp_main();
            println!("LSP not yet implemented");
        }
    } else if matches!(arg_string.as_str(), "help" | "--help" | "-h") {
        usage();
        println!("detailed documentation can be found at https://github.com/pannous/warp/wiki");
    } else if arg_string == "version" || arg_string == "--version" || arg_string == "-v" {
        println!("Wasp 🐝 {}", WARP_VERSION);
    } else {
        // Default: eval and print
        let result = eval(&arg_string);
        println!("» {}", result.serialize());
    }
}

/// The program's value printed, its Int the exit status
fn print_and_exit(result: Node) -> ! {
    println!("{}", result.serialize());
    std::process::exit(node_to_i32(&result));
}

/// The text of the file `target` names, else `target` itself as code
fn source_of(target: &str) -> String {
    if file_exists(target) { load_file(target) } else { target.to_string() }
}

/// `dir/program.warp` compiles to `dir/program.wasm`; inline code compiles to `out.wasm`
fn compiled_output_path(target: &str) -> String {
    if file_exists(target) {
        std::path::Path::new(target).with_extension("wasm").to_string_lossy().into_owned()
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
            println!("compiled {} bytes of machine code to {}", machine_code.len(), path.display());
        }
        Err(failure) => {
            eprintln!("could not compile to machine code: {failure}");
            std::process::exit(1);
        }
    }
}

/// The compile flags `arguments` start with, in any order, and the arguments after them
fn leading_flags(mut arguments: String) -> (Vec<&'static str>, String) {
    let mut flags = vec![];
    while let Some(flag) = COMPILE_FLAGS.into_iter().find(|flag| arguments.split_whitespace().next() == Some(flag)) {
        flags.push(flag);
        arguments = arguments[flag.len()..].trim_start().to_string();
    }
    (flags, arguments)
}

/// P103: the executable next to a program file that ran, unless one newer than the file is there already; a program
/// the runtime cannot carry (it fetches, say) gets a note on stderr instead
fn leave_executable(path: &str) {
    let modified = |file: &std::path::Path| fs::metadata(file).and_then(|metadata| metadata.modified()).ok();
    let executable = standalone_output_path(path);
    if modified(&executable).zip(modified(std::path::Path::new(path))).is_some_and(|(built, written)| built >= written) {
        return;
    }
    let program_file = std::path::Path::new(path);
    if let Err(failure) = warp::modules::with_program_file(program_file, || write_standalone_executable(&load_file(path), path)) {
        eprintln!("note: no executable {}: {failure}", executable.display());
    }
}

/// The program, printing its value, compiled to machine code and appended to a copy of the compiler-less `warp-runtime`
/// stub (crates/warp-runtime, notes/aot.md); Ok is the report of what was written
fn write_standalone_executable(code: &str, target: &str) -> Result<String, String> {
    let program = wasm_emitter::compile_printing_result(code).map_err(|value| format!("nothing to compile: {}", value.serialize()))?;
    let engine = util::gc_engine();
    let module = run::module_cache::compiled_module(&engine, &program.bytes).map_err(|failure| failure.to_string())?;
    let provided: Vec<(&str, &str)> = warp_runtime::standalone::provided_imports().collect();
    let missing: Vec<String> = module.imports().filter(|import| !provided.contains(&(import.module(), import.name()))).map(|import| format!("{}.{}", import.module(), import.name())).collect();
    if !missing.is_empty() {
        return Err(format!("a standalone executable provides print, libm, sleep, random, random_below and clock, not {}: run the program with warp", missing.join(", ")));
    }
    let machine_code = module.serialize().map_err(|failure| failure.to_string())?;
    let stub = runtime_stub_path()?;
    let runtime = fs::read(&stub).map_err(|failure| format!("cannot read the runtime {}: {failure}", stub.display()))?;
    let output = standalone_output_path(target);
    let executable = warp_runtime::standalone::with_machine_code(&runtime, &machine_code);
    fs::write(&output, &executable).map_err(|failure| format!("cannot write {}: {failure}", output.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&output, fs::Permissions::from_mode(EXECUTABLE_MODE)).map_err(|failure| failure.to_string())?;
    }
    // macOS starts only signed executables: an ad-hoc signature, which `codesign --verify --strict` accepts
    #[cfg(target_os = "macos")]
    match std::process::Command::new("codesign").args(["--sign", "-", "--force"]).arg(&output).output() {
        Ok(signed) if signed.status.success() => {}
        Ok(signed) => return Err(format!("codesign {} failed: {}", output.display(), String::from_utf8_lossy(&signed.stderr).trim())),
        Err(failure) => return Err(format!("cannot run codesign for {}: {failure}", output.display())),
    }
    Ok(format!("wrote {} ({} bytes: runtime {}, machine code {})", output.display(), executable.len(), stub.display(), machine_code.len()))
}

fn standalone_output_path(target: &str) -> std::path::PathBuf {
    if file_exists(target) {
        std::path::Path::new(target).with_extension(env::consts::EXE_EXTENSION)
    } else {
        std::path::Path::new(DEFAULT_EXECUTABLE_NAME).with_extension(env::consts::EXE_EXTENSION)
    }
}

/// The runtime an executable is built from: `WARP_RUNTIME_STUB`, else `warp-runtime` next to this warp. P104 (user):
/// never a copy of warp itself (~120 MB); without a stub nothing is written
fn runtime_stub_path() -> Result<std::path::PathBuf, String> {
    if let Ok(path) = env::var(RUNTIME_STUB_VARIABLE) {
        return Ok(std::path::PathBuf::from(path));
    }
    let warp = env::current_exe().map_err(|failure| failure.to_string())?;
    let stub = warp.with_file_name(RUNTIME_STUB_NAME);
    match stub.is_file() {
        true => Ok(stub),
        false => Err(format!("no runtime stub {}: build it with `cargo build --release -p warp-runtime`, or name one in {RUNTIME_STUB_VARIABLE}", stub.display())),
    }
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
    println!("  warp <file.warp>     Run a warp file and leave its standalone executable <file> next to it");
    println!("  warp run <file.warp> Run a warp file, no executable");
    println!("  warp <file.wasm>     Run a wasm file (or a .cwasm from compile --aot)");
    println!("  warp eval <code>     Evaluate code");
    println!("  warp lower <code>    Show the program after the lowering passes");
    println!("  warp parse <code>    Show the parsed AST");
    println!("  warp verify <file>   Test and prove the laws of a file");
    println!("  warp data <file>     Read untrusted data without evaluating it");
    println!("  warp tool <package> [args]  Run a package's prebuilt <package>.wasm in its directory");
    println!("  warp repl            Start interactive console");
    println!("  --fuel <steps>       Execution budget before 'out of fuel' (env WARP_FUEL)");
    println!("  --no-ask             Never prompt \"got it?\" after a warning or note");
    println!("  warp compile --wasm <file|code>  Only the module, <file>.wasm (out.wasm for inline code), without running");
    println!("  warp compile --aot <file|code>   The module and its machine code for this machine, <file>.cwasm");
    println!("  warp test            Run tests");
    println!("  warp docs            Open documentation");
    println!("  warp version         Show version");
    println!("  warp help            Show this help");
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

    loop {
        match rl.readline("🐝 ") {
            Ok(line) => {
                let input = line.trim();
                if input.is_empty() { continue; }
                if input == "exit" || input == "quit" { break; }
                let _ = rl.add_history_entry(input);
                let result = eval(input);
                println!("» {}", result.serialize());
            }
            Err(ReadlineError::Interrupted) => {
                println!("^C");
                continue;
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

fn dirs_home() -> std::path::PathBuf {
    env::var("HOME").map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::PathBuf::from("."))
}

fn load_file(path: &str) -> String {
    fs::read_to_string(path).unwrap_or_else(|_| {
        eprintln!("Error: Could not read file '{}'", path);
        String::new()
    })
}

fn file_exists(path: &str) -> bool {
    std::path::Path::new(path).exists()
}

fn extract_after(s: &str, sep: &str) -> String {
    s.split_once(sep).map(|(_, after)| after.to_string()).unwrap_or_default()
}

