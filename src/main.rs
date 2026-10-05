#![cfg_attr(test, allow(unused))] // main() is not compiled under test
use warp::{diagnostic, extensions, law, package_tools, run, util, wasm_emitter, wasp_parser};
use warp::node;
use std::env;
use std::fs;
use std::io::{self, Read, IsTerminal};
use node::Node;
use wasm_emitter::eval;
use extensions::numbers::Number;

const DEFAULT_COMPILED_NAME: &str = "out.wasm";
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
        // DONE: don't run, just compile and save binary
        let target = extract_after(&arg_string, " ");
        let code = source_of(&target);
        match wasm_emitter::compile(&code) {
            Ok(module) => {
                let output_path = compiled_output_path(&target);
                fs::write(&output_path, &module.bytes).expect("could not write the compiled module");
                println!("compiled {} bytes to {}", module.bytes.len(), output_path);
            }
            Err(final_value) => {
                eprintln!("nothing to compile: {}", final_value.serialize());
                std::process::exit(1);
            }
        }
    } else if arg_string.ends_with(".wasp") || arg_string.ends_with(".warp") {
        if !file_exists(&arg_string) {
            eprintln!("Error: Could not read file '{}'", arg_string);
        }
        print_and_exit(eval(&arg_string)); // a file: its folder is in scope (D15)
    } else if arg_string.ends_with(".wat") || arg_string.ends_with(".wast") {
        // Compile WAT/WAST text format to WASM binary, then execute
        let wat_code = load_file(&arg_string);
        print_and_exit(run::wasmtime_runner::run_wat(&wat_code));
    } else if arg_string.ends_with(".wasm") {
        if args.len() >= 3 {
            {
                todo!("linking files needs compilation with WABT_MERGE");
            }
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
    println!("  warp <file.warp>     Execute a warp file");
    println!("  warp <file.wasm>     Run a wasm file");
    println!("  warp eval <code>     Evaluate code");
    println!("  warp lower <code>    Show the program after the lowering passes");
    println!("  warp parse <code>    Show the parsed AST");
    println!("  warp verify <file>   Test and prove the laws of a file");
    println!("  warp data <file>     Read untrusted data without evaluating it");
    println!("  warp tool <package> [args]  Run a package's prebuilt <package>.wasm in its directory");
    println!("  warp repl            Start interactive console");
    println!("  --fuel <steps>       Execution budget before 'out of fuel' (env WARP_FUEL)");
    println!("  --no-ask             Never prompt \"got it?\" after a warning or note");
    println!("  warp compile <file|code>  Compile to <file>.wasm (out.wasm for inline code) without running");
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

