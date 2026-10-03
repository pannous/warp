// Size and timing of programs for notes/multi_value.md; copy to examples/multi_value_bench.rs and
// `cargo run --release --example multi_value_bench`: each program is compiled once, then run alternately
use std::time::Instant;
use warp::wasm_emitter::compile;
use warp::wasm_reader::read_bytes;

const PROGRAMS: [(&str, &str); 3] = [
	("big-int digit sum", "fac(n) := n<2 ? 1 : n*fac(n-1); f = fac(1000); s = 0; while f > 0 { s += f % 10; f = f//10 }; s"),
	("divmod as list", "dm(a, b) := [a//b, a%b]; s = 0; i = 0; while i < 200000 { p = dm(i, 7); s += p#1 + p#2; i++ }; s"),
	("divmod as tuple", "dm(a, b) := return a//b, a%b; s = 0; i = 0; while i < 200000 { q, r = dm(i, 7); s += q + r; i++ }; s"),
];
const ROUNDS: usize = 9;

fn main() {
	let modules: Vec<Vec<u8>> = PROGRAMS.iter().map(|(_, program)| compile(program).unwrap_or_else(|error| panic!("{error:?}")).bytes).collect();
	let mut times = vec![Vec::new(); modules.len()];
	for _ in 0..ROUNDS {
		for ((module, timings), (label, _)) in modules.iter().zip(&mut times).zip(PROGRAMS) {
			let start = Instant::now();
			let result = read_bytes(module).unwrap();
			timings.push(start.elapsed().as_secs_f64() * 1000.0);
			assert!(matches!(format!("{result:?}").as_str(), "10539" | "2857642852"), "{label}: {result:?}");
		}
	}
	for ((timings, module), (label, _)) in times.iter_mut().zip(&modules).zip(PROGRAMS) {
		timings.sort_by(f64::total_cmp);
		println!("{label}: {} bytes, min {:.0} ms, median {:.0} ms", module.len(), timings[0], timings[ROUNDS / 2]);
	}
}
