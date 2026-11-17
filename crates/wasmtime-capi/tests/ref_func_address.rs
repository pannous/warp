//! ref.func of a defined function is its VMFuncRef's address, filled when the instance is made
//! (patches/vmctx-ref-func.patch): the reference calls the function, directly, through a GC struct's field, through a
//! table, and the host sees the same function as the table's element and as ref.func's result
use wasmtime::{Config, Engine, Func, Instance, Module, Store};

const MODULE: &str = r#"
(module
  (type $answer (func (result i32)))
  (type $holder (struct (field (ref $answer))))
  (table $calls 1 funcref)
  (elem (table $calls) (i32.const 0) func $seven)
  (elem declare func $eight)
  (func $seven (type $answer) (i32.const 7))
  (func $eight (type $answer) (i32.const 8))
  (func (export "direct") (result i32) (call_ref $answer (ref.func $eight)))
  (func (export "field") (result i32)
    (call_ref $answer (struct.get $holder 0 (struct.new $holder (ref.func $eight)))))
  (func (export "tabled") (result i32) (call_indirect (type $answer) (i32.const 0)))
  (func (export "eight") (result funcref) (ref.func $eight))
  (func (export "seven_from_ref") (result funcref) (ref.func $seven))
  (func (export "seven_from_table") (result funcref) (table.get $calls (i32.const 0))))
"#;

fn called(store: &mut Store<()>, f: Func) -> i32 {
    f.typed::<(), i32>(&*store).unwrap().call(store, ()).unwrap()
}

#[test]
fn ref_func_calls_its_function() {
    let mut config = Config::new();
    config.wasm_gc(true).wasm_function_references(true);
    let engine = Engine::new(&config).unwrap();
    let module = Module::new(&engine, MODULE).unwrap();
    let mut store = Store::new(&engine, ());
    let instance = Instance::new(&mut store, &module, &[]).unwrap();
    for (name, expected) in [("direct", 8), ("field", 8), ("tabled", 7)] {
        let f = instance.get_typed_func::<(), i32>(&mut store, name).unwrap();
        for _ in 0..2 {
            assert_eq!(f.call(&mut store, ()).unwrap(), expected, "{name}");
        }
    }
    let reference = |store: &mut Store<()>, name: &str| -> Func {
        let f = instance.get_typed_func::<(), Option<Func>>(&mut *store, name).unwrap();
        f.call(store, ()).unwrap().unwrap()
    };
    let eight = reference(&mut store, "eight");
    assert_eq!(called(&mut store, eight), 8);
    let from_ref = reference(&mut store, "seven_from_ref");
    let from_table = reference(&mut store, "seven_from_table");
    assert_eq!(called(&mut store, from_ref), 7);
    assert_eq!(called(&mut store, from_table), 7);
    // one VMFuncRef slot for both: the same function to the host
    assert_eq!(from_ref.to_raw(&mut store), from_table.to_raw(&mut store));
}
