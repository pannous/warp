// Diagnosis for card start, one cloud round: whether `[]` keeps its mark through lowering in a function body, and
// what the variants give; fails on purpose so the report reaches the log. Delete once the cause is known
use warp::wasm_emitter::eval;

const PROGRAMS: [&str; 6] = [
	"t = []; t += \"a\"; t",
	"def g() { t = []; t += \"a\"; t }\ng()",
	"def g(ps: any) { t = []; t += \"a\"; t }\ng([1])",
	"def g(ps: any) { t = []; for p in ps { t += p }; t }\ng([\"a\"])",
	"def g(ps: list) { t = []; for p in ps { t += p }; t }\ng([\"a\"])",
	"def g(ps: any) { t = []; for p in ps { t += [p] }; t }\ng([\"a\"])",
];

#[test]
fn empty_start_diagnosis_report() {
	let mut report = String::from("\nEMPTY-START DIAGNOSIS\n");
	for program in PROGRAMS {
		let lowered = match warp::pipeline::lower(program) {
			Ok(node) | Err(node) => node.serialize(),
		};
		report += &format!("{program}\n  lowered: {lowered}\n  value: {}\n", eval(program).serialize().trim());
	}
	panic!("{report}");
}
