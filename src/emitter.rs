use crate::{FileExtensions, s, StringExtensions}; // fucking "".to_string()

use wasm_ast::*;
use wasm_ast::model::Module;
use wasm_ast::emitter::emit_binary;
use wasm_ast::Instruction::*;
use wasm_ast::NumericInstruction::*;

use std::fs::File;
use std::io::prelude::*;

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

    let parameters = ResultType::new(vec![ValueType::I32, ValueType::I32]);
    let results = ResultType::new(vec![ValueType::I32]);
    let function_type = FunctionType::new(parameters, results);
    let _ = builder.add_function_type(function_type);
    let kind: TypeIndex = 0u32.into();
    let locals: ResultType = vec![ValueType::I32].into();
    let body: Expression = vec![
        42i32.into(),
        // I32Constant(3).into(),
        I32Constant(4).into(),
        Multiply(NumberType::I32).into(),
    ].into();

    assert_eq!(
        Numeric(I32Constant(42)),
        42i32.into()
    );
    assert_eq!(
        Instruction::Numeric(NumericInstruction::I64Constant(42i64)),
        42i64.into()
    );

    let fun = Function::new(kind.into(), locals.clone(), body);
    let _result = builder.add_function(fun);
    builder.add_export(Export::new(
        Name::new(s!("main")),
        ExportDescription::Function(0),
    ));
    let module = builder.build();

    let mut buffer = Vec::new();
    let size = emit_binary(&module, &mut buffer).unwrap();
    println!("{:?}",size);
    println!("{:?}",buffer);

    let mut file = File::create(file_name).unwrap();
    let _ = file.write_all(&buffer);
    // println!("Wrote to file {}", file.name());
    println!("Wrote to file {} {}", file_name, file.path());



    // module.emit_wasm();
    // module.
    // let module = Module::empty();
}
