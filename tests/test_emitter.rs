use std::env;
// use wasp::Parser;
// use crate::parser::wasp::*;
use wasp;
use wasp::compiler::wasm_reader::*;
use wasp::emitter::*;
use wasp::run::wasmtime_runner::*;
// use wasmtime::*;

#[test]
pub fn test_wasm_parser() {
    let module = parse_wasm("test.wasm");
    println!("module {:#?}", module);
}

#[test]
pub fn test_emitter() {
    greet();
    println!("Current working directory: {:?}", env::current_dir());
    build("test.wasm");
    let result = run_wasm("test.wasm");
    match result {
        Ok(x) => {
            assert_eq!(x, 42);
            println!("OK Result: {:?}", x)
        }
        Err(e) => {
            assert!(false, "Error: {:?}", e)
        }
    }
}
