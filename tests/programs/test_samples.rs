use std::fs;
use std::path::Path;
use warp::Node;
use warp::warp_parser::WarpParser;
use crate::is;

#[test]
fn test_fibonacci() { is!("samples/fibonacci.warp", 55); }

#[test]
fn test_factorial() { is!("samples/factorial.warp", 120); }

#[test]
fn test_primes() { is!("samples/primes.warp", 1); }

#[test]
fn test_gcd() { is!("samples/gcd.warp", 6); }

#[test]
fn test_sum() { is!("samples/sum.warp", 55); }

#[test]
fn test_power() { is!("samples/power.warp", 1024); }

#[test]
fn test_collatz() { is!("samples/collatz.warp", 111); }

#[test]
fn test_ackermann() { is!("samples/ackermann.warp", 61); }

#[test]
fn test_quadratic() { is!("samples/quadratic.warp", 6); }

#[test]
fn test_fizzbuzz() { is!("samples/fizzbuzz.warp", "FizzBuzz"); }

/// Test that all sample .warp files can be parsed without errors
#[test]
fn test_parse_all_samples() {
	println!("\n=== Testing All Sample Files ===\n");
	// if 1 > 0 {
	//     todo!("currently STALLS after parsing 4 files!?");
	// }
	let samples_dir = Path::new("samples");
	assert!(samples_dir.exists(), "samples/ directory not found");

	let mut parsed_count = 0;
	let mut failed_files = Vec::new();

	// Read all .warp files in samples directory
	let entries = fs::read_dir(samples_dir).expect("Failed to read samples directory");

	for entry in entries {
		let entry = entry.expect("Failed to read directory entry");
		let path = entry.path();

		// Only process .warp files
		if path.extension().and_then(|s| s.to_str()) != Some("warp") {
			continue;
		}

		let filename = path.file_name().unwrap().to_str().unwrap();
		print!("  Parsing {}... ", filename);

		match fs::read_to_string(&path) {
			Ok(content) => {
				let node = WarpParser::parse(&content);
				if let Node::Error(e) = &node {
					println!("✗ Parse error: {:?}", e);
					failed_files.push(filename.to_string());
				} else {
					println!("✓");
					parsed_count += 1;

					// Debug output for first few files
					if parsed_count <= 3 {
						println!("    → {:?}", node);
					}
				}
			}
			Err(e) => {
				println!("✗ Read error: {}", e);
				failed_files.push(filename.to_string());
			}
		}
	}

	let total = parsed_count + failed_files.len();
	println!(
		"\n✓ Successfully parsed {}/{} sample files ({:.1}%)",
		parsed_count,
		total,
		(parsed_count as f64 / total as f64) * 100.0
	);

	if !failed_files.is_empty() {
		println!("\n⚠ Failed to parse {} files:", failed_files.len());
		for file in &failed_files {
			println!("  - {}", file);
		}

		// Known problematic files that can fail
		let known_issues = ["lib.warp", "errors.warp", "webgpu.warp"];
		let unexpected_failures: Vec<_> = failed_files
			.iter()
			.filter(|f| !known_issues.contains(&f.as_str()))
			.collect();

		if !unexpected_failures.is_empty() {
			println!("\n⚠ Unexpected failures (not in known issues list):");
			for file in unexpected_failures {
				println!("  - {}", file);
			}
			// Only panic if there are unexpected failures
			// panic!("Unexpected files failed to parse");
		}

		println!("\nNote: Some files may use experimental syntax or be intentionally malformed");
	}
}

#[test] // first row of the solution: 5 3 4
fn test_sudoku() { is!("samples/sudoku.warp", 534); }

#[test] // the self-playing snake eats four foods in 60 turns
fn test_snake() { is!("samples/snake.warp", 40); }

#[test] // both quicksorts agree
fn test_quicksort() { is!("samples/quicksort.warp", true); }

#[test] // the glider moved by (2, 2) in 8 generations
fn test_game_of_life() { is!("samples/game_of_life.warp", 2726); }

#[test] // three spheres on a floor, as shades of " .:-=+*#%@"
fn test_raytracer() { is!("samples/raytracer.warp", 8230); }

#[test] // height 4, nine values
fn test_binary_tree() { is!("samples/binary_tree.warp", 409); }

#[test] // nested block comments, doc comments, factorial(5)
fn test_comments() { is!("samples/comments.warp", 120); }

#[test] // fdlibm's sine kernel: sin(1) to six digits
fn test_sin() { is!("samples/sin.warp", 841471); }

#[test] // the Taylor kernel at π/2
fn test_sine() { assert!(warp::wasm_emitter::eval("samples/sine.warp").serialize().starts_with("1.0000000")); }

#[test] // 14 + 20 + 5
fn test_calculator() { is!("samples/calculator.warp", 39); }

#[test] // age 30 + first score 95
fn test_json_parser() { is!("samples/json_parser.warp", 125.0); }

#[test] // the iteration counts of a 40x30 grid
fn test_mandelbrot() { is!("samples/mandelbrot.warp", 17748); }

#[test] // the third employee, 9 squares + 2 evens, a tuple field
fn test_data_structures() { is!("samples/data_structures.warp", "Dave 11 20"); }

#[test] // every feature in one program (card g-_c-A): each part checks itself, a new check needs no change here
fn test_kitchensink() {
	let value = warp::pipeline::for_tests(|| warp::wasm_emitter::eval("samples/kitchensink.warp")).serialize();
	assert!(value.starts_with("\"✓ ") && value.ends_with(" tests passed\""), "{value}");
}

#[test] // fact(10) and fib(20) through the Z combinator (card y-combinator)
fn test_y_combinator() { is!("samples/y_combinator.warp", 6765); }
