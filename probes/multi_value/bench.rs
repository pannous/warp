// Size and timing of a big-int heavy program (notes/multi_value.md)
// `multi_value_bench <label>` writes the compiled program to probes/<label>.wasm;
// `multi_value_bench a.wasm b.wasm …` runs the files alternately and reports per-file timings
use std::time::Instant;
use warp::wasm_emitter::compile;
use warp::wasm_reader::read_bytes;

const PROGRAM: &str = "fac(n) := n<2 ? 1 : n*fac(n-1); f = fac(1000); s = 0; while f > 0 { s += f % 10; f = f//10 }; s";
const ROUNDS: usize = 9;

fn main() {
	let arguments: Vec<String> = std::env::args().skip(1).collect();
	if let [label] = arguments.as_slice() {
		if !label.ends_with(".wasm") {
			let module = compile(PROGRAM).unwrap_or_else(|error| panic!("{error:?}"));
			std::fs::write(format!("probes/{label}.wasm"), &module.bytes).unwrap();
			println!("{label}: {} bytes", module.bytes.len());
			return;
		}
	}
	let modules: Vec<Vec<u8>> = arguments.iter().map(|path| std::fs::read(path).unwrap()).collect();
	let mut times = vec![Vec::new(); modules.len()];
	for _ in 0..ROUNDS {
		for (module, timings) in modules.iter().zip(&mut times) {
			let start = Instant::now();
			let result = read_bytes(module).unwrap();
			timings.push(start.elapsed().as_secs_f64() * 1000.0);
			assert_eq!(format!("{result:?}"), "10539");
		}
	}
	for (path, timings) in arguments.iter().zip(&mut times) {
		timings.sort_by(f64::total_cmp);
		println!("{path}: min {:.0} ms, median {:.0} ms", timings[0], timings[ROUNDS / 2]);
	}
}
