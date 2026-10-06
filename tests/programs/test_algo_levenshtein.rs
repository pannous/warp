use crate::is;

// natural scripting-style version: `let` inside blocks, string parameters indexed with [], push of indexed values, Kotlin-inclusive 0..n
#[test]
fn test_levenshtein() { is!("samples/levenshtein.wasp", 11); }

#[test]
fn test_levenshtein_idiomatic() { is!("samples/levenshtein_idiomatic.wasp", 11); }

// Bisection helper: evaluates every probes/algo/levenshtein/*.wasp and prints the result
#[test]
#[ignore = "probe"]
fn probe_levenshtein_snippets() {
	let mut paths: Vec<_> = std::fs::read_dir("probes/algo/levenshtein").unwrap().map(|entry| entry.unwrap().path()).collect();
	paths.sort();
	for path in paths.iter().filter(|path| path.extension().is_some_and(|extension| extension == "wasp")) {
		let file = path.to_str().unwrap().to_string();
		let source = std::fs::read_to_string(&file).unwrap();
		let shown = match std::panic::catch_unwind(|| warp::wasm_emitter::eval(&file)) {
			Ok(node) => format!("{:?}", node),
			Err(panic) => format!("PANIC {:?}", panic.downcast_ref::<String>().cloned().or(panic.downcast_ref::<&str>().map(|text| text.to_string()))),
		};
		println!("=== {}\n{}\n--> {}\n", file, source.trim(), shown);
	}
}
