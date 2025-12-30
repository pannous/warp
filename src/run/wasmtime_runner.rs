use wasmtime::*;
use std::fs::read;
use std::path::Path;
use crate::node::Node;
use crate::wasm_gc_reader;

pub fn run_wasm(path: &str) -> Result<i32>{
    let engine = Engine::default();
    let _wat = r#" (module
            (import "host" "host_func" (func $host_hello (param i32)))
            (func (export "main") i32.const 42) ) "#;
    // let module = Module::new(&engine, _wat)?;
    let module = Module::new(&engine, &read(Path::new(path))?)?;

    let data = 4; // could be any data
    let mut store = Store::new(&engine, data);
    let _host_func = Func::wrap(&mut store, |caller: Caller<'_, u32>, param: i32| {
        println!("Got {} from WebAssembly", param);
        println!("my host state is: {}", caller.data());
    });

    // Error: expected 0 imports, found 1   Instance::new needs FIXED imports known ahead of time WTH
    // let imports :&[Extern]= &[];// &[host_func.into()];
// let instance = Instance::new(&mut store, &module, imports)?;

    let mut linker = Linker::new(&engine);
    // let external_func = Func::new(&mut store, typ, test_func);
    // linker.func_new("namespace", "external_func",typ, external_func);
    let typ = FuncType::new(&engine, [ValType::I32], [ValType::I32]);
    linker.func_new("namespace", "external_func",typ,move |_, _, _| {
        Ok(())
        })?;
    linker.func_wrap("", "", || {})?;
    let instance = linker.instantiate(&mut store, &module)?;

    type ReturnType = i32;
    let wasm_main = instance.get_typed_func::<(), ReturnType>(&mut store, "main")?;
    let result= wasm_main.call(&mut store, ())?;
    println!("Result: {:?}", result);

    Ok(result)
}

fn test_func(_caller: Caller<'_, u32>, param: &[Val], _xyz:&mut[Val]) -> Result<i32, Trap> {
    Ok(param[0].unwrap_i32() * 2) // Dummy implementation
}

pub fn run(path: &str) -> Node
{
    let result = wasm_gc_reader::run_wasm_gc_object(path).unwrap();
    Node::from_gc_object(&result)
}