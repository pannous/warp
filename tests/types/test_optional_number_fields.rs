// An optional number field (card optional-int): `age: int?` left out of a construction is ø, given it is the number
use crate::is;
use warp::Node;

#[test]
fn an_omitted_optional_number_is_empty() {
	is!("type P: { name: string; age: int? }; p = P{name: \"Al\"}; p.name", "Al");
	is!("type P: { name: string; age: int? }; p = P{name: \"Al\"}; p.age", Node::Empty);
	is!("class P { name: string; score: float? }; p = P(name: \"Al\"); p.score", Node::Empty);
}

#[test]
fn a_given_optional_number_is_the_number() {
	is!("class P { name: string; age: int? }; p = P(name: \"Al\", age: 3); p.age + 1", 4);
	is!("type P: { name: string; age: int? }; p = P{name: \"Al\", age: 41}; p.age = p.age + 1; p.age", 42);
}
