//! card bodiless-imports (decision bodiless-imports): `is_null(x: any): bool from runtime` declares an intrinsic of the
//! self-hosted emitter without a body, so a file of lib/compiler/runtime/ compiles alone; the Rust lane has no such
//! intrinsic and traps if one is called
use crate::common::fails_with;
use crate::is;

#[test]
fn a_declared_runtime_word_compiles_where_it_is_not_called() {
	is!("is_null(x: any): bool from runtime\nis_blank(n: any): bool := is_null(n)\n7", 7);
	is!("byte_count(t: text): int from runtime\nsize_of(t: text): int := byte_count(t)\n8", 8);
}

#[test]
fn a_declared_runtime_word_traps_when_the_rust_lane_calls_it() {
	fails_with("is_null(x: any): bool from runtime\nis_null(3)", "unreachable");
}

/// Each part of lib/compiler/runtime/ names the intrinsics it calls (runtime/intrinsics.warp) and the parts whose words
/// it calls, so it compiles and runs alone in the Rust lane
#[cfg(feature = "native")]
#[test]
fn each_runtime_part_runs_alone() {
	let mut parts: Vec<_> = std::fs::read_dir("lib/compiler/runtime").unwrap().map(|entry| entry.unwrap().path()).collect();
	parts.sort();
	let failed: Vec<String> = parts
		.iter()
		.filter_map(|part| {
			let output = crate::common::warp_command().arg("--no-ask").arg(part).output().expect("warp runs");
			let said = String::from_utf8_lossy(&output.stdout).to_string() + &String::from_utf8_lossy(&output.stderr);
			(!output.status.success() || said.contains("Error")).then(|| format!("{}: {said}", part.display()))
		})
		.collect();
	assert!(failed.is_empty(), "{}", failed.join("\n"));
}
