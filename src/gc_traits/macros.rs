//! GcReadable and the wrapper macros on GcObject: wasm_struct!, obj!, wasm_object!, gc_struct!

use super::*;

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

/// Numbers and bools read through `get`
macro_rules! gc_readable_by_get {
    ($($readable:ty),*) => {$(
        impl GcReadable for $readable {
            fn read_from_gc(gc_obj: &GcObject, idx: usize) -> anyhow::Result<Self> {
                gc_obj.get(idx)
            }
        }
    )*};
}
gc_readable_by_get!(i32, i64, f32, f64, bool);

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
