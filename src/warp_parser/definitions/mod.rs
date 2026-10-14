//! Declarations: functions, lambdas, variables, foreign modifiers

use super::*;

mod foreign;
mod functions;
mod lambdas;
mod operators;
mod variables;
pub(crate) use functions::*;
pub(crate) use lambdas::*;
pub(crate) use operators::*;
pub(crate) use variables::*;
