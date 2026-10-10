//! GC Traits - Ergonomic WebAssembly GC struct access
//!
//! Ported from rasm project for type-safe, named field access to WASM GC structs.
//!
//! # Usage
//! ```ignore
//! use warp::gc_traits::GcObject;
//! use warp::gc_struct;
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


use anyhow::{anyhow, bail, Result};
use std::cell::RefCell;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
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
        Ok(warp_runtime::floats::canonical_nan(val.unwrap_f64()))
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

/// The reference a value holds, an error for a number or null
pub(crate) fn anyref_of(val: &Val) -> Result<Rooted<AnyRef>> {
    val.unwrap_anyref().copied().ok_or_else(|| anyhow!("not an anyref"))
}

impl FromVal for Rooted<StructRef> {
    fn from_val(val: Val, store: &mut Store<()>) -> Result<Self> {
        let anyref = anyref_of(&val)?;
        Ok(anyref.unwrap_struct(&*store)?)
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
        Ok(Val::AnyRef(Some((*self).into())))
    }
}

impl ToVal for Val {
    fn to_val(&self, _store: &mut Store<()>, _instance: Option<&Instance>) -> Result<Val> {
        Ok(*self)
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
        wasm_name_resolver::lookup_field_index(&struct_type, self, None)
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
    /// Module this object originated from; disambiguates structurally identical
    /// types (e.g. two classes with the same field layout) in the global registry
    module: Option<u64>,
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

            // Try to get type name and field names from registered metadata
            let type_name = wasm_name_resolver::type_name(&struct_type, self.module).ok().flatten();
            let field_names = wasm_name_resolver::field_names(&struct_type, self.module).ok();
            let field_count = struct_type.fields().len();

            // Format: GcObject{TypeName{field:value ...}} or GcObject{field:value ...}
            write!(f, "GcObject{{")?;
            if let Some(ref name) = type_name {
                write!(f, "{}{{", name)?;
            }

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

            if type_name.is_some() {
                write!(f, "}}")?;
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
        let anyref = anyref_of(&val)?;
        let inner = anyref.unwrap_struct(&store)?;
        Ok(Self {
            inner,
            store: Rc::new(RefCell::new(store)),
            instance,
            module: None,
        })
    }

    /// Tie this object to the module it was produced by (see `register_gc_types_from_wasm`)
    pub fn with_module(mut self, module: Option<u64>) -> Self {
        self.module = module;
        self
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
            module: None,
        }
    }

    /// Access the store mutably
    pub fn with_store<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&mut Store<()>) -> R,
    {
        f(&mut self.store.borrow_mut())
    }

    /// `read` of the field's value and index (a field index or name)
    fn with_field<I: FieldIndex, R>(&self, field: I, read: impl FnOnce(Val, usize, &mut Store<()>) -> Result<R>) -> Result<R> {
        self.with_store(|store| {
            let idx = field.to_field_index(&self.inner, &*store)?;
            let val = self.inner.field(&mut *store, idx)?;
            read(val, idx, store)
        })
    }

    /// Get a field with automatic type conversion (supports both index and field name)
    pub fn get<T: FromVal, I: FieldIndex>(&self, field: I) -> Result<T> {
        self.with_field(field, |val, _, store| T::from_val(val, store))
    }

    /// Get a string field with instance access for ptr/len strings
    pub fn get_string<I: FieldIndex>(&self, field: I) -> Result<String> {
        self.with_field(field, |val, _, store| {
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
        self.with_field(index, |val, idx, store| {
            let anyref = val
                .unwrap_anyref()
                .ok_or_else(|| anyhow!("field {} is not an anyref", idx))?;
            Ok(anyref.unwrap_struct(&*store)?)
        })
    }

    /// Get nested struct as a GcObject that shares the same store
    pub fn get_struct_object<I: FieldIndex>(&self, index: I) -> Result<GcObject> {
        let struct_ref = self.get_struct(index)?;
        Ok(GcObject::from_struct_shared(
            struct_ref,
            self.store.clone(),
            self.instance,
        )
        .with_module(self.module))
    }

    /// Get nested struct as a wrapped type (Person, Point, etc.)
    pub fn get_as<T: GcStructWrapper, I: FieldIndex>(&self, index: I) -> Result<T> {
        let gc_obj = self.get_struct_object(index)?;
        Ok(T::from_gc_object(gc_obj))
    }

    /// Check if a field is null
    pub fn is_null<I: FieldIndex>(&self, index: I) -> Result<bool> {
        self.with_field(index, |val, _, _| Ok(val.unwrap_anyref().is_none()))
    }

    /// Check if a field is not null
    pub fn has<I: FieldIndex>(&self, index: I) -> Result<bool> {
        Ok(!self.is_null(index)?)
    }

    /// Set a field value (for mutable fields)
    pub fn set_field<T: ToVal, I: FieldIndex>(&self, field: I, value: T) -> Result<()> {
        self.with_store(|store| {
            let idx = field.to_field_index(&self.inner, &*store)?;
            let val = value.to_val(&mut *store, self.instance.as_ref())?;
            Ok(self.inner.set_field(&mut *store, idx, val)?)
        })
    }

    /// Get the inner StructRef for passing as a field value
    pub fn as_struct_ref(&self) -> &Rooted<StructRef> {
        &self.inner
    }

    /// Convert to Val for passing to WASM functions
    pub fn to_val(&self) -> Val {
        Val::AnyRef(Some(self.inner.into()))
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

mod gc_string;
mod macros;
pub use gc_string::*;
pub use macros::*;

/// Register a WebAssembly module's GC metadata for field name lookup
/// Returns the module id to pass to `GcObject::with_module` for unambiguous name lookup
pub fn register_gc_types_from_wasm(bytes: &[u8]) -> Result<u64> {
    wasm_name_resolver::register_module(bytes)
}

/// WASM name resolver module for looking up field names from WASM metadata
pub mod wasm_name_resolver;
