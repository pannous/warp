//! A failed run as the error value it means: a trap is a runtime error of the program (abi/traps.rs trap_error), a
//! task's failure its own words, an out-of-time or out-of-fuel run says so (card rust-bootstrap: harness, not compiler)
use crate::abi::traps::trap_error;
use crate::node::Node;

/// A trap is a runtime error of the program; any other failure (link, instantiation, validation) is an error
/// of the compiler or the environment. Both become error values, never the unevaluated program.
pub(crate) fn failed_run(failure: anyhow::Error) -> Node {
	if let Some(task) = failure.chain().find_map(|cause| cause.downcast_ref::<crate::host::tasks::TaskFailure>()) {
		return crate::node::error(&task.0);
	}
	if let Some(out_of_time) = failure.chain().find_map(|cause| cause.downcast_ref::<warp_runtime::engine::OutOfTime>())
	{
		return crate::errors::run_time::out_of_time(out_of_time.0);
	}
	let trap = match failure.downcast_ref::<wasmtime::Trap>() {
		None => return crate::node::error(&format!("could not run the program: {failure:#}")),
		Some(wasmtime::Trap::OutOfFuel) => return crate::errors::run_time::out_of_fuel(crate::util::fuel_budget()),
		Some(trap) => trap,
	};
	trap_error(&format!("{:?}", failure), trap.to_string())
}
