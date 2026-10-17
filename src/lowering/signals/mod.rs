//! Signals: values that change over time and the passes that wire their listeners (events, fetches, system values)

// the passes here name their siblings in src/lowering as `super::x`, as they did before the folder
#[allow(unused_imports)]
use super::*;

pub mod event_signals;
pub mod fetch_signals;
pub mod signal_values;
pub mod system_signals;
pub mod system_values;
pub mod variable_signals;
