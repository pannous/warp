//! runtime.warp time_text (card time-text-warp): a time printed under WARP_SELF_HOSTED shows as the Rust runtime's
//! time_text shows it, whichever emitter takes the program
const CACHE_FOLDER: &str = "self_hosted_stage_cache_time_text";
/// The forms time_text writes in warp: a date, an instant with and without a fraction, a local time, a time of day
const TIMES: [&str; 5] = [
	"print(2026-10-10)",
	"print(2026-10-10T14:43:33.682Z)",
	"print(1969-12-31T23:59Z)",
	"print(2026-10-14T22:27:46)",
	"print(2026-10-14T22:27.time)",
];

fn printed(self_hosted: bool, cache: &std::path::Path, code: &str) -> String {
	let mut command = crate::common::warp_command();
	if self_hosted {
		command.env("WARP_SELF_HOSTED", "1");
	}
	let run = command.env("WARP_MODULE_CACHE", cache).arg(code).output().expect("warp runs");
	String::from_utf8_lossy(&run.stdout).into_owned()
}

#[test]
fn self_hosted_times_print_as_rust_prints_them() {
	let cache = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join(CACHE_FOLDER);
	for code in TIMES {
		assert_eq!(printed(true, &cache, code), printed(false, &cache, code), "{code}");
	}
}
