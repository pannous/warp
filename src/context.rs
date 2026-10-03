use crate::ffi::FfiSignature;
use crate::function::FunctionRegistry;
use crate::node::Node;
use crate::type_kinds::{Kind, TypeRegistry};
use std::collections::{HashMap, HashSet};

/// A function parameter: its name, the type node of an explicit `x:type` annotation and an optional per-call default
#[derive(Clone, Debug)]
pub struct Param {
    pub name: String,
    pub annotation: Option<Node>,
    pub default: Option<Node>,
    /// The kind the function body demands of an undeclared parameter, e.g. List for `xs#2`
    pub used_as: Option<Kind>,
}

impl Param {
    pub fn untyped(name: &str) -> Self {
        Param { name: name.to_string(), annotation: None, default: None, used_as: None }
    }
}

/// User-defined function definition
#[derive(Clone, Debug)]
pub struct UserFunctionDef {
    pub name: String,
    pub params: Vec<Param>,
    pub body: Box<Node>,
    pub return_kind: Kind,
    /// Kinds of the values a `return a, b` function returns as wasm multi-value results (empty: one Node or number)
    pub tuple_kinds: Vec<Kind>,
    pub func_index: Option<u32>,
}

/// A variable captured by a closure: the global holding its value at definition time, and its kind
pub type Capture = (String, (u32, Kind));

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
    /// The program's `global` declarations with kind and type, which every function body sees
    pub declared_globals: HashMap<String, crate::local::Local>,
    pub user_functions: HashMap<String, UserFunctionDef>,
    /// Per function: captured outer variable → (global holding its value at definition time, kind)
    pub captures: HashMap<String, Vec<Capture>>,
    /// Calls that disagree on the kind of an undeclared parameter, reported as type errors
    pub parameter_conflicts: Vec<String>,
    /// Functions made into closures (closures.rs): (function, number of captured values leading its parameters)
    pub closure_targets: Vec<(String, usize)>,
    /// Field names looked up by a constant key (`p.x`, `p["x"]`): each has a runtime error `no_field_x` for the miss
    pub missing_field_names: std::collections::BTreeSet<String>,
    /// Subjects of a `switch` without default: each has a runtime error `no_case_<subject>` for the miss
    pub missing_case_labels: std::collections::BTreeSet<String>,
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
            declared_globals: HashMap::new(),
            captures: HashMap::new(),
            user_functions: HashMap::new(),
            parameter_conflicts: Vec::new(),
            closure_targets: Vec::new(),
            missing_field_names: Default::default(),
            missing_case_labels: Default::default(),
        }
    }

}
