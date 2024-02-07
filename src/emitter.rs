use crate::s; // fucking "".to_string()

use wasm_ast::model::Module;
use wasm_ast::{
    Export, ExportDescription, Expression, Function, FunctionType, Name, NumberType,
    NumericInstruction, ResultType, TypeIndex, ValueType,
};
// use wasm_ast::emitter::Emit;
// use wasm_ast::emit_binary;
use wasm_ast::emitter::emit_binary;
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

pub fn build() {
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
        32u32.into(),
        2u32.into(),
        NumericInstruction::Multiply(NumberType::I32).into(),
    ]
    .into();
    let fun = Function::new(kind.into(), locals.clone(), body);
    let _result = builder.add_function(fun);
    builder.add_export(Export::new(
        Name::new(s!("add")),
        ExportDescription::Function(0),
    ));
    let module = builder.build();

    let mut buffer = Vec::new();
    let binary = emit_binary(&module, &mut buffer).unwrap();
    println!("{:?}", binary);

    // module.emit_wasm();
    // module.
    // let module = Module::empty();
}
