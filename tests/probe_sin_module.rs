use warp::wasm_emitter::eval;
#[test]
fn probe_sin_module() {
	let mut report = String::new();
	for code in ["use sin;sin π/2", "use sin;sin π", "use sin;sin -π/2", "use sine;sine π/2",
		"real sin(real x):={x*2};sin 3", "sin(x):=x*2;sin 3", "def sin(x){x*2};sin 3"] {
		report += &format!("PROBE {code} => {:?}\n", eval(code));
	}
	panic!("{report}");
}
