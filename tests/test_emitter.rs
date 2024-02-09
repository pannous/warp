use std::env;
// use wasp::Parser;
// use crate::parser::wasp::*;
use wasp;
use wasp::run::wasmtime_runner::*;
use wasp::emitter::*;
use wasmtime::*;

#[test]
pub fn test_emitter() {
    greet();
    println!("Current working directory: {:?}", env::current_dir());
    build("test.wasm");
    run_wasm("test.wasm");
}
