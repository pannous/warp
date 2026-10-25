//! `now` through compiler.warp: lib/time.warp's rule, the instant WASI's clock_time_get reads, to the millisecond as
//! the Rust lane's (user 2026-10-25)
use super::test_warp_pure::lane;

/// Two readings of the clock are ordered, and `now` shows as an RFC 3339 instant, `2026-10-25T11:43:40.692Z`
#[test]
fn warp_self_reads_now() {
	let code = "a = now\nb = now\nprint b >= a\nprint \"at \" + now";
	let ran = lane("self").arg(code).output().expect("warp runs");
	let printed = String::from_utf8_lossy(&ran.stdout).to_string() + &String::from_utf8_lossy(&ran.stderr);
	assert!(printed.lines().any(|line| line == "yes"), "{printed}");
	let instant = printed.lines().find_map(|line| line.strip_prefix("at ")).unwrap_or_default();
	assert!(instant.starts_with("20") && instant.ends_with('Z') && instant.contains('T'), "{printed}");
}
