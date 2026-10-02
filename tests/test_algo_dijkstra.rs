use warp::is;

#[test]
fn test_dijkstra() { is!("samples/dijkstra.wasp", "ACBDEF:13"); }

#[test]
fn test_dijkstra_idiomatic() { is!("samples/dijkstra_idiomatic.wasp", 13245613); }

// bisection helper: cargo test --test test_algo_dijkstra probe_snippets -- --ignored --nocapture
#[test]
#[ignore = "probe"]
fn probe_snippets() {
	let mut files: Vec<_> = std::fs::read_dir("probes/algo/dijkstra").unwrap().flatten().map(|entry| entry.path()).filter(|path| path.extension().map_or(false, |ext| ext == "wasp")).collect();
	files.sort();
	for file in files {
		let path = file.to_str().unwrap().to_string();
		let outcome = std::panic::catch_unwind(|| warp::wasm_emitter::eval(&path));
		match outcome {
			Ok(result) => println!("{path} => {result:?}"),
			Err(_) => println!("{path} => PANIC"),
		}
	}
}
