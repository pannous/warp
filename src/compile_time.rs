//! What the compile-time evaluators of whole programs (real.rs, units.rs) share: why an evaluation stops and its answer
use crate::node::{error, Node};

/// Why a compile-time evaluation stops
pub(crate) enum Stop {
	/// not a program this evaluator answers: compile it normally
	Unsupported,
	Error(String),
}

pub(crate) fn fail<T>(message: impl Into<String>) -> Result<T, Stop> {
	Err(Stop::Error(message.into()))
}

/// The program's value as a node, its error, or None to compile it normally
pub(crate) fn answer_of<Value>(evaluated: Result<Value, Stop>, node_of: impl FnOnce(Value) -> Node) -> Option<Node> {
	match evaluated {
		Ok(value) => Some(node_of(value)),
		Err(Stop::Error(message)) => Some(error(&message)),
		Err(Stop::Unsupported) => None,
	}
}
