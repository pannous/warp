//! Tested laws: property tests on generated values, edge values first, from a fixed seed
use super::*;

const RANDOM_SEED: u64 = 0x9E37_79B9_7F4A_7C15;
const RANDOM_INT_RANGE: i64 = 1000;
/// Keep machine-word boundaries in the generated corpus even though Warp Int promotes on overflow.
const EDGE_INTS: [i64; 9] = [0, 1, -1, 2, -2, i64::MAX, i64::MIN, 3_037_000_500, -3_037_000_500];
const EDGE_FLOATS: [f64; 5] = [0.0, 1.0, -1.0, 0.5, -2.5];
/// The wasm trap of a recursion deeper than the stack
const STACK_EXHAUSTED: &str = "call stack exhausted";

/// Deterministic xorshift so failures reproduce without a rand dependency.
struct Generator(u64);

impl Generator {
	fn next(&mut self) -> u64 {
		self.0 ^= self.0 << 13;
		self.0 ^= self.0 >> 7;
		self.0 ^= self.0 << 17;
		self.0
	}

	fn value(&mut self, kind: Kind, trial: usize) -> Node {
		let pick = |edges: usize| trial < edges;
		match kind {
			Kind::Float if pick(EDGE_FLOATS.len()) => Node::Number(Number::Float(EDGE_FLOATS[trial])),
			Kind::Float => {
				let unit = (self.next() >> 11) as f64 / (1u64 << 53) as f64;
				Node::Number(Number::Float((unit * 2.0 - 1.0) * RANDOM_INT_RANGE as f64))
			}
			_ if pick(EDGE_INTS.len()) => Node::Number(Number::Int(EDGE_INTS[trial])),
			_ => Node::Number(Number::Int((self.next() % (2 * RANDOM_INT_RANGE as u64 + 1)) as i64 - RANDOM_INT_RANGE)),
		}
	}
}

/// An instance recursing deeper than the wasm stack (`sum_to(i64::MAX)`) is skipped: no verdict, Lean decides
pub fn property_test(lawful: &Lawful, law: &Law, trials: usize, code: &str) -> Verdict {
	let mut generator = Generator(RANDOM_SEED);
	let (mut run, mut too_deep) = (0, 0);
	for trial in 0..trials {
		let bindings: HashMap<String, Node> =
			law.variables.iter().map(|(name, kind)| (name.clone(), generator.value(*kind, trial))).collect();
		run += 1;
		match check_instance(lawful, law, &bindings, code) {
			Verdict::Holds => {}
			Verdict::Unknown(why) if why.contains(STACK_EXHAUSTED) => too_deep += 1,
			other => return other,
		}
		if law.variables.is_empty() {
			break;
		}
	}
	match too_deep == run {
		true => Verdict::Unknown(format!("every instance recursed too deep to run ({STACK_EXHAUSTED})")),
		false => Verdict::Holds,
	}
}
