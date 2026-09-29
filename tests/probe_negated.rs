use warp::wasp_parser::WaspParser;
#[test]
fn probe_negated() {
	let mut report = String::new();
	for code in ["double -3", "sin -π/2", "double(x):=x*2;double -3", "double(x):=x*2\ndouble -3", "double(x):=x*2;double(-3)"] {
		report += &format!("PROBE {code:?} => {:?}\n", WaspParser::parse(code));
	}
	panic!("{report}");
}
