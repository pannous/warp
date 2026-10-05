/// Fuel for one run: wasmtime burns about one unit per executed WASM instruction, so this is roughly
/// ten billion steps (several seconds of compute) before a run stops with `out of fuel after N steps`
/// instead of hanging. Raise it with the environment variable `WARP_FUEL=<steps>` or `warp --fuel <steps>`.
pub const DEFAULT_FUEL: u64 = 10_000_000_000;
/// Environment variable that overrides `DEFAULT_FUEL`
pub const FUEL_VARIABLE: &str = "WARP_FUEL";

/// The steps `WARP_FUEL` gives (underscores allowed: 1_000_000), if it is set to a number
pub fn fuel_from_environment() -> Option<u64> {
	std::env::var(FUEL_VARIABLE).ok().and_then(|steps| steps.trim().replace('_', "").parse().ok())
}
