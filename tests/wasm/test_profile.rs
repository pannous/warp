// WARP_PROFILE=<file> writes where a run spent its time as Firefox Profiler JSON (card warp-profile)

#[test]
fn a_profiled_run_names_its_busy_function() {
	let folder = crate::common::scratch_directory("profile");
	std::fs::create_dir_all(&folder).unwrap();
	let profile = folder.join("busy.json");
	let _ = std::fs::remove_file(&profile);
	let output = crate::common::warp_command()
		.args(["eval", "busy(n) := { s = 0; for i in 1..n { s += i % 7 }; s }; busy(100000000)"])
		.env("WARP_PROFILE", &profile)
		.env("WARP_NO_WINDOW", "1")
		.output()
		.unwrap();
	assert!(String::from_utf8_lossy(&output.stdout).contains("299999995"), "{output:?}");
	let written = std::fs::read_to_string(&profile).unwrap_or_default();
	assert!(written.contains("\"busy\""), "{output:?}\n{}", &written[..written.len().min(2000)]);
}
