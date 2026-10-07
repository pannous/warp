//! P184 (user, 2026-10-07): English number words are numbers (`one plus two` is 3) unless the program names a variable,
//! parameter or function so; each one hints its digits. Keys (`{one: 1}`) and members (`x.one`) stay words.

use crate::node::Node;
use crate::operators::Op;

const NUMBER_WORDS: [(&str, i64); 28] = [
	("zero", 0), ("one", 1), ("two", 2), ("three", 3), ("four", 4), ("five", 5), ("six", 6), ("seven", 7), ("eight", 8),
	("nine", 9), ("ten", 10), ("eleven", 11), ("twelve", 12), ("thirteen", 13), ("fourteen", 14), ("fifteen", 15),
	("sixteen", 16), ("seventeen", 17), ("eighteen", 18), ("nineteen", 19), ("twenty", 20), ("thirty", 30), ("forty", 40),
	("fifty", 50), ("sixty", 60), ("seventy", 70), ("eighty", 80), ("ninety", 90),
];

pub fn lower(program: Node) -> Node {
	let unnamed: Vec<(&str, i64)> = NUMBER_WORDS.into_iter().filter(|(word, _)| !crate::soft_keywords::program_names(&program, word)).collect();
	if unnamed.is_empty() {
		return program;
	}
	as_numbers(program, &unnamed)
}

fn as_numbers(node: Node, words: &[(&str, i64)]) -> Node {
	match node {
		Node::Symbol(word) => match words.iter().find(|(number_word, _)| *number_word == word) {
			Some((_, number)) => {
				crate::normalize::hint(&word, &number.to_string(), "a number is written in digits");
				Node::int(*number)
			}
			None => Node::Symbol(word),
		},
		Node::Key(key, op @ (Op::Colon | Op::Dot), value) => {
			let key = if op == Op::Dot { Box::new(as_numbers(*key, words)) } else { key };
			let value = if op == Op::Dot { value } else { Box::new(as_numbers(*value, words)) };
			Node::Key(key, op, value)
		}
		other => other.map_children(|child| as_numbers(child, words)),
	}
}
