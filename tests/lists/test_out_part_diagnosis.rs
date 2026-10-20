// Diagnosis for card out-part, one cloud round: prints what each variant of `out += part` does and fails on purpose
// so the report lands in the cloud log. Not an outcome test: delete it once the card's cause is known.
use std::panic::{catch_unwind, AssertUnwindSafe};
use warp::util::with_fuel;
use warp::wasm_emitter::eval;

const PARAMETER_KINDS: [&str; 2] = ["any", "list"];
const STARTS: [&str; 3] = ["[]", "[0]", "list = []"];
const GROWTHS: [&str; 2] = ["out += p", "out = out + p"];
const FUEL_LADDER: [u64; 5] = [1_000_000, 3_000_000, 10_000_000, 30_000_000, 100_000_000];
const PARTS: &str = "parts = (1 to 3000).map(i => [i, i])\n";

fn printed(code: &str) -> String {
	match catch_unwind(AssertUnwindSafe(|| eval(code).serialize())) {
		Ok(text) => text.trim().replace('\n', " "),
		Err(_) => "panicked".into(),
	}
}

fn smallest_budget(code: &str) -> String {
	for budget in FUEL_LADDER {
		if with_fuel(budget, || printed(code)) == "6000" {
			return format!("{budget}");
		}
	}
	format!("> {}", FUEL_LADDER[FUEL_LADDER.len() - 1])
}

fn variant_report(parameter: &str, start: &str, growth: &str) -> String {
	let declared = if start.starts_with("list") { format!("out: {start}") } else { format!("out = {start}") };
	let observed = format!(
		"def j(ps: {parameter}): list {{ {declared}; before = type(out); b = out; kinds = []; \
		 for p in ps {{ kinds += [type(p)]; {growth} }}; [b, out, before, type(out), kinds] }}\nj([[1], [2]])"
	);
	let timed = format!(
		"def j(ps: {parameter}): list {{ {declared}; for p in ps {{ {growth} }}; out }}\n{PARTS}count(j(parts))"
	);
	format!("ps: {parameter} | {declared} | {growth}\n  b, out, type before, type after, item types: {}\n  smallest budget for 3000 parts: {}\n",
		printed(&observed), smallest_budget(&timed))
}

#[test]
fn out_part_diagnosis_report() {
	let mut report = String::from("\nOUT-PART DIAGNOSIS\n");
	for parameter in PARAMETER_KINDS {
		for start in STARTS {
			for growth in GROWTHS {
				report += &variant_report(parameter, start, growth);
			}
		}
	}
	panic!("{report}");
}
