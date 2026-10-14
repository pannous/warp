//! card int-let: `x:int = agent "33/3"` asks for a value of the expected type: a call with an expected result type
//! (notes/dispatch.md) of a function f calls its companion f_as(…, "type") when one is defined, as lib/agent.warp's
//! agent_as. The answers themselves need a key: probes/agent_types.sh
use warp::pipeline::lower;

fn lowered(code: &str) -> String {
	lower(code).unwrap_or_else(|error| error).serialize()
}

#[test]
fn a_typed_declaration_asks_the_agent_for_its_type() {
	assert!(lowered("x:int = agent \"33/3\"\nx").contains("agent_as"), "{}", lowered("x:int = agent \"33/3\"\nx"));
	assert!(lowered("float y = agent \"pi\"\ny").contains("agent_as"));
	assert!(lowered("agent \"is 7 prime\" as bool").contains("agent_as"));
}

#[test]
fn a_companion_takes_the_expected_type_of_any_function() {
	let code = "guess(q:text) := 0\nguess_as(q:text, type:text) := count(type)\nn:int = guess \"x\"\nn";
	assert!(lowered(code).contains("guess_as"), "{}", lowered(code));
}

#[test]
fn without_an_expected_type_the_agent_answers_text() {
	assert!(!lowered("x = agent \"say hi\"\nx").contains("agent_as \"say hi\""));
}
