use wasmtime::*;
use std::fs::read;
use std::path::Path;
use std::str::from_utf8;
pub fn run_wasm(path: &str) -> wasmtime::Result<()>{
    let engine = Engine::default();
    let _wat = r#" (module
            (import "host" "host_func" (func $host_hello (param i32)))
            (func (export "hello") i32.const 3; call $host_hello) ) "#;
    // let module = Module::new(&engine, wat)?;
    let module = Module::new(&engine, &read(Path::new(path))?)?;

    let data = 4; // could be any data
    let mut store = Store::new(&engine, data);
    let host_func = Func::wrap(&mut store, |caller: Caller<'_, u32>, param: i32| {
        println!("Got {} from WebAssembly", param);
        println!("my host state is: {}", caller.data());
    });

    let instance = Instance::new(&mut store, &module, &[host_func.into()])?;
    let wasm_main = instance.get_typed_func::<(), ()>(&mut store, "main")?;

    // And finally we can call the wasm!
    let result=wasm_main.call(&mut store, ())?;
    println!("Result: {:?}", result);

    Ok(())
}