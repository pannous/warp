use std::time::Duration;

/// A run stops after this much computing (the CPU time of the thread running it: sleeping and waiting for input don't
/// count) with `out of time`, instead of hanging (decision fuel-metering). Change it with `WARP_TIME_LIMIT=<seconds>`.
pub const DEFAULT_TIME_LIMIT: Duration = Duration::from_secs(60);
/// Environment variable that overrides `DEFAULT_TIME_LIMIT`, in seconds
pub const TIME_LIMIT_VARIABLE: &str = "WARP_TIME_LIMIT";

/// Fuel for a run that asks for fuel instead of a time limit (`use fuel`, `warp fuel <steps>`, `WARP_FUEL`): wasmtime
/// burns about one unit per executed WASM instruction, so this is roughly a hundred billion steps before a run stops
/// with `out of fuel after N steps`. Counting steps makes calls about four times slower (probes/fast_int/fuel_cost.sh),
/// so only such runs count them.
pub const DEFAULT_FUEL: u64 = 100_000_000_000;
/// Environment variable that asks for fuel, its value the steps (overrides `DEFAULT_FUEL`)
pub const FUEL_VARIABLE: &str = "WARP_FUEL";

/// The steps `WARP_FUEL` gives (underscores allowed: 1_000_000), if it is set to a number
pub fn fuel_from_environment() -> Option<u64> {
	std::env::var(FUEL_VARIABLE).ok().and_then(|steps| steps.trim().replace('_', "").parse().ok())
}

/// The time `WARP_TIME_LIMIT` gives (seconds, fractions allowed), if it is set to a positive number
pub fn time_limit_from_environment() -> Option<Duration> {
	let seconds: f64 = std::env::var(TIME_LIMIT_VARIABLE).ok()?.trim().parse().ok()?;
	(seconds > 0.0).then(|| Duration::from_secs_f64(seconds))
}
