use crate::analyzer::Scope;
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
pub struct EmitterContext {
    pub func_registry: FunctionRegistry,
    pub used_functions: HashSet<&'static str>,
    pub required_functions: HashSet<&'static str>,
    pub emit_all_functions: bool,
    pub emit_kind_globals: bool,
    pub emit_host_imports: bool,
    pub emit_wasi_imports: bool,
    pub emit_ffi_imports: bool,
    pub ffi_imports: HashMap<String, FfiSignature>,
    pub kind_global_indices: HashMap<Kind, u32>,
    pub string_table: HashMap<String, u32>,
    pub next_data_offset: u32,
    pub scope: Scope,
    pub next_temp_local: u32,
    pub user_type_indices: HashMap<String, u32>,
    pub type_registry: TypeRegistry,
    pub user_globals: HashMap<String, (u32, Kind)>,
    pub user_functions: HashMap<String, UserFunctionDef>,
}

impl Default for EmitterContext {
    fn default() -> Self {
        Self::new()
    }
}

impl EmitterContext {
    pub fn new() -> Self {
        EmitterContext {
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
            emit_all_functions: true,
            emit_kind_globals: true,
            emit_host_imports: false,
            emit_wasi_imports: false,
            emit_ffi_imports: false,
            ffi_imports: HashMap::new(),
            kind_global_indices: HashMap::new(),
            string_table: HashMap::new(),
            next_data_offset: 8,
            scope: Scope::new(),
            next_temp_local: 0,
            user_type_indices: HashMap::new(),
            type_registry: TypeRegistry::new(),
            user_globals: HashMap::new(),
            user_functions: HashMap::new(),
        }
    }
}
