use crate::{s, FileExtensions, StringExtensions}; // fucking "".to_string()

use wasm_ast::emitter::emit_binary;
use wasm_ast::model::Module;
use wasm_ast::Instruction::*;
use wasm_ast::NumericInstruction::*;
use wasm_ast::*;

use std::fs::File;
use std::io::prelude::*;
use wasm_ast::ValueType::I32;

// use wasm_bindgen::prelude::*;

// #[wasm_bindgen]
#[allow(improper_ctypes)] // `str`, which is not FFI-safe
extern "C" {
    fn alert(s: &str);
}

// #[wasm_bindgen]
#[no_mangle]
pub fn greet() {
    println!("Hello, WASI-game-of-life!")
    // alert("Hello, wasm-game-of-life!");
}

pub fn build(file_name: &str) {
    let mut builder = Module::builder();
    // builder.add_function("add", |a: i32, b: i32| a + b);
    // builder.add_function("sub", |a: i32, b: i32| a - b);
    let void = ResultType::new(vec![]);
    let int_result = ResultType::new(vec![I32]);

    let main_function_type = FunctionType::new(void.clone(), int_result.clone());
    let _ = builder.add_function_type(main_function_type);
    let main_body = Expression::new(vec![I32Constant(42).into()]);
    let main_func = Function::new(0, void.clone(), main_body);

    let parameters = ResultType::new(vec![I32, I32]);
    let results = ResultType::new(vec![I32]);
    let function_type = FunctionType::new(parameters, results);
    let _ = builder.add_function_type(function_type);
    let locals: ResultType = vec![I32].into();
    let body: Expression = vec![
        42i32.into(),
        // I32Constant(3).into(),
        I32Constant(4).into(),
        Multiply(NumberType::I32).into(),
    ]
    .into();

    let fun = Function::new(1, locals.clone(), body);
    let _ = builder.add_function(main_func);
    let _ = builder.add_function(fun);
    builder.add_export(Export::new(
        Name::new(s!("main")),
        ExportDescription::Function(0),
    ));
    let module = builder.build();

    let mut buffer = Vec::new();
    let size = emit_binary(&module, &mut buffer).unwrap();
    println!("{:?}", size);
    println!("{:?}", buffer);

    let mut file = File::create(file_name).unwrap();
    let _ = file.write_all(&buffer);
    // println!("Wrote to file {}", file.name());
    println!("Wrote to file {} {}", file_name, file.path());

    // module.emit_wasm();
    // module.
    // let module = Module::empty();
}
