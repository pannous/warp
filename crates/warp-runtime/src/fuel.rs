/// Fuel for one run: wasmtime burns about one unit per executed WASM instruction, so this is roughly
/// a hundred billion steps before a run stops with `out of fuel after N steps` instead of hanging: a loop of 10^7
/// steps of ~10^4 instructions each finishes (card fuel-default), `while 1 {}` stops after a few seconds. Raise it with the environment variable `WARP_FUEL=<steps>` or `warp --fuel <steps>`.
pub const DEFAULT_FUEL: u64 = 100_000_000_000;
/// Environment variable that overrides `DEFAULT_FUEL`
pub const FUEL_VARIABLE: &str = "WARP_FUEL";

/// The steps `WARP_FUEL` gives (underscores allowed: 1_000_000), if it is set to a number
pub fn fuel_from_environment() -> Option<u64> {
	std::env::var(FUEL_VARIABLE).ok().and_then(|steps| steps.trim().replace('_', "").parse().ok())
}
