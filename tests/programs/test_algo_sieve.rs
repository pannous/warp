use warp::is;

// 25 primes below 100 sum to 1060; 97 sits at index 24; 50 is absent (-1)
#[test]
fn test_sieve() { is!("samples/sieve.wasp", 1083); }

// scratch bisection: evaluates every probes/algo/sieve/*.wasp snippet and prints its result
#[test]
#[ignore = "probe"]
fn probe_sieve_snippets() {
    let mut paths: Vec<_> = std::fs::read_dir("probes/algo/sieve").unwrap().map(|e| e.unwrap().path()).filter(|p| p.extension().is_some_and(|x| x == "wasp")).collect();
    paths.sort();
    for path in paths {
        let file = path.to_str().unwrap().to_string();
        let result = std::panic::catch_unwind(|| warp::wasm_emitter::eval(&file));
        match result {
            Ok(node) => println!("PROBE {} => {:?}", file, node),
            Err(e) => println!("PROBE {} => PANIC {:?}", file, e.downcast_ref::<String>().cloned().or(e.downcast_ref::<&str>().map(|s| s.to_string()))),
        }
    }
}

#[test]
fn test_sieve_idiomatic() { is!("samples/sieve_idiomatic.wasp", 1083); }
