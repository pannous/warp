//! card bodiless-imports (decision bodiless-imports): `is_null(x: any): bool from runtime` declares an intrinsic of the
//! self-hosted emitter without a body, so a file of core/runtime/ compiles alone; the Rust lane has no such
//! intrinsic and traps if one is called
use crate::common::fails_with;
use crate::is;

/// The status of a Rust panic
#[cfg(feature = "native")]
const PANIC_STATUS: i32 = 101;

#[test]
fn a_declared_runtime_word_compiles_where_it_is_not_called() {
	is!("is_null(x: any): bool from runtime\nis_blank(n: any): bool := is_null(n)\n7", 7);
	is!("byte_count(t: text): int from runtime\nsize_of(t: text): int := byte_count(t)\n8", 8);
}

#[test]
fn a_declared_runtime_word_traps_when_the_rust_lane_calls_it() {
	fails_with("is_null(x: any): bool from runtime\nis_null(3)", "unreachable");
}

/// The trap standing in for a declared word has no kind of its own: a caller reads the declared result type
#[test]
fn a_declared_runtime_word_gives_its_declared_type() {
	is!("new_list(room: int): list from runtime\nfirst_of(items: list): int := 1\ncopied(count: int): int := {\n\tout = new_list(count)\n\tfirst_of(out)\n}\n7", 7);
}

/// A value the emitter supplies, `failed_in: text from runtime`, is a global the Rust lane starts at its type's zero
#[test]
fn a_declared_runtime_value_is_a_global() {
	is!("g: text from runtime\nf() := { global g; g + \"!\" }\nf()", "!");
}

/// Each part of core/runtime/ names the intrinsics it calls (runtime/intrinsics.warp) and the parts whose words
/// it calls, so it compiles and runs alone in the Rust lane
#[cfg(feature = "native")]
#[test]
fn each_runtime_part_runs_alone() {
	let mut parts: Vec<_> =
		std::fs::read_dir("core/runtime").unwrap().map(|entry| entry.unwrap().path()).collect();
	parts.sort();
	let failed: Vec<String> = parts
		.iter()
		.filter_map(|part| {
			let output = crate::common::warp_command().arg("--no-ask").arg(part).output().expect("warp runs");
			let said = String::from_utf8_lossy(&output.stdout).to_string() + &String::from_utf8_lossy(&output.stderr);
			// a part's value is its exit status (P77): only a crash, a signal or a panic, fails it
			let crashed = matches!(output.status.code(), None | Some(PANIC_STATUS));
			(crashed || said.contains("Error")).then(|| format!("{}: {said}", part.display()))
		})
		.collect();
	assert!(failed.is_empty(), "{}", failed.join("\n"));
}
