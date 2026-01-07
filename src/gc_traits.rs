//! GC Traits - Ergonomic WebAssembly GC struct access
//!
//! Ported from rasm project for type-safe, named field access to WASM GC structs.
//!
//! # Usage
//! ```ignore
//! use wasp::gc_traits::GcObject;
//! use wasp::gc_struct;
//!
//! gc_struct! {
//!     Person {
//!         name: 0 => String,
//!         age: 1 => i64,
//!     }
//! }
//!
//! let person = Person::from_val(val, store, Some(instance))?;
//! let name: String = person.name()?;
//! let age: i64 = person.age()?;
//! ```

#![allow(unused)]

use anyhow::{anyhow, bail, Result};
use paste::paste;
use std::cell::RefCell;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::ops::Index;
use std::rc::Rc;
use std::str;
use std::sync::{Arc, Mutex};
use wasmtime::*;

// Re-export for macro use
pub use paste::paste as paste_paste;

/// Trait for converting Val to Rust types
pub trait FromVal: Sized {
    fn from_val(val: Val, store: &mut Store<()>) -> Result<Self>;
}

/// Trait for converting Rust types to Val
pub trait ToVal {
    fn to_val(&self, store: &mut Store<()>, instance: Option<&Instance>) -> Result<Val>;
}

impl FromVal for i32 {
    fn from_val(val: Val, _store: &mut Store<()>) -> Result<Self> {
        Ok(val.unwrap_i32())
    }
}

impl FromVal for i64 {
    fn from_val(val: Val, _store: &mut Store<()>) -> Result<Self> {
        Ok(val.unwrap_i64())
    }
}

impl FromVal for f32 {
    fn from_val(val: Val, _store: &mut Store<()>) -> Result<Self> {
        Ok(f32::from_bits(val.unwrap_f32() as u32))
    }
}

impl FromVal for f64 {
    fn from_val(val: Val, _store: &mut Store<()>) -> Result<Self> {
        Ok(val.unwrap_f64())
    }
}

impl FromVal for bool {
    fn from_val(val: Val, _store: &mut Store<()>) -> Result<Self> {
        Ok(val.unwrap_i32() != 0)
    }
}

impl FromVal for String {
    fn from_val(val: Val, store: &mut Store<()>) -> Result<Self> {
        let gc_string = GcString::from_val(store, val)?;
        gc_string.to_string(store)
    }
}

impl FromVal for Rooted<StructRef> {
    fn from_val(val: Val, store: &mut Store<()>) -> Result<Self> {
        let anyref = val
            .unwrap_anyref()
            .ok_or_else(|| anyhow!("not an anyref"))?;
        anyref.unwrap_struct(&*store)
    }
}

// ToVal implementations
impl ToVal for i32 {
    fn to_val(&self, _store: &mut Store<()>, _instance: Option<&Instance>) -> Result<Val> {
        Ok(Val::I32(*self))
    }
}

impl ToVal for i64 {
    fn to_val(&self, _store: &mut Store<()>, _instance: Option<&Instance>) -> Result<Val> {
        Ok(Val::I64(*self))
    }
}

impl ToVal for f32 {
    fn to_val(&self, _store: &mut Store<()>, _instance: Option<&Instance>) -> Result<Val> {
        Ok(Val::F32(self.to_bits()))
    }
}

impl ToVal for f64 {
    fn to_val(&self, _store: &mut Store<()>, _instance: Option<&Instance>) -> Result<Val> {
        Ok(Val::F64(self.to_bits()))
    }
}

impl ToVal for bool {
    fn to_val(&self, _store: &mut Store<()>, _instance: Option<&Instance>) -> Result<Val> {
        Ok(Val::I32(if *self { 1 } else { 0 }))
    }
}

impl ToVal for &str {
    fn to_val(&self, store: &mut Store<()>, instance: Option<&Instance>) -> Result<Val> {
        let instance =
            instance.ok_or_else(|| anyhow!("Instance required for string creation"))?;
        GcString::create(store, instance, self)
    }
}

impl ToVal for String {
    fn to_val(&self, store: &mut Store<()>, instance: Option<&Instance>) -> Result<Val> {
        self.as_str().to_val(store, instance)
    }
}

impl ToVal for Rooted<StructRef> {
    fn to_val(&self, _store: &mut Store<()>, _instance: Option<&Instance>) -> Result<Val> {
        Ok(Val::AnyRef(Some(self.clone().into())))
    }
}

impl ToVal for Val {
    fn to_val(&self, _store: &mut Store<()>, _instance: Option<&Instance>) -> Result<Val> {
        Ok(self.clone())
    }
}

/// Trait for field index types (supports both usize and &str field names)
pub trait FieldIndex {
    fn to_field_index(&self, struct_ref: &Rooted<StructRef>, store: &Store<()>) -> Result<usize>;
}

impl FieldIndex for usize {
    fn to_field_index(&self, _struct_ref: &Rooted<StructRef>, _store: &Store<()>) -> Result<usize> {
        Ok(*self)
    }
}

impl FieldIndex for &str {
    fn to_field_index(&self, struct_ref: &Rooted<StructRef>, store: &Store<()>) -> Result<usize> {
        let struct_type = struct_ref.ty(store)?;
        wasm_name_resolver::lookup_field_index(&struct_type, self)
    }
}

/// Simplified wrapper that owns the store and provides clean API
///
/// # Example
/// ```ignore
/// let mut person = GcObject::new(person_val, store, Some(&instance));
/// let name: String = person.get(0)?;  // No &mut store needed!
/// let age: i32 = person.get(1)?;
/// person.set_field("age", 30)?;       // Mutation!
/// ```
/// WASM GC struct wrapper that owns the store
/// Can be stored in Node::Data for roundtrip
#[derive(Clone)]
pub struct GcObject {
    inner: Rooted<StructRef>,
    store: Rc<RefCell<Store<()>>>,
    instance: Option<Instance>,
}

impl PartialEq for GcObject {
    fn eq(&self, other: &Self) -> bool {
        // Compare by store identity (same underlying store = same object space)
        Rc::ptr_eq(&self.store, &other.store)
    }
}

impl std::fmt::Debug for GcObject {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.with_store(|store| {
            // Get struct type info
            let struct_type = match self.inner.ty(&*store) {
                Ok(ty) => ty,
                Err(_) => return write!(f, "GcObject {{ <error getting type> }}"),
            };

            // Try to get field names from registered metadata
            let field_names = wasm_name_resolver::field_names(&struct_type).ok();
            let field_count = struct_type.fields().len();

            write!(f, "GcObject{{")?;

            for idx in 0..field_count {
                if idx > 0 {
                    write!(f, " ")?;
                }

                // Get field name if available
                let field_name = field_names
                    .as_ref()
                    .and_then(|names| names.get(idx))
                    .and_then(|n| n.as_ref());

                if let Some(name) = field_name {
                    write!(f, "{}:", name)?;
                }

                // Get and format field value
                match self.inner.field(&mut *store, idx) {
                    Ok(val) => {
                        format_gc_val(f, store, &val, self.instance.as_ref())?;
                    }
                    Err(_) => write!(f, "<error>")?,
                }
            }

            write!(f, "}}")
        })
    }
}

/// Format a Val for debug output
fn format_gc_val(f: &mut std::fmt::Formatter<'_>, store: &mut Store<()>, val: &Val, instance: Option<&Instance>) -> std::fmt::Result {
    match val {
        Val::I32(n) => write!(f, "{}", n),
        Val::I64(n) => write!(f, "{}", n),
        Val::F32(n) => write!(f, "{}", f32::from_bits(*n)),
        Val::F64(n) => write!(f, "{}", f64::from_bits(*n)),
        Val::AnyRef(Some(anyref)) => {
            // Try to read as string struct first
            if let Ok(struct_ref) = anyref.clone().unwrap_struct(&*store) {
                // Check if it's a String struct (ptr/len pattern)
                if let Ok(struct_type) = struct_ref.ty(&*store) {
                    if struct_type.fields().len() == 2 {
                        // Likely a String struct - try to read it
                        if let Some(inst) = instance {
                            if let Ok(ptr_val) = struct_ref.field(&mut *store, 0) {
                                if let Ok(len_val) = struct_ref.field(&mut *store, 1) {
                                    if let (Some(ptr), Some(len)) = (ptr_val.i32(), len_val.i32()) {
                                        if let Some(memory) = inst.get_memory(&mut *store, "memory") {
                                            let mut buf = vec![0u8; len as usize];
                                            if memory.read(&*store, ptr as usize, &mut buf).is_ok() {
                                                if let Ok(s) = String::from_utf8(buf) {
                                                    return write!(f, "'{}'", s);
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                // Nested struct - show as GcObject
                write!(f, "<struct>")
            } else {
                write!(f, "<anyref>")
            }
        }
        Val::AnyRef(None) => write!(f, "null"),
        _ => write!(f, "<val>"),
    }
}

impl GcObject {
    /// Create a new GcObject that owns the store
    pub fn new(val: Val, store: Store<()>, instance: Option<Instance>) -> Result<Self> {
        let anyref = val
            .unwrap_anyref()
            .ok_or_else(|| anyhow!("not an anyref"))?;
        let inner = anyref.unwrap_struct(&store)?;
        Ok(Self {
            inner,
            store: Rc::new(RefCell::new(store)),
            instance,
        })
    }

    /// Create from an existing StructRef, sharing the store with another GcObject
    pub fn from_struct_shared(
        struct_ref: Rooted<StructRef>,
        store: Rc<RefCell<Store<()>>>,
        instance: Option<Instance>,
    ) -> Self {
        Self {
            inner: struct_ref,
            store,
            instance,
        }
    }

    /// Create from an existing StructRef
    pub fn from_struct(
        struct_ref: Rooted<StructRef>,
        store: Store<()>,
        instance: Option<Instance>,
    ) -> Self {
        Self {
            inner: struct_ref,
            store: Rc::new(RefCell::new(store)),
            instance,
        }
    }

    /// Access the store mutably
    pub fn with_store<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&mut Store<()>) -> R,
    {
        f(&mut *self.store.borrow_mut())
    }

    /// Get a field with automatic type conversion (supports both index and field name)
    pub fn get<T: FromVal, I: FieldIndex>(&self, field: I) -> Result<T> {
        self.with_store(|store| {
            let idx = field.to_field_index(&self.inner, &*store)?;
            let val = self.inner.field(&mut *store, idx)?;
            T::from_val(val, &mut *store)
        })
    }

    pub fn get_int(&self, idx: usize) -> Result<i32> {
        self.with_store(|store| {
            let val = self.inner.field(&mut *store, idx)?;
            i32::from_val(val, &mut *store)
        })
    }

    pub fn get_str(&self, idx: usize) -> Result<String> {
        self.with_store(|store| {
            let val = self.inner.field(&mut *store, idx)?;
            // Try with instance for ptr/len strings
            if let Some(instance) = &self.instance {
                let gc_string = GcString::from_val(store, val)?;
                gc_string.to_string_with_instance(store, instance)
            } else {
                String::from_val(val, store)
            }
        })
    }

    /// Get a string field with instance access for ptr/len strings
    pub fn get_string<I: FieldIndex>(&self, field: I) -> Result<String> {
        self.with_store(|store| {
            let idx = field.to_field_index(&self.inner, &*store)?;
            let val = self.inner.field(&mut *store, idx)?;
            if let Some(instance) = &self.instance {
                let gc_string = GcString::from_val(store, val)?;
                gc_string.to_string_with_instance(store, instance)
            } else {
                String::from_val(val, store)
            }
        })
    }

    /// Get a nested struct
    pub fn get_struct<I: FieldIndex>(&self, index: I) -> Result<Rooted<StructRef>> {
        self.with_store(|store| {
            let idx = index.to_field_index(&self.inner, &*store)?;
            let val = self.inner.field(&mut *store, idx)?;
            let anyref = val
                .unwrap_anyref()
                .ok_or_else(|| anyhow!("field {} is not an anyref", idx))?;
            anyref.unwrap_struct(&*store)
        })
    }

    /// Get nested struct as a GcObject that shares the same store
    pub fn get_struct_object<I: FieldIndex>(&self, index: I) -> Result<GcObject> {
        let struct_ref = self.get_struct(index)?;
        Ok(GcObject::from_struct_shared(
            struct_ref,
            self.store.clone(),
            self.instance.clone(),
        ))
    }

    /// Get nested struct as a wrapped type (Person, Point, etc.)
    pub fn get_as<T: GcStructWrapper, I: FieldIndex>(&self, index: I) -> Result<T> {
        let gc_obj = self.get_struct_object(index)?;
        Ok(T::from_gc_object(gc_obj))
    }

    /// Check if a field is null
    pub fn is_null<I: FieldIndex>(&self, index: I) -> Result<bool> {
        self.with_store(|store| {
            let idx = index.to_field_index(&self.inner, &*store)?;
            let val = self.inner.field(&mut *store, idx)?;
            Ok(val.unwrap_anyref().is_none())
        })
    }

    /// Check if a field is not null
    pub fn has<I: FieldIndex>(&self, index: I) -> Result<bool> {
        self.with_store(|store| {
            let idx = index.to_field_index(&self.inner, &*store)?;
            let val = self.inner.field(&mut *store, idx)?;
            Ok(!val.unwrap_anyref().is_none())
        })
    }

    /// Set a field value (for mutable fields)
    pub fn set_field<T: ToVal, I: FieldIndex>(&self, field: I, value: T) -> Result<()> {
        self.with_store(|store| {
            let idx = field.to_field_index(&self.inner, &*store)?;
            let val = value.to_val(&mut *store, self.instance.as_ref())?;
            self.inner.set_field(&mut *store, idx, val)
        })
    }

    /// Get the inner StructRef for passing as a field value
    pub fn as_struct_ref(&self) -> &Rooted<StructRef> {
        &self.inner
    }

    /// Convert to Val for passing to WASM functions
    pub fn to_val(&self) -> Val {
        Val::AnyRef(Some(self.inner.clone().into()))
    }

    /// Get a clone of the store
    pub fn clone_store(&self) -> Rc<RefCell<Store<()>>> {
        self.store.clone()
    }

    /// Get a clone of the instance
    pub fn clone_instance(&self) -> Option<Instance> {
        self.instance.clone()
    }
}

/// Trait for types that wrap a GcObject
pub trait GcStructWrapper: Sized {
    fn from_gc_object(obj: GcObject) -> Self;
    fn get_inner(&self) -> &GcObject;
}

// Blanket implementation for all GcStructWrapper types
impl<T: GcStructWrapper> ToVal for T {
    fn to_val(&self, _store: &mut Store<()>, _instance: Option<&Instance>) -> Result<Val> {
        Ok(self.get_inner().to_val())
    }
}

/// GC string wrapper (wraps WebAssembly GC struct with ptr/len or array of bytes)
pub struct GcString {
    inner: GcStringInner,
}

enum GcStringInner {
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

        Ok(results[0].clone())
    }

    /// Create from a Val (auto-detects struct vs array)
    pub fn from_val(store: &Store<()>, val: Val) -> Result<Self> {
        let anyref = val
            .unwrap_anyref()
            .ok_or_else(|| anyhow!("not an anyref"))?;

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
            GcStringInner::Array(arrayref) => {
                let len = arrayref.len(&*store)? as usize;
                let mut bytes = Vec::with_capacity(len);

                for i in 0..len {
                    let elem = arrayref.get(&mut *store, i as u32)?;
                    bytes.push(elem.unwrap_i32() as u8);
                }

                Ok(String::from_utf8(bytes)?)
            }
        }
    }

    /// Convert to Rust String with instance access for ptr/len strings
    pub fn to_string_with_instance(&self, store: &mut Store<()>, instance: &Instance) -> Result<String> {
        match &self.inner {
            GcStringInner::PtrLen(structref) => {
                // Read ptr and len from the $String struct
                let ptr = structref.field(&mut *store, 0)?.unwrap_i32();
                let len = structref.field(&mut *store, 1)?.unwrap_i32();

                if len == 0 {
                    return Ok(String::new());
                }

                // Read from linear memory
                let memory = instance.get_memory(&mut *store, "memory")
                    .ok_or_else(|| anyhow!("no memory export"))?;
                let mut buf = vec![0u8; len as usize];
                memory.read(&*store, ptr as usize, &mut buf)?;
                Ok(String::from_utf8(buf)?)
            }
            GcStringInner::Array(arrayref) => {
                // Array strings don't need instance
                let len = arrayref.len(&*store)? as usize;
                let mut bytes = Vec::with_capacity(len);

                for i in 0..len {
                    let elem = arrayref.get(&mut *store, i as u32)?;
                    bytes.push(elem.unwrap_i32() as u8);
                }

                Ok(String::from_utf8(bytes)?)
            }
        }
    }
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

/// Trait for reading fields from GcObject with proper type dispatch
/// (String uses get_string for ptr/len support, others use get)
pub trait GcReadable: Sized {
    fn read_from_gc(gc_obj: &GcObject, idx: usize) -> anyhow::Result<Self>;
}

impl GcReadable for String {
    fn read_from_gc(gc_obj: &GcObject, idx: usize) -> anyhow::Result<Self> {
        gc_obj.get_string(idx)
    }
}

impl GcReadable for i32 {
    fn read_from_gc(gc_obj: &GcObject, idx: usize) -> anyhow::Result<Self> {
        gc_obj.get(idx)
    }
}

impl GcReadable for i64 {
    fn read_from_gc(gc_obj: &GcObject, idx: usize) -> anyhow::Result<Self> {
        gc_obj.get(idx)
    }
}

impl GcReadable for f32 {
    fn read_from_gc(gc_obj: &GcObject, idx: usize) -> anyhow::Result<Self> {
        gc_obj.get(idx)
    }
}

impl GcReadable for f64 {
    fn read_from_gc(gc_obj: &GcObject, idx: usize) -> anyhow::Result<Self> {
        gc_obj.get(idx)
    }
}

impl GcReadable for bool {
    fn read_from_gc(gc_obj: &GcObject, idx: usize) -> anyhow::Result<Self> {
        gc_obj.get(idx)
    }
}

/// Unified macro for defining structs that work both as Rust types and WASM GC wrappers
///
/// # Usage
/// ```ignore
/// wasm_struct! {
///     Person {
///         name: String,
///         age: i64,
///     }
/// }
///
/// // Create from field values
/// let alice = Person::new("Alice", 30);
/// let bob = Person { name: "Bob".into(), age: 25 };
///
/// // Create from GcObject
/// let person = Person::from_gc(&gc_obj)?;
///
/// // Accessor methods return field values
/// let name: String = person.name()?;
/// let age: i64 = person.age()?;
///
/// // Compare with Node using is! macro with gc flag
/// is!("class Person{...}; Person{...}", alice, gc);
/// ```
#[macro_export]
macro_rules! wasm_struct {
    (
        $name:ident {
            $($field_name:ident : $field_type:ty),* $(,)?
        }
    ) => {
        #[derive(Debug, Clone)]
        pub struct $name {
            $(pub $field_name: $field_type,)*
        }

        impl $name {
            /// Create a new instance with all fields (Rust values)
            pub fn new($($field_name: impl Into<$field_type>),*) -> Self {
                Self {
                    $($field_name: $field_name.into(),)*
                }
            }

            /// Create from GcObject reference by reading all fields
            #[allow(unused_assignments)]
            pub fn from_gc(gc_obj: &$crate::gc_traits::GcObject) -> anyhow::Result<Self> {
                let mut _idx: usize = 0;
                $(
                    let $field_name: $field_type = $crate::gc_traits::GcReadable::read_from_gc(gc_obj, _idx)?;
                    _idx += 1;
                )*
                Ok(Self { $($field_name,)* })
            }
        }

        // Generate accessor methods that return field clones
        $crate::wasm_struct!(@accessors $name; $($field_name: $field_type),*);

        impl PartialEq for $name {
            fn eq(&self, other: &Self) -> bool {
                true $(&& self.$field_name == other.$field_name)*
            }
        }

        impl PartialEq<$crate::node::Node> for $name {
            fn eq(&self, other: &$crate::node::Node) -> bool {
                other.eq_gc(self)
            }
        }

        impl $crate::node::GcComparable for $name {
            fn try_from_gc(gc_obj: &$crate::gc_traits::GcObject) -> Option<Self> {
                Self::from_gc(gc_obj).ok()
            }

            fn gc_eq(&self, other: &Self) -> bool {
                self == other
            }
        }
    };

    // Generate accessor methods for each field
    (@accessors $name:ident; $field_name:ident : $field_type:ty, $($rest_name:ident : $rest_type:ty),*) => {
        impl $name {
            pub fn $field_name(&self) -> anyhow::Result<$field_type> {
                Ok(self.$field_name.clone())
            }
        }
        $crate::wasm_struct!(@accessors $name; $($rest_name: $rest_type),*);
    };

    (@accessors $name:ident; $field_name:ident : $field_type:ty) => {
        impl $name {
            pub fn $field_name(&self) -> anyhow::Result<$field_type> {
                Ok(self.$field_name.clone())
            }
        }
    };

    (@accessors $name:ident;) => {};
}

/// Object literal macro
#[macro_export]
macro_rules! obj {
    ( $($k:ident : $v:expr),* $(,)? ) => {{
        vec![
            $(
                (stringify!($k), $crate::gc_traits::ObjFieldValue::from($v)),
            )*
        ]
    }};
}

/// Magic macro that creates struct definition AND instance in one declaration
///
/// # Usage
/// ```ignore
/// // Creates both the struct type and an instance
/// let alice = wasm_object! { Person { name: String = "Alice", age: i64 = 30 } };
///
/// // Equivalent to:
/// // wasm_struct! { Person { name: String, age: i64 } }
/// // let alice = Person::new("Alice", 30);
/// ```
///
/// This is significantly more concise than defining the struct separately!
#[macro_export]
macro_rules! wasm_object {
    // Explicit type annotation syntax: field: Type = value
    ($name:ident { $($field:ident : $type:ty = $value:expr),* $(,)? }) => {{
        $crate::wasm_struct! {
            $name {
                $($field: $type),*
            }
        }
        $name::new($($value),*)
    }};
}

/// Macro for defining type-safe struct wrappers with ergonomic field access
#[macro_export]
macro_rules! gc_struct {
    // Entry point
    (
        $name:ident {
            $($field_spec:tt)*
        }
    ) => {
        pub struct $name {
            pub inner: $crate::gc_traits::GcObject,
        }

        impl $crate::gc_traits::GcStructWrapper for $name {
            fn from_gc_object(obj: $crate::gc_traits::GcObject) -> Self {
                Self { inner: obj }
            }

            fn get_inner(&self) -> &$crate::gc_traits::GcObject {
                &self.inner
            }
        }

        impl $name {
            /// Create from a GcObject
            pub fn new(obj: $crate::gc_traits::GcObject) -> Self {
                Self { inner: obj }
            }

            /// Create directly from Val, Store, and optional Instance
            #[allow(unused)]
            pub fn from_val(val: wasmtime::Val, store: wasmtime::Store<()>, instance: Option<wasmtime::Instance>) -> anyhow::Result<Self> {
                let obj = $crate::gc_traits::GcObject::new(val, store, instance)?;
                Ok(Self::new(obj))
            }

            /// Get a field with automatic type conversion
            pub fn get<T: $crate::gc_traits::FromVal, I: $crate::gc_traits::FieldIndex>(&self, field: I) -> anyhow::Result<T> {
                self.inner.get(field)
            }

            /// Generic field setter
            pub fn set_field<T: $crate::gc_traits::ToVal, I: $crate::gc_traits::FieldIndex>(&self, field: I, value: T) -> anyhow::Result<()> {
                self.inner.set_field(field, value)
            }

            /// Get nested struct as a typed wrapper
            pub fn get_as<T: $crate::gc_traits::GcStructWrapper, I: $crate::gc_traits::FieldIndex>(&self, field: I) -> anyhow::Result<T> {
                self.inner.get_as(field)
            }

            /// Get a field from a nested struct
            pub fn get_nested<T: $crate::gc_traits::FromVal, I1: $crate::gc_traits::FieldIndex, I2: $crate::gc_traits::FieldIndex>(
                &self,
                struct_field: I1,
                nested_field: I2
            ) -> anyhow::Result<T> {
                self.with_store(|store| {
                    let struct_field_idx = struct_field.to_field_index(self.inner.as_struct_ref(), &*store)?;
                    let nested_struct = self.inner.as_struct_ref().field(&mut *store, struct_field_idx)?;
                    let nested_anyref = nested_struct.unwrap_anyref()
                        .ok_or_else(|| anyhow::anyhow!("nested field is null or not a struct"))?;
                    let nested_struct_ref = nested_anyref.unwrap_struct(&*store)?;
                    let field_idx = nested_field.to_field_index(&nested_struct_ref, &*store)?;
                    let val = nested_struct_ref.field(&mut *store, field_idx)?;
                    T::from_val(val, &mut *store)
                })
            }

            /// Access the store with a closure
            pub fn with_store<F, R>(&self, f: F) -> R
            where
                F: FnOnce(&mut wasmtime::Store<()>) -> R,
            {
                self.inner.with_store(f)
            }
        }

        // Generate getters and conditional setters
        $crate::gc_struct!(@parse_fields $name; $($field_spec)*);
    };

    // Parse mutable String field
    (@parse_fields $name:ident; $field_name:ident : $field_idx:literal => mut String, $($rest:tt)*) => {
        $crate::gc_struct!(@impl_mut_string_field $name, $field_name, $field_idx);
        $crate::gc_struct!(@parse_fields $name; $($rest)*);
    };

    (@parse_fields $name:ident; $field_name:ident : $field_idx:literal => mut String) => {
        $crate::gc_struct!(@impl_mut_string_field $name, $field_name, $field_idx);
    };

    // Parse mutable field (general case)
    (@parse_fields $name:ident; $field_name:ident : $field_idx:literal => mut $field_type:ty, $($rest:tt)*) => {
        $crate::gc_struct!(@impl_mut_field $name, $field_name, $field_idx, $field_type);
        $crate::gc_struct!(@parse_fields $name; $($rest)*);
    };

    (@parse_fields $name:ident; $field_name:ident : $field_idx:literal => mut $field_type:ty) => {
        $crate::gc_struct!(@impl_mut_field $name, $field_name, $field_idx, $field_type);
    };

    // Parse immutable String field (special case for ptr/len strings)
    (@parse_fields $name:ident; $field_name:ident : $field_idx:literal => String, $($rest:tt)*) => {
        $crate::gc_struct!(@impl_string_field $name, $field_name, $field_idx);
        $crate::gc_struct!(@parse_fields $name; $($rest)*);
    };

    (@parse_fields $name:ident; $field_name:ident : $field_idx:literal => String) => {
        $crate::gc_struct!(@impl_string_field $name, $field_name, $field_idx);
    };

    // Parse immutable field
    (@parse_fields $name:ident; $field_name:ident : $field_idx:literal => $field_type:ty, $($rest:tt)*) => {
        $crate::gc_struct!(@impl_field $name, $field_name, $field_idx, $field_type);
        $crate::gc_struct!(@parse_fields $name; $($rest)*);
    };

    (@parse_fields $name:ident; $field_name:ident : $field_idx:literal => $field_type:ty) => {
        $crate::gc_struct!(@impl_field $name, $field_name, $field_idx, $field_type);
    };

    // Base case
    (@parse_fields $name:ident;) => {};

    // Implement getter + setter for mutable String field (uses get_string for instance access)
    (@impl_mut_string_field $name:ident, $field_name:ident, $field_idx:literal) => {
        paste::paste! {
            impl $name {
                pub fn $field_name(&self) -> anyhow::Result<String> {
                    self.inner.get_string($field_idx)
                }

                pub fn [<set_ $field_name>](&self, value: &str) -> anyhow::Result<()> {
                    self.inner.set_field($field_idx, value)
                }
            }
        }
    };

    // Implement getter + setter for mutable field
    (@impl_mut_field $name:ident, $field_name:ident, $field_idx:literal, $field_type:ty) => {
        paste::paste! {
            impl $name {
                pub fn $field_name(&self) -> anyhow::Result<$field_type> {
                    self.inner.get($field_idx)
                }

                pub fn [<set_ $field_name>](&self, value: $field_type) -> anyhow::Result<()> {
                    self.inner.set_field($field_idx, value)
                }
            }
        }
    };

    // Implement getter only for immutable String field (uses get_string for instance access)
    (@impl_string_field $name:ident, $field_name:ident, $field_idx:literal) => {
        impl $name {
            pub fn $field_name(&self) -> anyhow::Result<String> {
                self.inner.get_string($field_idx)
            }
        }
    };

    // Implement getter only for immutable field
    (@impl_field $name:ident, $field_name:ident, $field_idx:literal, $field_type:ty) => {
        impl $name {
            pub fn $field_name(&self) -> anyhow::Result<$field_type> {
                self.inner.get($field_idx)
            }
        }
    };
}

/// Register a WebAssembly module's GC metadata for field name lookup
pub fn register_gc_types_from_wasm(bytes: &[u8]) -> Result<()> {
    wasm_name_resolver::register_module(bytes)
}

/// WASM name resolver module for looking up field names from WASM metadata
pub mod wasm_name_resolver {
    use super::*;
    use once_cell::sync::Lazy;
    use wasmparser as wp;

    static REGISTRY: Lazy<Mutex<FieldNameRegistry>> =
        Lazy::new(|| Mutex::new(FieldNameRegistry::default()));

    pub fn register_module(bytes: &[u8]) -> Result<()> {
        let mut registry = REGISTRY.lock().expect("registry lock poisoned");
        registry.register_module(bytes)
    }

    pub fn lookup_field_index(struct_type: &StructType, field_name: &str) -> Result<usize> {
        let mut registry = REGISTRY.lock().expect("registry lock poisoned");
        registry.lookup(struct_type, field_name)
    }

    pub fn field_names(struct_type: &StructType) -> Result<Vec<Option<String>>> {
        let mut registry = REGISTRY.lock().expect("registry lock poisoned");
        registry.field_names(struct_type)
    }

    #[derive(Default)]
    struct FieldNameRegistry {
        modules: Vec<Arc<ParsedModule>>,
        cache: HashMap<StructTypeKey, Arc<StructFieldMapping>>,
    }

    impl FieldNameRegistry {
        fn register_module(&mut self, bytes: &[u8]) -> Result<()> {
            let hash = module_hash(bytes);
            if self.modules.iter().any(|m| m.hash == hash) {
                return Ok(());
            }
            let module = Arc::new(ParsedModule::parse(bytes, hash)?);
            self.modules.push(module);
            Ok(())
        }

        fn lookup(&mut self, struct_type: &StructType, field_name: &str) -> Result<usize> {
            let mapping = self.ensure_mapping(struct_type)?;
            mapping
                .index_of(field_name)
                .ok_or_else(|| unknown_field_error(field_name, &mapping))
        }

        fn field_names(&mut self, struct_type: &StructType) -> Result<Vec<Option<String>>> {
            let mapping = self.ensure_mapping(struct_type)?;
            Ok(mapping.names.clone())
        }

        fn ensure_mapping(&mut self, struct_type: &StructType) -> Result<Arc<StructFieldMapping>> {
            if self.modules.is_empty() {
                return Err(anyhow!(
                    "No WASM metadata registered. Call `register_gc_types_from_wasm` before accessing fields by name."
                ));
            }

            let key = StructTypeKey(struct_type.clone());
            if let Some(mapping) = self.cache.get(&key) {
                return Ok(mapping.clone());
            }

            for module in &self.modules {
                if let Some(mapping) = module.try_match(struct_type) {
                    let mapping = Arc::new(mapping);
                    self.cache.insert(key.clone(), mapping.clone());
                    return Ok(mapping);
                }
            }

            Err(anyhow!(
                "Failed to resolve struct fields from registered modules"
            ))
        }
    }

    fn unknown_field_error(field_name: &str, mapping: &StructFieldMapping) -> anyhow::Error {
        let mut available: Vec<&str> = mapping.available_names();
        available.sort_unstable();
        anyhow!(
            "Unknown field '{}'. Available fields: {}",
            field_name,
            if available.is_empty() {
                "<none>".to_string()
            } else {
                available.join(", ")
            }
        )
    }

    fn module_hash(bytes: &[u8]) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        let mut hasher = DefaultHasher::new();
        bytes.hash(&mut hasher);
        hasher.finish()
    }

    #[derive(Clone)]
    struct StructTypeKey(StructType);

    impl PartialEq for StructTypeKey {
        fn eq(&self, other: &Self) -> bool {
            StructType::eq(&self.0, &other.0)
        }
    }

    impl Eq for StructTypeKey {}

    impl Hash for StructTypeKey {
        fn hash<H: Hasher>(&self, state: &mut H) {
            self.0.hash(state);
        }
    }

    #[derive(Clone)]
    struct StructFieldMapping {
        names: Vec<Option<String>>,
        lookup: HashMap<String, usize>,
    }

    impl StructFieldMapping {
        fn new(parsed: &ParsedStructType) -> Self {
            let mut lookup = HashMap::new();
            for (idx, name) in parsed.field_names.iter().enumerate() {
                if let Some(name) = name {
                    lookup.insert(name.clone(), idx);
                }
            }
            Self {
                names: parsed.field_names.clone(),
                lookup,
            }
        }

        fn index_of(&self, name: &str) -> Option<usize> {
            self.lookup.get(name).copied()
        }

        fn available_names(&self) -> Vec<&str> {
            self.names.iter().filter_map(|n| n.as_deref()).collect()
        }
    }

    struct ParsedModule {
        hash: u64,
        types: Vec<ParsedTypeInfo>,
    }

    impl ParsedModule {
        fn parse(bytes: &[u8], hash: u64) -> Result<Self> {
            let mut types = Vec::new();
            let mut next_index: u32 = 0;
            let mut field_names: HashMap<u32, Vec<(u32, String)>> = HashMap::new();

            for payload in wp::Parser::new(0).parse_all(bytes) {
                match payload? {
                    wp::Payload::TypeSection(reader) => {
                        next_index = Self::read_type_section(reader, next_index, &mut types)?;
                    }
                    wp::Payload::CustomSection(section) => {
                        if let wp::KnownCustom::Name(name_reader) = section.as_known() {
                            Self::read_name_section(name_reader, &mut field_names)?;
                        }
                    }
                    _ => {}
                }
            }

            let mut module = Self { hash, types };
            module.apply_field_names(field_names);
            Ok(module)
        }

        fn read_type_section(
            reader: wp::TypeSectionReader,
            mut next_index: u32,
            types: &mut Vec<ParsedTypeInfo>,
        ) -> Result<u32> {
            for group in reader {
                let group = group?;
                let entries: Vec<(usize, wp::SubType)> = group.into_types_and_offsets().collect();
                let group_start = next_index;
                for (_offset, subtype) in entries.into_iter() {
                    let actual_index = next_index;
                    let info = ParsedTypeInfo::from_subtype(actual_index, subtype, group_start)?;
                    types.push(info);
                    next_index = actual_index + 1;
                }
            }
            Ok(next_index)
        }

        fn read_name_section(
            mut reader: wp::NameSectionReader<'_>,
            out: &mut HashMap<u32, Vec<(u32, String)>>,
        ) -> Result<()> {
            while let Some(entry) = reader.next() {
                match entry? {
                    wp::Name::Field(map) => {
                        for naming in map {
                            let naming = naming?;
                            let type_index = naming.index;
                            let mut names = Vec::new();
                            for field in naming.names {
                                let field = field?;
                                names.push((field.index, field.name.to_string()));
                            }
                            if !names.is_empty() {
                                out.entry(type_index).or_default().extend(names.into_iter());
                            }
                        }
                    }
                    _ => {}
                }
            }
            Ok(())
        }

        fn apply_field_names(&mut self, names: HashMap<u32, Vec<(u32, String)>>) {
            for (type_index, entries) in names {
                if let Some(ParsedTypeInfo::Struct(struct_ty)) =
                    self.types.get_mut(type_index as usize)
                {
                    for (field_idx, name) in entries {
                        if let Some(slot) = struct_ty.field_names.get_mut(field_idx as usize) {
                            *slot = Some(name);
                        }
                    }
                }
            }
        }

        fn try_match(&self, struct_type: &StructType) -> Option<StructFieldMapping> {
            for (idx, ty) in self.types.iter().enumerate() {
                if let ParsedTypeInfo::Struct(parsed_struct) = ty {
                    let mut ctx = MatchContext::new(self);
                    if ctx.match_struct(idx as u32, struct_type) {
                        return Some(StructFieldMapping::new(parsed_struct));
                    }
                }
            }
            None
        }

        fn parsed_type(&self, idx: u32) -> Option<&ParsedTypeInfo> {
            self.types.get(idx as usize)
        }
    }

    enum ParsedTypeInfo {
        Struct(ParsedStructType),
        Array(ParsedArrayType),
        Func(ParsedFuncType),
        Other,
    }

    impl ParsedTypeInfo {
        fn from_subtype(
            type_index: u32,
            subtype: wp::SubType,
            group_start: u32,
        ) -> Result<Self> {
            use wp::CompositeInnerType::*;
            match subtype.composite_type.inner {
                Struct(ty) => Ok(Self::Struct(ParsedStructType::from_parser(
                    type_index,
                    ty,
                    group_start,
                )?)),
                Array(ty) => Ok(Self::Array(ParsedArrayType::from_parser(
                    type_index,
                    ty,
                    group_start,
                )?)),
                Func(ty) => Ok(Self::Func(ParsedFuncType::from_parser(
                    type_index,
                    ty,
                    group_start,
                )?)),
                _ => Ok(Self::Other),
            }
        }
    }

    #[derive(Clone)]
    struct ParsedStructType {
        #[allow(dead_code)]
        type_index: u32,
        fields: Vec<ParsedField>,
        field_names: Vec<Option<String>>,
    }

    impl ParsedStructType {
        fn from_parser(type_index: u32, ty: wp::StructType, group_start: u32) -> Result<Self> {
            let mut fields = Vec::with_capacity(ty.fields.len());
            for field in ty.fields.iter() {
                fields.push(ParsedField::from_parser(field, group_start)?);
            }
            let field_names = vec![None; fields.len()];
            Ok(Self {
                type_index,
                fields,
                field_names,
            })
        }
    }

    #[derive(Clone)]
    struct ParsedArrayType {
        #[allow(dead_code)]
        type_index: u32,
        field: ParsedField,
    }

    impl ParsedArrayType {
        fn from_parser(type_index: u32, ty: wp::ArrayType, group_start: u32) -> Result<Self> {
            Ok(Self {
                type_index,
                field: ParsedField::from_parser(&ty.0, group_start)?,
            })
        }
    }

    #[derive(Clone)]
    struct ParsedFuncType {
        #[allow(dead_code)]
        type_index: u32,
        params: Vec<ParsedValType>,
        results: Vec<ParsedValType>,
    }

    impl ParsedFuncType {
        fn from_parser(type_index: u32, ty: wp::FuncType, group_start: u32) -> Result<Self> {
            let params = ty
                .params()
                .iter()
                .map(|p| ParsedValType::from_parser(p, group_start))
                .collect::<Result<Vec<_>>>()?;
            let results = ty
                .results()
                .iter()
                .map(|r| ParsedValType::from_parser(r, group_start))
                .collect::<Result<Vec<_>>>()?;
            Ok(Self {
                type_index,
                params,
                results,
            })
        }
    }

    #[derive(Clone)]
    struct ParsedField {
        mutable: bool,
        storage: ParsedStorageType,
    }

    impl ParsedField {
        fn from_parser(field: &wp::FieldType, group_start: u32) -> Result<Self> {
            Ok(Self {
                mutable: field.mutable,
                storage: ParsedStorageType::from_parser(&field.element_type, group_start)?,
            })
        }
    }

    #[derive(Clone)]
    enum ParsedStorageType {
        I8,
        I16,
        Val(ParsedValType),
    }

    impl ParsedStorageType {
        fn from_parser(ty: &wp::StorageType, group_start: u32) -> Result<Self> {
            Ok(match ty {
                wp::StorageType::I8 => ParsedStorageType::I8,
                wp::StorageType::I16 => ParsedStorageType::I16,
                wp::StorageType::Val(v) => {
                    ParsedStorageType::Val(ParsedValType::from_parser(v, group_start)?)
                }
            })
        }
    }

    #[derive(Clone)]
    enum ParsedValType {
        I32,
        I64,
        F32,
        F64,
        V128,
        Ref(ParsedRefType),
    }

    impl ParsedValType {
        fn from_parser(ty: &wp::ValType, group_start: u32) -> Result<Self> {
            use wp::ValType::*;
            Ok(match ty {
                I32 => ParsedValType::I32,
                I64 => ParsedValType::I64,
                F32 => ParsedValType::F32,
                F64 => ParsedValType::F64,
                V128 => ParsedValType::V128,
                Ref(r) => ParsedValType::Ref(ParsedRefType::from_parser(r, group_start)?),
            })
        }
    }

    #[derive(Clone)]
    struct ParsedRefType {
        nullable: bool,
        heap: ParsedHeapType,
    }

    impl ParsedRefType {
        fn from_parser(ty: &wp::RefType, group_start: u32) -> Result<Self> {
            Ok(Self {
                nullable: ty.is_nullable(),
                heap: ParsedHeapType::from_parser(&ty.heap_type(), group_start)?,
            })
        }
    }

    #[derive(Clone)]
    enum ParsedHeapType {
        Abstract {
            #[allow(dead_code)]
            shared: bool,
            kind: ParsedAbstractHeapType,
        },
        Concrete(u32),
    }

    impl ParsedHeapType {
        fn from_parser(ty: &wp::HeapType, group_start: u32) -> Result<Self> {
            Ok(match ty {
                wp::HeapType::Abstract { shared, ty } => ParsedHeapType::Abstract {
                    shared: *shared,
                    kind: ParsedAbstractHeapType::from_parser(*ty),
                },
                wp::HeapType::Concrete(idx) => {
                    let resolved = resolve_index(*idx, group_start)?;
                    ParsedHeapType::Concrete(resolved)
                }
                wp::HeapType::Exact(idx) => {
                    let resolved = resolve_index(*idx, group_start)?;
                    ParsedHeapType::Concrete(resolved)
                }
            })
        }
    }

    #[derive(Clone, Copy)]
    enum ParsedAbstractHeapType {
        Func,
        Extern,
        Any,
        None,
        NoExtern,
        NoFunc,
        Eq,
        Struct,
        Array,
        I31,
        #[allow(dead_code)]
        Exn,
        #[allow(dead_code)]
        NoExn,
        #[allow(dead_code)]
        Cont,
        #[allow(dead_code)]
        NoCont,
    }

    impl ParsedAbstractHeapType {
        fn from_parser(ty: wp::AbstractHeapType) -> Self {
            use wp::AbstractHeapType::*;
            match ty {
                Func => ParsedAbstractHeapType::Func,
                Extern => ParsedAbstractHeapType::Extern,
                Any => ParsedAbstractHeapType::Any,
                None => ParsedAbstractHeapType::None,
                NoExtern => ParsedAbstractHeapType::NoExtern,
                NoFunc => ParsedAbstractHeapType::NoFunc,
                Eq => ParsedAbstractHeapType::Eq,
                Struct => ParsedAbstractHeapType::Struct,
                Array => ParsedAbstractHeapType::Array,
                I31 => ParsedAbstractHeapType::I31,
                Exn => ParsedAbstractHeapType::Exn,
                NoExn => ParsedAbstractHeapType::NoExn,
                Cont => ParsedAbstractHeapType::Cont,
                NoCont => ParsedAbstractHeapType::NoCont,
            }
        }
    }

    fn resolve_index(index: wp::UnpackedIndex, group_start: u32) -> Result<u32> {
        Ok(match index {
            wp::UnpackedIndex::Module(i) => i,
            wp::UnpackedIndex::RecGroup(i) => group_start + i,
            _ => bail!("unsupported canonical type index"),
        })
    }

    struct MatchContext<'a> {
        module: &'a ParsedModule,
        bindings: HashMap<u32, RuntimeTypeId>,
    }

    impl<'a> MatchContext<'a> {
        fn new(module: &'a ParsedModule) -> Self {
            Self {
                module,
                bindings: HashMap::new(),
            }
        }

        fn match_struct(&mut self, idx: u32, ty: &StructType) -> bool {
            if let Some(existing) = self.bindings.get(&idx) {
                return existing.matches_struct(ty);
            }
            let parsed = match self.module.parsed_type(idx) {
                Some(ParsedTypeInfo::Struct(s)) => s,
                _ => return false,
            };
            self.bindings.insert(idx, RuntimeTypeId::Struct(ty.clone()));
            self.compare_struct(parsed, ty)
        }

        fn compare_struct(&mut self, parsed: &ParsedStructType, ty: &StructType) -> bool {
            let mut runtime_fields = ty.fields();
            if parsed.fields.len() != runtime_fields.len() {
                return false;
            }
            for (parsed_field, runtime_field) in parsed.fields.iter().zip(runtime_fields.by_ref())
            {
                if parsed_field.mutable != runtime_field.mutability().is_var() {
                    return false;
                }
                if !self.match_storage_type(&parsed_field.storage, runtime_field.element_type()) {
                    return false;
                }
            }
            true
        }

        fn match_array(&mut self, idx: u32, ty: &ArrayType) -> bool {
            if let Some(existing) = self.bindings.get(&idx) {
                return existing.matches_array(ty);
            }
            let parsed = match self.module.parsed_type(idx) {
                Some(ParsedTypeInfo::Array(a)) => a,
                _ => return false,
            };
            self.bindings.insert(idx, RuntimeTypeId::Array(ty.clone()));
            let runtime_field = ty.field_type();
            parsed.field.mutable == runtime_field.mutability().is_var()
                && self.match_storage_type(&parsed.field.storage, runtime_field.element_type())
        }

        fn match_func(&mut self, idx: u32, ty: &FuncType) -> bool {
            if let Some(existing) = self.bindings.get(&idx) {
                return existing.matches_func(ty);
            }
            let parsed = match self.module.parsed_type(idx) {
                Some(ParsedTypeInfo::Func(f)) => f,
                _ => return false,
            };
            self.bindings.insert(idx, RuntimeTypeId::Func(ty.clone()));
            if parsed.params.len() != ty.params().len()
                || parsed.results.len() != ty.results().len()
            {
                return false;
            }
            for (parsed_val, runtime_val) in parsed.params.iter().zip(ty.params()) {
                if !self.match_val_type(parsed_val, &runtime_val) {
                    return false;
                }
            }
            for (parsed_val, runtime_val) in parsed.results.iter().zip(ty.results()) {
                if !self.match_val_type(parsed_val, &runtime_val) {
                    return false;
                }
            }
            true
        }

        fn match_storage_type(&mut self, parsed: &ParsedStorageType, ty: &StorageType) -> bool {
            match (parsed, ty) {
                (ParsedStorageType::I8, StorageType::I8)
                | (ParsedStorageType::I16, StorageType::I16) => true,
                (ParsedStorageType::Val(p), StorageType::ValType(v)) => self.match_val_type(p, v),
                _ => false,
            }
        }

        fn match_val_type(&mut self, parsed: &ParsedValType, ty: &ValType) -> bool {
            use ParsedValType::*;
            match (parsed, ty) {
                (I32, ValType::I32)
                | (I64, ValType::I64)
                | (F32, ValType::F32)
                | (F64, ValType::F64)
                | (V128, ValType::V128) => true,
                (Ref(parsed_ref), ValType::Ref(runtime_ref)) => {
                    if parsed_ref.nullable != runtime_ref.is_nullable() {
                        return false;
                    }
                    self.match_heap_type(&parsed_ref.heap, &runtime_ref.heap_type())
                }
                _ => false,
            }
        }

        fn match_heap_type(&mut self, parsed: &ParsedHeapType, ty: &HeapType) -> bool {
            match (parsed, ty) {
                (ParsedHeapType::Abstract { kind, .. }, _) => matches_abstract_heap(*kind, ty),
                (ParsedHeapType::Concrete(idx), HeapType::ConcreteStruct(s)) => {
                    self.match_struct(*idx, s)
                }
                (ParsedHeapType::Concrete(idx), HeapType::ConcreteArray(a)) => {
                    self.match_array(*idx, a)
                }
                (ParsedHeapType::Concrete(idx), HeapType::ConcreteFunc(f)) => {
                    self.match_func(*idx, f)
                }
                _ => false,
            }
        }
    }

    fn matches_abstract_heap(kind: ParsedAbstractHeapType, ty: &HeapType) -> bool {
        use ParsedAbstractHeapType::*;
        match (kind, ty) {
            (Extern, HeapType::Extern)
            | (NoExtern, HeapType::NoExtern)
            | (Func, HeapType::Func)
            | (NoFunc, HeapType::NoFunc)
            | (Any, HeapType::Any)
            | (None, HeapType::None)
            | (Eq, HeapType::Eq)
            | (Struct, HeapType::Struct)
            | (Array, HeapType::Array)
            | (I31, HeapType::I31) => true,
            _ => false,
        }
    }

    enum RuntimeTypeId {
        Struct(StructType),
        Array(ArrayType),
        Func(FuncType),
    }

    impl RuntimeTypeId {
        fn matches_struct(&self, ty: &StructType) -> bool {
            matches!(self, RuntimeTypeId::Struct(existing) if StructType::eq(existing, ty))
        }

        fn matches_array(&self, ty: &ArrayType) -> bool {
            matches!(self, RuntimeTypeId::Array(existing) if ArrayType::eq(existing, ty))
        }

        fn matches_func(&self, ty: &FuncType) -> bool {
            matches!(self, RuntimeTypeId::Func(existing) if FuncType::eq(existing, ty))
        }
    }
}
