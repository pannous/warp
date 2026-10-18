//! Definitions: functions, lambdas and closures, parameters, overloads, variables and their mutation

// the passes here name their siblings in src/lowering as `super::x`, as they did before the folder
#[allow(unused_imports)]
use super::*;

pub mod bare_calls;
pub mod closures;
pub mod declaration_forms;
pub mod declarations;
pub mod file_declarations;
pub mod function_values;
pub mod getters;
pub mod inlining;
pub mod lambdas;
pub mod late_binding;
pub mod memoization;
pub mod mutation;
pub mod named_arguments;
pub mod nonlocal_cells;
pub mod overloads;
pub mod parameter_patterns;
pub mod parameter_shapes;
pub mod preposition_calls;
pub mod references;
pub mod specialising;
pub mod typed_block_arguments;
pub mod undo_history;
pub mod variadic;
