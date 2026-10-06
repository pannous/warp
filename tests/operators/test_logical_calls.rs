//! `and` / `or` test a call's value at run time, and evaluate it once
use crate::is;

#[test] // samples/game_of_life.wasp: `cell(x, y) and n == 2` was always `n == 2`
fn test_call_operand_of_and_or() {
	is!("def c(){ return 0 }; c() and 1", 0);
	is!("def c(x){ return false }; k=2; c(5) and k == 2", false);
	is!("def c(x){ return 3 }; c(5) and 7", 7);
	is!("def c(x){ return 0 }; c(5) or 4", 4);
	is!("def c(x){ return 3 }; c(5) or 4", 3);
	is!("def c(){ return 0.0 }; c() or 2.5", 2.5);
	is!("def c(){ return 1.5 }; c() and \"yes\"", "yes");
	is!("def c(){ return 0 }; c() or \"no\"", "no");
}

#[test]
fn test_call_operand_is_evaluated_once() {
	is!("global calls = 0; def c(){ calls = calls + 1; return 3 }; c() or 4; calls", 1);
	is!("global calls = 0; def c(){ calls = calls + 1; return 3 }; c() or \"x\"; calls", 1);
}

#[test] // an element of a list of mixed values is a Node: its truthiness is tested at run time
fn test_node_operand_of_and_or() {
	is!("xs=[false, \"a\"]; xs#1 and 5", false);
	is!("xs=[false, \"a\"]; xs#2 and 5", 5);
	is!("xs=[0, \"a\"]; xs#1 or \"b\"", "b");
	is!("x=[]; x or 3", 3);
	is!("global g=[]; g.push(false); def c(){ return g#1 }; k=2; c() and k == 2", false);
}
