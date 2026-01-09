#![allow(dead_code, unused_imports)]
mod extensions;
use extensions::lists::*;
use extensions::numbers::*;
use extensions::strings::*;
use extensions::utils::*;
pub mod node;
pub mod wasm_gc_emitter;
pub mod wasm_gc_reader;
pub mod wasp_parser;
pub mod type_kinds;
pub mod gc_traits;
pub mod analyzer;
pub mod ast;
pub mod meta;
pub mod smarty;
pub mod operators;
pub mod host;

use std::env;
use std::fs;
use std::io::{self, Read, IsTerminal};
use node::Node;
use wasm_gc_emitter::eval;
use extensions::numbers::Number;

const WARP_VERSION: &str = env!("CARGO_PKG_VERSION");

fn node_to_i32(node: &Node) -> i32 {
    match node {
        Node::Number(Number::Int(n)) => *n as i32,
        Node::Number(Number::Float(f)) => *f as i32,
        _ => 0,
    }
}

#[cfg(not(any(feature = "wasm", test)))]
fn main() {
    let args: Vec<String> = env::args().collect();
    let _executable_path = &args[0];

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
    } else if arg_string.ends_with(".wasp") || arg_string.ends_with(".warp") {
        let warp_code = load_file(&arg_string);
        let result = eval(&warp_code);
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
    } else if matches!(arg_string.as_str(), "repl" | "console" | "start" | "run") {
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
    } else if arg_string.contains("compile") || arg_string.contains("build") || arg_string.contains("link") {
        let code = extract_after(&arg_string, " ");
        let _result = eval(&code);
        // TODO: don't run, just compile and save binary
    } else {
        // Default: eval and print
        let result = eval(&arg_string);
        println!("» {}", result.serialize());
    }
}

fn usage() {
    // println!("Usage: warp [options] [file]");
    println!("  warp <file.warp>     Execute a warp file");
    println!("  warp <file.wasm>     Run a wasm file");
    println!("  warp eval <code>     Evaluate code");
    println!("  warp repl            Start interactive console");
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

pub mod run;
