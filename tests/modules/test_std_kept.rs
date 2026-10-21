// `kept(name) ?? keep(name, value)` of use os (card pure-warm-start): a value computed by one run is the next run's
fn printed_by(program: &std::path::Path, cache: &std::path::Path) -> String {
	let output =
		crate::common::warp_command().arg(program).env("WARP_MODULE_CACHE", cache).output().expect("warp runs");
	String::from_utf8_lossy(&output.stdout).trim().to_string()
}

#[test]
fn a_kept_value_is_the_next_runs() {
	let folder = crate::common::scratch_directory("kept_value");
	let cache = folder.join("cache");
	let _ = std::fs::remove_dir_all(&cache);
	std::fs::create_dir_all(&cache).unwrap();
	let keeping = folder.join("keeping.warp");
	std::fs::write(&keeping, "use os\nprint kept(\"answer\") ?? keep(\"answer\", [6 * 7, \"computed\"])\n").unwrap();
	let reading = folder.join("reading.warp");
	std::fs::write(&reading, "use os\nprint kept(\"answer\") ?? \"not kept\"\n").unwrap();
	assert_eq!(printed_by(&reading, &cache), "not kept");
	let computed = printed_by(&keeping, &cache);
	assert!(computed.contains("42") && computed.contains("computed"), "{computed}");
	assert_eq!(printed_by(&reading, &cache), computed);
}
