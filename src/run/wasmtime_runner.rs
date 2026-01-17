use crate::node::Node;
use crate::wasm_gc_reader;
use std::fs::read;
use std::path::Path;
use wasmtime::*;

pub fn run_wasm(path: &str) -> Result<i32> {
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
	linker.func_new("namespace", "external_func", typ, move |_, _, _| Ok(()))?;
	linker.func_wrap("", "", || {})?;
	let instance = linker.instantiate(&mut store, &module)?;

	type ReturnType = i32;
	let wasm_main = instance.get_typed_func::<(), ReturnType>(&mut store, "main")?;
	let result = wasm_main.call(&mut store, ())?;
	println!("Result: {:?}", result);

	Ok(result)
}

fn test_func(_caller: Caller<'_, u32>, param: &[Val], _xyz: &mut [Val]) -> Result<i32, Trap> {
	Ok(param[0].unwrap_i32() * 2) // Dummy implementation
}

pub fn run(path: &str) -> Node {
	let result = wasm_gc_reader::run_wasm_gc_object(path).unwrap();
	Node::from_gc_object(&result)
}

/// Run WAT/WAST text format code - compiles to WASM binary first
pub fn run_wat(wat_code: &str) -> Node {
	use crate::extensions::numbers::Number;
	use crate::ffi::{link_ffi_functions, FfiState};
	use crate::type_kinds::Kind;

	// Create engine with GC support
	let mut config = Config::new();
	config.wasm_gc(true);
	config.wasm_function_references(true);
	let engine = Engine::new(&config).expect("Failed to create engine");

	// Compile WAT to module (wasmtime handles text → binary conversion)
	let module = match Module::new(&engine, wat_code) {
		Ok(m) => m,
		Err(e) => {
			eprintln!("WAT compilation error: {}", e);
			return Node::Empty;
		}
	};

	// Create store with FFI state
	let mut store: Store<FfiState> = Store::new(&engine, FfiState::new());

	// Create linker with FFI functions
	let mut linker: Linker<FfiState> = Linker::new(&engine);
	if let Err(e) = link_ffi_functions(&mut linker, &engine) {
		eprintln!("FFI linking error: {}", e);
		return Node::Empty;
	}

	// Instantiate module
	let instance = match linker.instantiate(&mut store, &module) {
		Ok(i) => i,
		Err(e) => {
			eprintln!("Instantiation error: {}", e);
			return Node::Empty;
		}
	};

	// Try to find and call main function
	let main = match instance.get_func(&mut store, "main") {
		Some(f) => f,
		None => {
			eprintln!("No main function found");
			return Node::Empty;
		}
	};

	// Call main and get result - use AnyRef for GC struct results
	let mut results = vec![Val::null_any_ref()];
	if let Err(e) = main.call(&mut store, &[], &mut results) {
		eprintln!("Execution error: {}", e);
		return Node::Empty;
	}

	// Try to read as GC struct (Node type)
	if let Some(anyref) = results[0].unwrap_anyref() {
		if let Ok(structref) = anyref.unwrap_struct(&store) {
			// Read kind field (i64) - field 0
			if let Ok(kind_val) = structref.field(&mut store, 0) {
				let kind = kind_val.unwrap_i64();
				let tag = (kind & 0xFF) as u8;

				match tag {
					t if t == Kind::Empty as u8 => return Node::Empty,
					t if t == Kind::Int as u8 => {
						// data field (field 1) contains boxed i64
						if let Ok(data_val) = structref.field(&mut store, 1) {
							if let Some(data_any) = data_val.unwrap_anyref() {
								if let Ok(box_ref) = data_any.unwrap_struct(&store) {
									if let Ok(inner) = box_ref.field(&mut store, 0) {
										return Node::Number(Number::Int(inner.unwrap_i64()));
									}
								}
							}
						}
						return Node::Number(Number::Int(0));
					}
					t if t == Kind::Float as u8 => {
						// data field (field 1) contains boxed f64
						if let Ok(data_val) = structref.field(&mut store, 1) {
							if let Some(data_any) = data_val.unwrap_anyref() {
								if let Ok(box_ref) = data_any.unwrap_struct(&store) {
									if let Ok(inner) = box_ref.field(&mut store, 0) {
										return Node::Number(Number::Float(inner.unwrap_f64()));
									}
								}
							}
						}
						return Node::Number(Number::Float(0.0));
					}
					_ => return Node::Empty,
				}
			}
		}
	}

	// Fallback: try as primitive types
	match &results[0] {
		Val::I32(n) => Node::Number(Number::Int(*n as i64)),
		Val::I64(n) => Node::Number(Number::Int(*n)),
		Val::F32(n) => Node::Number(Number::Float(f32::from_bits(*n) as f64)),
		Val::F64(n) => Node::Number(Number::Float(f64::from_bits(*n))),
		_ => Node::Empty,
	}
}
