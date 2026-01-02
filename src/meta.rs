use crate::node::DataType;
use serde::ser::SerializeStruct;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::any::Any;
use std::fmt;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct MetaData {
    pub comment: Option<String>,
    pub line: Option<usize>,
    pub column: Option<usize>,
}

impl MetaData {
    pub fn new() -> Self {
        MetaData {
            comment: None,
            line: None,
            column: None,
        }
    }

    pub fn with_comment(comment: String) -> Self {
        MetaData {
            comment: Some(comment),
            line: None,
            column: None,
        }
    }

    pub fn with_position(line: usize, column: usize) -> Self {
        MetaData {
            comment: None,
            line: Some(line),
            column: Some(column),
        }
    }
}

// Custom trait for cloneable Any types with equality support
pub trait CloneAny: Any {
    fn clone_any(&self) -> Box<dyn CloneAny>;
    fn as_any(&self) -> &dyn Any;
    fn eq_any(&self, other: &dyn CloneAny) -> bool;
}

impl<T: 'static + Clone + PartialEq> CloneAny for T {
    fn clone_any(&self) -> Box<dyn CloneAny> {
        Box::new(self.clone())
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn eq_any(&self, other: &dyn CloneAny) -> bool {
        if let Some(other_t) = other.as_any().downcast_ref::<T>() {
            self == other_t
        } else {
            false
        }
    }
}

pub struct Dada {
    pub(crate) data: Box<dyn CloneAny>,
    pub type_name: String,
    pub data_type: DataType,
}

// most generic container for any kind of data not captured by other node types
// Vec, tuples, primitives, custom structs, etc.
// let v = Node::data(vec![1, 2, 3]);
// let t = Node::data((42, "answer"));
// let n = Node::data(CustomData { id: 42, name: "test" });
// 💡 let extract = dada.downcast_ref::<MyType>(); 💡
impl Dada {
    pub fn new<T: 'static + Clone + PartialEq>(data: T) -> Self {
        let type_name = std::any::type_name::<T>().to_string();
        let data_type = Self::infer_type(&type_name);
        Dada {
            data: Box::new(data),
            type_name,
            data_type,
        }
    }

    pub fn downcast_ref<T: 'static>(&self) -> Option<&T> {
        self.data.as_any().downcast_ref::<T>()
    }

    fn infer_type(type_name: &str) -> DataType {
        if type_name.starts_with("alloc::vec::Vec") || type_name.starts_with("std::vec::Vec") {
            DataType::Vec
        } else if type_name.starts_with('(') && type_name.ends_with(')') {
            DataType::Tuple
        } else if type_name.contains("::String") || type_name == "str" || type_name == "&str" {
            DataType::String // map to Node(String) early!
        } else if type_name.contains("::") {
            DataType::Struct
        } else if matches!(
            type_name,
            "i8" | "i16"
                | "i32"
                | "i64"
                | "i128"
                | "isize"
                | "u8"
                | "u16"
                | "u32"
                | "u64"
                | "u128"
                | "usize"
                | "f32"
                | "f64"
                | "bool"
                | "char"
        ) {
            DataType::Primitive
        } else {
            DataType::Other
        }
    }

}

impl Clone for Dada {
    fn clone(&self) -> Self {
        Dada {
            data: self.data.clone_any(),
            type_name: self.type_name.clone(),
            data_type: self.data_type.clone(),
        }
    }
}

impl fmt::Debug for Dada {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Dada({:?}:{})", self.data_type, self.type_name)
    }
}

impl PartialEq for Dada {
    fn eq(&self, other: &Self) -> bool {
        self.data.eq_any(other.data.as_ref())
    }
}

impl Serialize for Dada {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("Dada", 2)?;
        state.serialize_field("type_name", &self.type_name)?;
        state.serialize_field("data_type", &self.data_type)?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for Dada {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct DadaHelper {
            type_name: String,
            data_type: DataType,
        }

        let helper = DadaHelper::deserialize(deserializer)?;
        // Create a placeholder Dada with empty string
        Ok(Dada {
            data: Box::new(String::from("<deserialized>")),
            type_name: helper.type_name,
            data_type: helper.data_type,
        })
    }
}
