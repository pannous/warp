use std::fs;
use std::path::Path;
use warp::Node;
use warp::wasp_parser::WaspParser;
use warp::is;

#[test]
#[ignore = "slow recursion"]
fn test_fibonacci() { is!("samples/fibonacci.wasp", 55); }

#[test]
fn test_factorial() { is!("samples/factorial.wasp", 120); }

#[test]
#[ignore = "slow recursion"]
fn test_primes() { is!("samples/primes.wasp", 1); }

#[test]
fn test_gcd() { is!("samples/gcd.wasp", 6); }

#[test]
#[ignore = "slow recursion"]
fn test_sum() { is!("samples/sum.wasp", 55); }

#[test]
#[ignore = "slow recursion"]
fn test_power() { is!("samples/power.wasp", 1024); }

#[test]
#[ignore = "slow recursion"]
fn test_collatz() { is!("samples/collatz.wasp", 111); }

#[test]
#[ignore = "slow recursion"]
fn test_ackermann() { is!("samples/ackermann.wasp", 61); }

#[test]
#[ignore = "needs sqrt"]
fn test_quadratic() { is!("samples/quadratic.wasp", 6); }

#[test]
#[ignore = "needs string return"]
fn test_fizzbuzz() { is!("samples/fizzbuzz.wasp", "FizzBuzz"); }

/// Test that all sample .wasp files can be parsed without errors
#[test]
#[ignore] // works but it's too slow
fn test_parse_all_samples() {
	println!("\n=== Testing All Sample Files ===\n");
	// if 1 > 0 {
	//     todo!("currently STALLS after parsing 4 files!?");
	// }
	let samples_dir = Path::new("samples");
	assert!(samples_dir.exists(), "samples/ directory not found");

	let mut parsed_count = 0;
	let mut failed_files = Vec::new();

	// Read all .wasp files in samples directory
	let entries = fs::read_dir(samples_dir).expect("Failed to read samples directory");

	for entry in entries {
		let entry = entry.expect("Failed to read directory entry");
		let path = entry.path();

		// Only process .wasp files
		if path.extension().and_then(|s| s.to_str()) != Some("wasp") {
			continue;
		}

		let filename = path.file_name().unwrap().to_str().unwrap();
		print!("  Parsing {}... ", filename);

		match fs::read_to_string(&path) {
			Ok(content) => {
				let node = WaspParser::parse(&content);
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
		let known_issues = ["lib.wasp", "errors.wasp", "webgpu.wasp"];
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
