//! D3 (user, 2026-10-03): `[1 2 3]+4` asks append or add to each element, fallback Error;
//! `.+` is element-wise, `xs + [4]` concatenates (like `[x]*n`, topic list-times)
use crate::common::fails_with;
use warp::diagnostic::{with_asker, ScriptedAnswers};
use warp::node::ints;
use warp::*;

fn answer(meaning: &str) -> ScriptedAnswers {
	ScriptedAnswers(vec![("list-plus".to_string(), meaning.to_string())])
}

#[test]
fn a_list_literal_plus_a_number_asks() {
	fails_with("[1 2 3]+4", "append 4 or add it to each element? (too ambiguous to guess)");
	fails_with("[1 2 3]+4", "`[1 2 3] + [4]` for append");
	fails_with("[1 2 3]+4", "`[1 2 3] .+ 4` for add to each element");
	fails_with("4+[1 2 3]", "(too ambiguous to guess)");
}

#[test]
fn the_answer_compiles_the_chosen_reading() {
	with_asker(answer("append"), || is!("[1 2 3]+4", ints(vec![1, 2, 3, 4])));
	with_asker(answer("prepend"), || is!("4+[1 2 3]", ints(vec![4, 1, 2, 3])));
	with_asker(answer("add to each element"), || is!("[1 2 3]+4", ints(vec![5, 6, 7])));
	with_asker(answer("add to each element"), || is!("4+[1 2 3]", ints(vec![5, 6, 7])));
}

#[test]
fn the_explicit_forms_never_ask() {
	is!("[1 2 3] + [4]", ints(vec![1, 2, 3, 4]));
	is!("[1 2 3] .+ 4", ints(vec![5, 6, 7]));
	is!("[1 2 3].+4", ints(vec![5, 6, 7]));
	is!("xs=[1 2]; xs .- 1", ints(vec![0, 1]));
	is!("[1 2 3] .* 2", ints(vec![2, 4, 6]));
	is!("[2 4] ./ 2", ints(vec![1, 2]));
	is!("n=3; [1 2] .+ n*2", ints(vec![7, 8]));
}

#[test]
fn a_list_variable_plus_a_number_stays_a_type_error() {
	fails_with("xs=[1 2]; xs+3", "type error");
}
