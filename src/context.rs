use crate::ffi::FfiSignature;
use crate::function::FunctionRegistry;
use crate::node::Node;
use crate::type_kinds::{Kind, TypeRegistry};
use std::collections::{HashMap, HashSet};

/// User-defined function definition
#[derive(Clone, Debug)]
pub struct UserFunctionDef {
    pub name: String,
    pub params: Vec<(String, Option<Node>)>,
    pub body: Box<Node>,
    pub return_kind: Kind,
    pub func_index: Option<u32>,
}

/// Compilation context for WASM GC emission
/// Contains state that tracks functions, types, variables, and strings during compilation
/// GLOBAL module scope containing several function scopes.
pub struct Context {
    pub func_registry: FunctionRegistry,
    pub used_functions: HashSet<&'static str>,
    pub required_functions: HashSet<&'static str>,
    pub ffi_imports: HashMap<String, FfiSignature>,
    pub kind_global_indices: HashMap<Kind, u32>,
    pub string_table: HashMap<String, u32>,
    pub user_type_indices: HashMap<String, u32>,
    pub type_registry: TypeRegistry,
    pub user_globals: HashMap<String, (u32, Kind)>,
    pub user_functions: HashMap<String, UserFunctionDef>,
}

impl Default for Context {
    fn default() -> Self {
        Self::new()
    }
}

impl Context {
    pub fn new() -> Self {
        Context {
            func_registry: FunctionRegistry::new(),
            used_functions: HashSet::new(),
            required_functions: HashSet::from([
                "new_empty",
                "new_int",
                "new_float",
                "new_text",
                "new_symbol",
                "new_codepoint",
                "new_key",
                "new_list",
            ]),
            ffi_imports: HashMap::new(),
            kind_global_indices: HashMap::new(),
            string_table: HashMap::new(),
            user_type_indices: HashMap::new(),
            type_registry: TypeRegistry::new(),
            user_globals: HashMap::new(),
            user_functions: HashMap::new(),
        }
    }

}
