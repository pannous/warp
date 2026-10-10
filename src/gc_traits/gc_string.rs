//! GcString (a ptr/len string or an i8 array) and ObjFieldValue, the field values of obj!

use super::*;

/// GC string wrapper (wraps WebAssembly GC struct with ptr/len or array of bytes)
pub struct GcString {
    inner: GcStringInner,
}

pub(super) enum GcStringInner {
    PtrLen(Rooted<StructRef>), // $String = (struct (field ptr i32) (field len i32))
    Array(Rooted<ArrayRef>),   // (array i8)
}

impl GcString {
    /// Create a new GC string from a Rust &str using new_string function
    pub fn create(store: &mut Store<()>, instance: &Instance, s: &str) -> Result<Val> {
        let new_string = instance.get_func(&mut *store, "new_string").ok_or_else(|| {
            anyhow!("new_string function not found - ensure gc_types.wat exports it")
        })?;

        let memory = instance
            .get_memory(&mut *store, "memory")
            .ok_or_else(|| anyhow!("memory not found"))?;

        let offset = 0;
        memory.write(&mut *store, offset, s.as_bytes())?;

        let mut results = vec![Val::I32(0)];
        new_string.call(
            &mut *store,
            &[Val::I32(offset as i32), Val::I32(s.len() as i32)],
            &mut results,
        )?;

        Ok(results[0])
    }

    /// Create from a Val (auto-detects struct vs array)
    pub fn from_val(store: &Store<()>, val: Val) -> Result<Self> {
        let anyref = anyref_of(&val)?;

        // Try struct first (ptr/len pattern)
        if let Ok(structref) = anyref.clone().unwrap_struct(store) {
            return Ok(Self {
                inner: GcStringInner::PtrLen(structref),
            });
        }

        // Try array (i8 array pattern)
        if let Ok(arrayref) = anyref.unwrap_array(store) {
            return Ok(Self {
                inner: GcStringInner::Array(arrayref),
            });
        }

        Err(anyhow!("Value is neither a string struct nor byte array"))
    }

    /// Convert to Rust String
    pub fn to_string(&self, store: &mut Store<()>) -> Result<String> {
        match &self.inner {
            GcStringInner::PtrLen(_structref) => {
                // For ptr/len strings, we need memory access
                // This requires the instance, so for now we return an error
                Err(anyhow!(
                    "ptr/len string requires instance for memory access"
                ))
            }
            GcStringInner::Array(arrayref) => array_text(arrayref, store),
        }
    }

    /// Convert to Rust String with instance access for ptr/len strings
    pub fn to_string_with_instance(&self, store: &mut Store<()>, instance: &Instance) -> Result<String> {
        match &self.inner {
            GcStringInner::PtrLen(structref) => memory_text(structref, store, instance),
            GcStringInner::Array(arrayref) => array_text(arrayref, store),
        }
    }
}

/// The text of a `$String` (ptr, len) struct: its bytes in the instance's linear memory
pub(super) fn memory_text(structref: &Rooted<StructRef>, store: &mut Store<()>, instance: &Instance) -> Result<String> {
    let mut field = |index| structref.field(&mut *store, index)?.i32().ok_or_else(|| anyhow!("$String field {index} is no i32"));
    let (ptr, len) = (field(0)?, field(1)?);
    if len == 0 {
        return Ok(String::new());
    }
    let memory = instance.get_memory(&mut *store, "memory").ok_or_else(|| anyhow!("no memory export"))?;
    crate::host::read_string_from_memory(&memory, &*store, ptr as u32, len as u32)
}

/// The text of an `(array i8)` string: its bytes as UTF-8
fn array_text(array: &Rooted<ArrayRef>, store: &mut Store<()>) -> Result<String> {
    let len = array.len(&*store)?;
    let bytes = (0..len).map(|i| Ok(array.get(&mut *store, i)?.unwrap_i32() as u8)).collect::<Result<Vec<u8>>>()?;
    Ok(String::from_utf8(bytes)?)
}

impl FromVal for GcString {
    fn from_val(val: Val, store: &mut Store<()>) -> Result<Self> {
        GcString::from_val(store, val)
    }
}

/// Value type for object-literal syntax
#[derive(Clone)]
pub enum ObjFieldValue {
    String(String),
    I32(i32),
    I64(i64),
    F32(f32),
    F64(f64),
    Bool(bool),
    Null,
}

impl From<&str> for ObjFieldValue {
    fn from(s: &str) -> Self {
        ObjFieldValue::String(s.to_string())
    }
}

impl From<String> for ObjFieldValue {
    fn from(s: String) -> Self {
        ObjFieldValue::String(s)
    }
}

impl From<i32> for ObjFieldValue {
    fn from(n: i32) -> Self {
        ObjFieldValue::I32(n)
    }
}

impl From<i64> for ObjFieldValue {
    fn from(n: i64) -> Self {
        ObjFieldValue::I64(n)
    }
}

impl From<f32> for ObjFieldValue {
    fn from(n: f32) -> Self {
        ObjFieldValue::F32(n)
    }
}

impl From<f64> for ObjFieldValue {
    fn from(n: f64) -> Self {
        ObjFieldValue::F64(n)
    }
}

impl From<bool> for ObjFieldValue {
    fn from(b: bool) -> Self {
        ObjFieldValue::Bool(b)
    }
}
