// DIAGNOSTIC (card browser-gpu, dropped before merge): how far the browser's @gpu results are from the CPU's
use warp::wasm_emitter::eval;

#[test]
fn gpu_precision_probe() {
	let filled = "linear xs = float[40000]\nfor i in 1 to 40000 { xs#i = i / 40000.0 }\n";
	let deviation = |rest: &str| eval(&format!("{filled}{rest}")).serialize();
	let report = [
		("sin at 1", deviation("ys = xs.map(x => sin(x)) @gpu\nabs(ys#40000 - sin(1))")),
		("sin worst", deviation("ys = xs.map(x => sin(x)) @gpu\nm = 0.0\nfor i in 1 to 40000 { d = abs(ys#i - sin(xs#i))\nif d > m { m = d } }\nm")),
		("exp(sin)+1 at 1", deviation("ys = xs.map(x => sin(x)) @gpu\nzs = ys.map(y => exp(y) + 1) @gpu\nabs(zs#40000 - (exp(sin(1)) + 1))")),
		("x*2 then sin+y at 1", deviation("ys = xs.map(x => x * 2).map(y => sin(y) + y) @gpu\nabs(ys#40000 - (sin(2) + 2))")),
		("max of sin", deviation("abs(max(xs.map(x => sin(x)) @gpu) - sin(1))")),
		("sum of sin, relative", deviation("c = 0.0\nfor i in 1 to 40000 { c = c + sin(xs#i) }\nabs(sum(xs.map(x => sin(x)) @gpu) - c) / c")),
		("min of sin", deviation("abs(min(xs.map(x => sin(x)) @gpu) - sin(1 / 40000))")),
	];
	panic!("adapter {:?}\n{report:#?}", std::env::var("WARP_GPU_ADAPTER"));
}
