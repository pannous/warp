use warp::is;

#[test]
fn test_sorting() { is!("samples/sorting.wasp", "-3,0,1,2,5,5,6,7,8,9"); }

#[test]
#[ignore = "probe"]
fn probe_sorting_snippets() {
	let mut paths: Vec<_> = std::fs::read_dir("probes/algo/sorting").unwrap().map(|e| e.unwrap().path()).filter(|p| p.extension().is_some_and(|x| x == "wasp")).collect();
	paths.sort();
	for path in paths {
		let path = path.to_str().unwrap().to_string();
		let outcome = std::panic::catch_unwind(|| warp::wasm_emitter::eval(&path));
		match outcome {
			Ok(node) => println!("PROBE {path}: {node:?}"),
			Err(_) => println!("PROBE {path}: PANIC"),
		}
	}
}

#[test]
fn test_sorting_idiomatic() { is!("samples/sorting_idiomatic.wasp", "-3,0,1,2,5,5,6,7,8,9"); }
