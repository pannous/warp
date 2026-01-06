/// Experimental: Return raw GC structs instead of Node encoding
/// This allows direct struct field access without Node wrapper overhead

use wasmtime::*;
use anyhow::Result;

/// Person struct mirrors the WASM GC $Person type
#[derive(Debug, Clone, PartialEq)]
pub struct Person {
    pub name: String,
    pub age: i64,
}

impl Person {
    pub fn new(name: &str, age: i64) -> Self {
        Self { name: name.to_string(), age }
    }
}

/// Read a Person from a GcObject (WASM GC struct)
fn person_from_gc(val: &Val, store: &mut Store<()>, instance: &Instance) -> Result<Person> {
    let anyref = val.unwrap_anyref().ok_or_else(|| anyhow::anyhow!("not anyref"))?;
    let structref = anyref.unwrap_struct(&*store)?;

    // Field 0: name (ref $String -> ptr, len)
    let name_val = structref.field(&mut *store, 0)?;
    let name = read_string_field(&name_val, store, instance)?;

    // Field 1: age (i64)
    let age_val = structref.field(&mut *store, 1)?;
    let age = age_val.unwrap_i64();

    Ok(Person { name, age })
}

/// Read string from $String struct (ptr, len) in linear memory
fn read_string_field(val: &Val, store: &mut Store<()>, instance: &Instance) -> Result<String> {
    let anyref = val.unwrap_anyref().ok_or_else(|| anyhow::anyhow!("not anyref"))?;
    let structref = anyref.unwrap_struct(&*store)?;

    let ptr = structref.field(&mut *store, 0)?.unwrap_i32();
    let len = structref.field(&mut *store, 1)?.unwrap_i32();

    if len == 0 {
        return Ok(String::new());
    }

    let memory = instance.get_memory(&mut *store, "memory")
        .ok_or_else(|| anyhow::anyhow!("no memory"))?;
    let mut buf = vec![0u8; len as usize];
    memory.read(&*store, ptr as usize, &mut buf)?;
    Ok(String::from_utf8(buf)?)
}

/// Emit a $Person struct and return it directly from main
fn emit_person_wasm(name: &str, age: i64) -> Vec<u8> {
    use wasm_encoder::*;
    use wasm_encoder::StorageType::Val;

    let mut module = Module::new();

    // Types section
    let mut types = TypeSection::new();

    // Type 0: $String = struct { ptr: i32, len: i32 }
    types.ty().struct_(vec![
        FieldType { element_type: Val(ValType::I32), mutable: false },
        FieldType { element_type: Val(ValType::I32), mutable: false },
    ]);

    // Type 1: $Person = struct { name: ref $String, age: i64 }
    let string_ref = RefType { nullable: false, heap_type: HeapType::Concrete(0) };
    types.ty().struct_(vec![
        FieldType { element_type: Val(ValType::Ref(string_ref)), mutable: false },
        FieldType { element_type: Val(ValType::I64), mutable: false },
    ]);

    // Type 2: main() -> ref $Person
    let person_ref = RefType { nullable: false, heap_type: HeapType::Concrete(1) };
    types.ty().func_type(&FuncType::new([], [ValType::Ref(person_ref)]));

    module.section(&types);

    // Function section (must come before Memory)
    let mut functions = FunctionSection::new();
    functions.function(2); // main uses type 2
    module.section(&functions);

    // Memory section
    let mut memories = MemorySection::new();
    memories.memory(MemoryType { minimum: 1, maximum: None, memory64: false, shared: false, page_size_log2: None });
    module.section(&memories);

    // Export section
    let mut exports = ExportSection::new();
    exports.export("memory", ExportKind::Memory, 0);
    exports.export("main", ExportKind::Func, 0);
    module.section(&exports);

    // Code section (before Data)
    let mut codes = CodeSection::new();
    let mut func = Function::new([]);

    // Create $String for name: struct.new $String (ptr=0, len=name.len())
    func.instruction(&Instruction::I32Const(0)); // ptr
    func.instruction(&Instruction::I32Const(name.len() as i32)); // len
    func.instruction(&Instruction::StructNew(0)); // $String

    // Create $Person: struct.new $Person (name_ref, age)
    func.instruction(&Instruction::I64Const(age));
    func.instruction(&Instruction::StructNew(1)); // $Person

    func.instruction(&Instruction::End);
    codes.function(&func);
    module.section(&codes);

    // Data section - store the name string (must be after Code)
    let mut data = DataSection::new();
    data.active(0, &ConstExpr::i32_const(0), name.as_bytes().iter().copied());
    module.section(&data);

    module.finish()
}

#[test]
fn test_raw_person_struct() -> Result<()> {
    let expected = Person::new("Alice", 30);

    // Emit WASM that creates a $Person struct directly
    let wasm_bytes = emit_person_wasm("Alice", 30);

    // Load and run
    let mut config = Config::new();
    config.wasm_gc(true);
    config.wasm_function_references(true);

    let engine = Engine::new(&config)?;
    let mut store = Store::new(&engine, ());
    let module = Module::new(&engine, &wasm_bytes)?;

    let linker = Linker::new(&engine);
    let instance = linker.instantiate(&mut store, &module)?;

    let main = instance.get_func(&mut store, "main")
        .ok_or_else(|| anyhow::anyhow!("no main"))?;

    let mut results = vec![Val::I32(0)];
    main.call(&mut store, &[], &mut results)?;

    // Convert GC struct to Person
    let result = person_from_gc(&results[0], &mut store, &instance)?;

    assert_eq!(result, expected);
    println!("Raw Person struct roundtrip works: {:?}", result);

    Ok(())
}

#[test]
fn test_class_instance_raw() -> Result<()> {
    // The expected Person - created simply in Rust
    let alice = Person::new("Alice", 30);

    // Parse and emit via our emitter, then read back as Person
    // TODO: This will use WasmGcEmitter to emit class Person and instance
    // For now, test the raw struct approach works

    let wasm_bytes = emit_person_wasm("Alice", 30);

    let mut config = Config::new();
    config.wasm_gc(true);
    config.wasm_function_references(true);

    let engine = Engine::new(&config)?;
    let mut store = Store::new(&engine, ());
    let module = Module::new(&engine, &wasm_bytes)?;

    let linker = Linker::new(&engine);
    let instance = linker.instantiate(&mut store, &module)?;

    let main = instance.get_func(&mut store, "main")
        .ok_or_else(|| anyhow::anyhow!("no main"))?;

    let mut results = vec![Val::I32(0)];
    main.call(&mut store, &[], &mut results)?;

    let result = person_from_gc(&results[0], &mut store, &instance)?;

    assert_eq!(result, alice);

    Ok(())
}
