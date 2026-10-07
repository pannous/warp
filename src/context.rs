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
    /// `use { memory, table } from "env"`: the memory and table the module imports instead of defining, (module, name)
    pub imported_entities: Vec<(String, String)>,
    pub kind_global_indices: HashMap<Kind, u32>,
    pub string_table: HashMap<String, u32>,
    pub user_type_indices: HashMap<String, u32>,
    pub type_registry: TypeRegistry,
    pub user_globals: HashMap<String, (u32, Kind)>,
    /// The program's `global` declarations with kind and type, which every function body sees
    pub declared_globals: HashMap<String, crate::local::Local>,
    /// Declared kinds of the fields of the program's types, keyed by analyzer::field_kind_key (`.x` → Float)
    pub field_kinds: HashMap<String, Kind>,
    pub user_functions: std::collections::BTreeMap<String, UserFunctionDef>,
    /// Per function: captured outer variable → (global holding its value at definition time, kind)
    pub captures: HashMap<String, Vec<Capture>>,
    /// Per function: the captured variables' bindings with their types (`k = {a: 10}`: `k.a` is an Int in the body)
    pub capture_bindings: HashMap<String, Vec<crate::local::Local>>,
    /// A function defined in a function body (`outer·inner`) → the function whose body defines it (`outer`)
    pub enclosing_functions: HashMap<String, String>,
    /// Calls that disagree on the kind of an undeclared parameter, reported as type errors
    pub parameter_conflicts: Vec<String>,
    /// Functions made into closures (closures.rs): (function, number of captured values leading its parameters)
    pub closure_targets: Vec<(String, usize)>,
    /// Variables that hold a closure: the `closure_new` targets each may contain (per-site result kinds)
    pub closure_variable_targets: HashMap<String, HashSet<String>>,
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
                "new_bool",
                "as_bool",
                "new_float",
                "new_text",
                "new_symbol",
                "new_codepoint",
                "new_key",
                "new_list",
            ]),
            ffi_imports: HashMap::new(),
            imported_entities: Vec::new(),
            kind_global_indices: HashMap::new(),
            string_table: HashMap::new(),
            user_type_indices: HashMap::new(),
            type_registry: TypeRegistry::new(),
            user_globals: HashMap::new(),
            declared_globals: HashMap::new(),
            field_kinds: HashMap::new(),
            captures: HashMap::new(),
            capture_bindings: HashMap::new(),
            enclosing_functions: HashMap::new(),
            user_functions: std::collections::BTreeMap::new(),
            parameter_conflicts: Vec::new(),
            closure_targets: Vec::new(),
            closure_variable_targets: HashMap::new(),
            missing_field_names: Default::default(),
            missing_case_labels: Default::default(),
        }
    }

}
