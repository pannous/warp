#![allow(dead_code, unused_imports)]
mod extensions;
use extensions::lists::*;
use extensions::numbers::*;
use extensions::strings::*;
use extensions::utils::*;
pub mod node;
pub mod context;
pub mod wasm_emitter;
pub mod wasm_reader;
pub mod wasp_parser;
pub mod type_kinds;
pub mod gc_traits;
pub mod analyzer;
pub mod ast;
pub mod meta;
pub mod smarty;
pub mod operators;
pub mod host;
pub mod ffi;
pub mod ffi_parser;
pub mod util;
pub mod function;
pub mod normalize;
pub mod run;
pub mod local;
pub mod law;
pub mod function_equality;
pub mod effects;
pub mod diagnostic;
pub mod injection;
pub mod time;
pub mod real;
pub mod units;
pub mod for_loop;
pub mod type_constructor;
pub mod function_values;
pub mod lambdas;
pub mod library_words;
pub mod type_tests;
pub mod min_max;
pub mod switch;
pub mod phrase_words;
pub mod declarations;
pub mod modules;
pub mod versions;
use std::env;
use std::fs;
use std::io::{self, Read, IsTerminal};
use node::Node;
use wasm_emitter::eval;
use extensions::numbers::Number;

const DEFAULT_COMPILED_NAME: &str = "out.wasm";
const COMPILE_COMMANDS: [&str; 3] = ["compile", "build", "link"];
const WARP_VERSION: &str = env!("CARGO_PKG_VERSION");
/// Answers to the compiler's questions (Asks), remembered per project: one `topic = explicit form` per line
const ANSWERS_FILE: &str = ".wasp-answers";
/// Never ask, every ambiguity falls back to its warning or error (as in CI or a pipe)
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
    let _executable_path = &args[0];
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

    // Ambiguities are asked on the terminal unless nobody is there to answer
    let no_ask = args.iter().position(|arg| arg == NO_ASK_FLAG).map(|flag| args.remove(flag)).is_some();
    if !no_ask && env::var_os("CI").is_none() && io::stdin().is_terminal() && io::stderr().is_terminal() {
        diagnostic::set_asker(Some(std::rc::Rc::new(diagnostic::TerminalAsker)));
    }
    diagnostic::use_answers_file(ANSWERS_FILE);

    // CGI mode detection
    if env::var("SERVER_SOFTWARE").is_ok() {
        println!("Content-Type: text/plain\n");
    }

    // Join args (skip program name)
    let arg_string: String = args.iter().skip(1).cloned().collect::<Vec<_>>().join(" ");

    if args.len() == 1 {
        // No args, just program name
        #[cfg(not(feature = "wasm"))]
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
        #[cfg(feature = "WEBAPP")]
        {
            // start_server in thread, open webview
            let arg = format!("http://localhost:{}/{}", 9999, arg_string);
            println!("Serving {}", arg);
            // open_webview(arg);
        }
        #[cfg(not(feature = "WEBAPP"))]
        println!("warp compiled without webview");
    } else if let Some(target) = arg_string.strip_prefix("verify ") {
        let code = if file_exists(target) { load_file(target) } else { target.to_string() };
        let reports = law::verify(&code);
        reports.iter().for_each(|report| println!("{}", report));
        std::process::exit(reports.iter().any(|report| report.failed()) as i32);
    } else if let Some(target) = arg_string.strip_prefix("data ") {
        let text = if file_exists(target) { load_file(target) } else { target.to_string() };
        println!("{}", wasp_parser::parse_data(&text).serialize());
    } else if COMPILE_COMMANDS.iter().any(|command| arg_string.starts_with(&format!("{command} "))) {
        // DONE: don't run, just compile and save binary
        let target = extract_after(&arg_string, " ");
        let code = if file_exists(&target) { load_file(&target) } else { target.clone() };
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
        let warp_code = load_file(&arg_string);
        let result = eval(&warp_code);
        println!("{}", result.serialize());
        std::process::exit(node_to_i32(&result));
    } else if arg_string.ends_with(".wat") || arg_string.ends_with(".wast") {
        // Compile WAT/WAST text format to WASM binary, then execute
        let wat_code = load_file(&arg_string);
        let result = run::wasmtime_runner::run_wat(&wat_code);
        println!("{}", result.serialize());
        std::process::exit(node_to_i32(&result));
    } else if arg_string.ends_with(".wasm") {
        if args.len() >= 3 {
            #[cfg(any(feature = "WABT_MERGE", feature = "INCLUDE_MERGER"))]
            {
                // merge_files
                todo!("linking files needs compilation with WABT_MERGE");
            }
            #[cfg(not(any(feature = "WABT_MERGE", feature = "INCLUDE_MERGER")))]
            {
                todo!("linking files needs compilation with WABT_MERGE");
            }
        } else {
            let result = run::wasmtime_runner::run(&arg_string);
            println!("{}", result.serialize());
            std::process::exit(node_to_i32(&result));
        }
    } else if arg_string == "test" || arg_string == "tests" {
        #[cfg(not(feature = "release"))]
        {
            println!("Run tests with: cargo test");
        }
        #[cfg(feature = "release")]
        println!("warp release compiled without tests");
    } else if matches!(arg_string.as_str(), "home" | "wiki" | "docs" | "documentation") {
        println!("Wasp documentation can be found at https://github.com/pannous/warp/wiki");
        #[cfg(not(feature = "wasm"))]
        {
            let _ = std::process::Command::new("open")
                .arg("https://github.com/pannous/warp/")
                .spawn();
        }
    } else if arg_string.starts_with("eval ") {
        let code = arg_string.strip_prefix("eval ").unwrap_or("");
        let result = eval(code);
        println!("» {}", result.serialize());
    } else if let Some(code) = arg_string.strip_prefix("parse ") {
        println!("{}", structure(&wasp_parser::parse(code)));
    } else if matches!(arg_string.as_str(), "repl"| "console" | "start" | "run") {
        console();
    } else if matches!(arg_string.as_str(), "2D" | "2d" | "SDL" | "sdl") {
        #[cfg(feature = "GRAFIX")]
        {
            // init_graphics();
        }
        #[cfg(not(feature = "GRAFIX"))]
        println!("warp compiled without sdl/webview");
    } else if matches!(arg_string.as_str(), "app" | "webview" | "browser") {
        #[cfg(feature = "WEBAPP")]
        {
            #[cfg(feature = "GRAFIX")]
            {
                // init_graphics();
            }
            #[cfg(not(feature = "GRAFIX"))]
            println!("warp compiled without sdl/webview");
        }
        #[cfg(not(feature = "WEBAPP"))]
        {
            println!("must compile with WEBAPP support");
            std::process::exit(-1);
        }
    } else if arg_string.starts_with("serv") || arg_string == "server" {
        // CGI/server mode
        println!("Content-Type: text/plain\n");
        let prog = arg_string.strip_prefix("server ").or(arg_string.strip_prefix("serv ")).unwrap_or("");
        let prog = if file_exists(prog) { load_file(prog) } else { prog.to_string() };
        if !prog.is_empty() {
            let result = eval(&prog);
            println!("{}", result.serialize());
        } else {
            println!("Wasp compiled without server OR no program given!");
        }
    } else if arg_string == "lsp" {
        #[cfg(not(feature = "wasm"))]
        {
            // lsp_main();
            println!("LSP not yet implemented");
        }
    } else if arg_string.contains("help") {
        println!("detailed documentation can be found at https://github.com/pannous/warp/wiki");
    } else if arg_string == "version" || arg_string == "--version" || arg_string == "-v" {
        println!("Wasp 🐝 {}", WARP_VERSION);
    } else {
        // Default: eval and print
        let result = eval(&arg_string);
        println!("» {}", result.serialize());
    }
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
        Node::List(items, bracket, _) => {
            let inner: Vec<String> = items.iter().map(structure).collect();
            format!("{}{}", bracket.opening(), inner.join(" "))
        }
        other => other.serialize(),
    }
}

fn usage() {
    // println!("Usage: warp [options] [file]");
    println!("  warp <file.warp>     Execute a warp file");
    println!("  warp <file.wasm>     Run a wasm file");
    println!("  warp eval <code>     Evaluate code");
    println!("  warp parse <code>    Show the parsed AST");
    println!("  warp verify <file>   Test and prove the laws of a file");
    println!("  warp data <file>     Read untrusted data without evaluating it");
    println!("  warp repl            Start interactive console");
    println!("  --fuel <steps>       Execution budget before 'out of fuel' (env WARP_FUEL)");
    println!("  --no-ask             Never ask about ambiguities: take their default with a warning (or fail)");
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

