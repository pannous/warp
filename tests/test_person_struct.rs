//! Test that user-defined Person struct can be constructed and read via WASM GC
use std::cell::RefCell;
use std::rc::Rc;
use wasmtime::{Config, Engine, Linker, Module, Store, Val};

const PERSON_WAT: &str = r#"
(module $person_getters_test
  (type $String (struct (field $ptr i32) (field $len i32)))
  (type $Person (struct (field $name (ref null $String)) (field $age i64)))

  (memory (export "memory") 1)
  (data (i32.const 8) "Alice")

  (func $create_person (export "create_person") (result (ref $Person))
    i32.const 8
    i32.const 5
    struct.new $String
    i64.const 30
    struct.new $Person
  )

  (func $get_age (export "get_age") (param $p (ref $Person)) (result i64)
    local.get $p
    struct.get $Person $age
  )

  (func $get_name_ptr (export "get_name_ptr") (param $p (ref $Person)) (result i32)
    local.get $p
    struct.get $Person $name
    struct.get $String $ptr
  )

  (func $get_name_len (export "get_name_len") (param $p (ref $Person)) (result i32)
    local.get $p
    struct.get $Person $name
    struct.get $String $len
  )
)
"#;

#[test]
fn test_person_struct_roundtrip() {
    let mut config = Config::new();
    config.wasm_gc(true);
    config.wasm_function_references(true);

    let engine = Engine::new(&config).unwrap();
    let store = Rc::new(RefCell::new(Store::new(&engine, ())));

    let module = Module::new(&engine, PERSON_WAT).unwrap();
    let linker = Linker::new(&engine);
    let instance = linker.instantiate(&mut *store.borrow_mut(), &module).unwrap();

    // Create a Person{name:"Alice", age:30}
    let create_person = instance.get_func(&mut *store.borrow_mut(), "create_person").unwrap();
    let mut results = vec![Val::I32(0)];
    create_person.call(&mut *store.borrow_mut(), &[], &mut results).unwrap();
    let person = results[0];

    // Verify age = 30
    let get_age = instance.get_func(&mut *store.borrow_mut(), "get_age").unwrap();
    let mut age_result = vec![Val::I64(0)];
    get_age.call(&mut *store.borrow_mut(), &[person], &mut age_result).unwrap();
    assert_eq!(age_result[0].unwrap_i64(), 30, "Age should be 30");

    // Verify name = "Alice"
    let get_name_ptr = instance.get_func(&mut *store.borrow_mut(), "get_name_ptr").unwrap();
    let get_name_len = instance.get_func(&mut *store.borrow_mut(), "get_name_len").unwrap();

    let mut ptr_result = vec![Val::I32(0)];
    let mut len_result = vec![Val::I32(0)];
    get_name_ptr.call(&mut *store.borrow_mut(), &[person], &mut ptr_result).unwrap();
    get_name_len.call(&mut *store.borrow_mut(), &[person], &mut len_result).unwrap();

    let ptr = ptr_result[0].unwrap_i32();
    let len = len_result[0].unwrap_i32();
    assert_eq!(ptr, 8);
    assert_eq!(len, 5);

    let memory = instance.get_memory(&mut *store.borrow_mut(), "memory").unwrap();
    let mut buf = vec![0u8; len as usize];
    memory.read(&*store.borrow(), ptr as usize, &mut buf).unwrap();
    assert_eq!(String::from_utf8(buf).unwrap(), "Alice");
}
