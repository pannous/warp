// card def-hello: `def hello {…}` defines a function without parameters, like `def hello() {…}` and `def hello: …`
use crate::is;

#[test]
fn a_def_name_and_block_is_a_function() {
	is!("def five { 5 }; five + 1", 6);
	is!("def five { 5 }\nfive() * 2", 10);
	is!("function seven { 7 }; seven", 7);
}

#[cfg(feature = "native")]
#[test]
fn a_def_name_and_block_runs_its_statements() {
	assert_eq!(crate::common::printed("def hello {print 1}; hello").trim(), "1");
}
