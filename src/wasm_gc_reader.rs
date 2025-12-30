use wasmtime::{Engine, Instance, Linker, Module, Store, Val, Config};
use std::rc::Rc;
use std::cell::RefCell;
use anyhow::{Result, anyhow};

/// GcObject wraps a WASM GC struct reference with ergonomic field access
pub struct GcObject {
    inner: Val,
    store: Rc<RefCell<Store<()>>>,
    instance: Instance,
    field_map: Rc<FieldMap>,
}

/// Maps field names to indices for the unified Node struct
pub struct FieldMap {
    // Unified Node struct field indices
    // Based on wasm_gc_emitter.rs structure
}

impl FieldMap {
    fn new() -> Self {
        FieldMap {}
    }

    fn field_index(&self, name: &str) -> Result<usize> {
        // Field layout from wasm_gc_emitter.rs:
        // 0: name_ptr (i32)
        // 1: name_len (i32)
        // 2: tag (i32)
        // 3: int_value (i64)
        // 4: float_value (f64)
        // 5: text_ptr (i32)
        // 6: text_len (i32)
        // 7: left (ref null node)
        // 8: right (ref null node)
        // 9: meta (ref null node)
        match name {
            "name_ptr" => Ok(0),
            "name_len" => Ok(1),
            "tag" => Ok(2),
            "int_value" => Ok(3),
            "float_value" => Ok(4),
            "text_ptr" => Ok(5),
            "text_len" => Ok(6),
            "left" => Ok(7),
            "right" => Ok(8),
            "meta" => Ok(9),
            // Convenience aliases
            "kind" => Ok(2), // tag
            _ => Err(anyhow!("Unknown field: {}", name))
        }
    }
}

impl GcObject {
    pub fn new(val: Val, store: Rc<RefCell<Store<()>>>, instance: Instance) -> Self {
        GcObject {
            inner: val,
            store,
            instance,
            field_map: Rc::new(FieldMap::new()),
        }
    }

    /// Get field by name with type inference
    pub fn get<T: FromVal>(&self, field_name: &str) -> Result<T> {
        let field_idx = self.field_map.field_index(field_name)?;

        // Use wasmtime introspection to read the field
        let mut store = self.store.borrow_mut();

        if let Some(anyref) = self.inner.unwrap_anyref() {
            if let Ok(structref) = anyref.unwrap_struct(&*store) {
                let field_val = structref.field(&mut *store, field_idx)?;
                return T::from_val(field_val, &mut *store, &self.instance, &self.store);
            }
        }

        Err(anyhow!("Cannot read field {}", field_name))
    }

    /// Get field by index
    pub fn get_field(&self, idx: usize) -> Result<Val> {
        let mut store = self.store.borrow_mut();

        if let Some(anyref) = self.inner.unwrap_anyref() {
            if let Ok(structref) = anyref.unwrap_struct(&*store) {
                return structref.field(&mut *store, idx);
            }
        }

        Err(anyhow!("Cannot read field at index {}", idx))
    }

    /// Check if field exists and is non-null
    pub fn has(&self, field_name: &str) -> Result<bool> {
        match self.field_map.field_index(field_name) {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    /// Read string from linear memory (for text/name fields)
    pub fn read_string(&self, ptr: i32, len: i32) -> Result<String> {
        if ptr == 0 || len == 0 {
            return Ok(String::new());
        }

        let mut store = self.store.borrow_mut();
        let memory = self.instance.get_memory(&mut *store, "memory")
            .ok_or_else(|| anyhow!("No memory export"))?;

        let mut buf = vec![0u8; len as usize];
        memory.read(&*store, ptr as usize, &mut buf)?;

        String::from_utf8(buf).map_err(|e| anyhow!("Invalid UTF-8: {}", e))
    }

    /// Get the "name" field as a string (reads from linear memory)
    pub fn name(&self) -> Result<String> {
        let ptr: i32 = self.get("name_ptr")?;
        let len: i32 = self.get("name_len")?;
        self.read_string(ptr, len)
    }

    /// Get the "text" field as a string (reads from linear memory)
    pub fn text(&self) -> Result<String> {
        let ptr: i32 = self.get("text_ptr")?;
        let len: i32 = self.get("text_len")?;
        self.read_string(ptr, len)
    }

    /// Get node kind/tag
    pub fn kind(&self) -> Result<i32> {
        self.get("tag")
    }
}

/// Trait for converting Val to Rust types
pub trait FromVal: Sized {
    fn from_val(val: Val, store: &mut Store<()>, instance: &Instance, store_rc: &Rc<RefCell<Store<()>>>) -> Result<Self>;
}

impl FromVal for i32 {
    fn from_val(val: Val, _store: &mut Store<()>, _instance: &Instance, _store_rc: &Rc<RefCell<Store<()>>>) -> Result<Self> {
        Ok(val.unwrap_i32())
    }
}

impl FromVal for i64 {
    fn from_val(val: Val, _store: &mut Store<()>, _instance: &Instance, _store_rc: &Rc<RefCell<Store<()>>>) -> Result<Self> {
        Ok(val.unwrap_i64())
    }
}

impl FromVal for f64 {
    fn from_val(val: Val, _store: &mut Store<()>, _instance: &Instance, _store_rc: &Rc<RefCell<Store<()>>>) -> Result<Self> {
        Ok(val.unwrap_f64())
    }
}

impl FromVal for GcObject {
    fn from_val(val: Val, _store: &mut Store<()>, instance: &Instance, store_rc: &Rc<RefCell<Store<()>>>) -> Result<Self> {
        Ok(GcObject::new(val, store_rc.clone(), instance.clone()))
    }
}

/// Load a WASM module with GC support and return root object
pub fn read(path: &str) -> Result<GcObject> {
    let mut config = Config::new();
    config.wasm_gc(true);
    config.wasm_function_references(true);

    let engine = Engine::new(&config)?;
    let store = Store::new(&engine, ());
    let store_rc = Rc::new(RefCell::new(store));

    let wasm_bytes = std::fs::read(path)?;
    let module = Module::new(&engine, wasm_bytes)?;

    let linker = Linker::new(&engine);
    let instance = {
        let mut s = store_rc.borrow_mut();
        linker.instantiate(&mut *s, &module)?
    };

    // Call main() to get the root node
    let main = {
        let mut s = store_rc.borrow_mut();
        instance.get_func(&mut *s, "main")
            .ok_or_else(|| anyhow!("No main function"))?
    };

    let mut results = vec![Val::I32(0)];
    {
        let mut s = store_rc.borrow_mut();
        main.call(&mut *s, &[], &mut results)?;
    }

    Ok(GcObject::new(results[0].clone(), store_rc, instance))
}

/// Load WASM bytes directly
pub fn read_bytes(bytes: &[u8]) -> Result<GcObject> {
    let mut config = Config::new();
    config.wasm_gc(true);
    config.wasm_function_references(true);

    let engine = Engine::new(&config)?;
    let store = Store::new(&engine, ());
    let store_rc = Rc::new(RefCell::new(store));

    let module = Module::new(&engine, bytes)?;

    let linker = Linker::new(&engine);
    let instance = {
        let mut s = store_rc.borrow_mut();
        linker.instantiate(&mut *s, &module)?
    };

    // Call main() to get the root node
    let main = {
        let mut s = store_rc.borrow_mut();
        instance.get_func(&mut *s, "main")
            .ok_or_else(|| anyhow!("No main function"))?
    };

    let mut results = vec![Val::I32(0)];
    {
        let mut s = store_rc.borrow_mut();
        main.call(&mut *s, &[], &mut results)?;
    }

    Ok(GcObject::new(results[0].clone(), store_rc, instance))
}

/// Create a node by calling a constructor function
pub fn call_constructor(
    func_name: &str,
    args: &[Val],
    store: Rc<RefCell<Store<()>>>,
    instance: &Instance,
) -> Result<GcObject> {
    let func = {
        let mut s = store.borrow_mut();
        instance.get_func(&mut *s, func_name)
            .ok_or_else(|| anyhow!("Function {} not found", func_name))?
    };

    let mut results = vec![Val::I32(0)];
    {
        let mut s = store.borrow_mut();
        func.call(&mut *s, args, &mut results)?;
    }

    Ok(GcObject::new(results[0].clone(), store, instance.clone()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_field_map() {
        let map = FieldMap::new();
        assert_eq!(map.field_index("tag").unwrap(), 2);
        assert_eq!(map.field_index("kind").unwrap(), 2); // alias
        assert_eq!(map.field_index("int_value").unwrap(), 3);
        assert_eq!(map.field_index("name_ptr").unwrap(), 0);
        assert!(map.field_index("invalid").is_err());
    }
}
