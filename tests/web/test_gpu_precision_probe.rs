// DIAGNOSTIC (card browser-gpu, dropped before merge): the error of an @gpu sum of 40000 cosines
use warp::wasm_emitter::eval;

#[test]
fn gpu_precision_probe() {
	let program = "linear xs = float[40000]\nfor i in 1 to 40000 { xs#i = i / 40000.0 }\nys = xs.map(x => sin(x)) @gpu\nprint \"\\(ys#1) of \\(#ys)\"\nzs = ys.map(y => exp(y) + 1) @gpu\ns = sum(ys.map(y => cos(y)) @gpu)\n";
	let deviation = |rest: &str| eval(&format!("{program}{rest}")).serialize();
	let report = [
		("zs at 1", deviation("abs(zs#40000 - (exp(sin(1)) + 1))")),
		("sum of cos", deviation("abs(s - sum(ys.map(y => cos(y))))")),
		("sum of cos itself", deviation("s")),
	];
	panic!("adapter {:?}\n{report:#?}", std::env::var("WARP_GPU_ADAPTER"));
}
