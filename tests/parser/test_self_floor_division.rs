//! Floor division under WARP_SELF_HOSTED (card crash-src): parser.warp reads `÷` and `÷=` as it reads `/!` and `/!=`,
//! the call floor_quotient, so a program prints what the Rust parser's compile prints (`-1÷6` panicked the emitter)
const CACHE_FOLDER: &str = "self_hosted_stage_cache_floor_division";
/// Each floor-division spelling, with a negative quotient that floors
const FLOOR_DIVISIONS: [&str; 5] =
	["print(-1÷6)", "print(-7 ÷ 2)", "x = -7\nx ÷= 2\nprint(x)", "print(-7 /! 2)", "print(-7 div 2)"];

fn printed(self_hosted: bool, cache: &std::path::Path, code: &str) -> String {
	let mut command = crate::common::warp_command();
	if self_hosted {
		command.env("WARP_SELF_HOSTED", "1");
	}
	let run = command.env("WARP_MODULE_CACHE", cache).arg(code).output().expect("warp runs");
	String::from_utf8_lossy(&run.stdout).into_owned()
}

#[test]
fn self_hosted_floor_division_prints_as_rust_prints_it() {
	let cache = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join(CACHE_FOLDER);
	for code in FLOOR_DIVISIONS {
		let rust = printed(false, &cache, code);
		assert!(rust.contains("-1") || rust.contains("-4"), "{code}: {rust}");
		assert_eq!(printed(true, &cache, code), rust, "{code}");
	}
}
