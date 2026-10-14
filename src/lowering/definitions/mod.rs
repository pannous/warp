//! Definitions: functions, lambdas and closures, parameters, overloads, variables and their mutation

// the passes here name their siblings in src/lowering as `super::x`, as they did before the folder
#[allow(unused_imports)]
use super::*;

pub mod declaration_forms;
pub mod definition_cases;
pub mod file_declarations;
pub mod overloads;
pub mod lambdas;
pub mod closures;
pub mod function_values;
pub mod named_arguments;
pub mod parameter_patterns;
pub mod parameter_shapes;
pub mod variadic;
pub mod inlining;
pub mod memoization;
pub mod late_binding;
pub mod getters;
pub mod nonlocal_cells;
pub mod mutation;
pub mod references;
pub mod undo_history;
