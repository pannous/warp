// Card symbolism-object (wiki/symbolism.md's cat, wiki/charged.md section 4): a tagged object `cat{…}` keeps every
// `key = value` entry as a value, and an entry `kill{print '🕱'}` is an uncharged block that runs only at `cat.kill!`
use crate::common::printed;
use crate::is;
use warp::wasm_emitter::eval;

#[test]
fn test_tagged_object_keeps_value_entries() {
	is!("cat{x=1 y=2}; cat.x + cat.y", 3);
	is!("cat{hair:{r=1 g=0 b=0} age=3}; cat.hair.r + cat.hair.b + cat.age", 4);
	let shown = eval("cat{hair-color:{r=1 g=0 b=0} age=3}").serialize();
	assert!(shown.contains("r:1") && shown.contains("g:0") && shown.contains("b:0"), "{shown}");
}

#[test]
fn test_block_entry_runs_only_at_bang() {
	assert!(!printed("cat{kill{print 'X'} age=3}; cat.age").contains('X'));
	assert!(!printed("cat = {kill{print 'X'} age=3}; cat.age").contains('X'));
	assert_eq!(printed("cat{kill{print 'X'} age=3}; cat.kill!").matches('X').count(), 1);
	is!("cat{kill{print 'X'} age=3}; cat.age", 3);
	// statements before a final read of the tagged object still run
	assert!(printed("cat{kill{print 'X'} age=3}\ncat.kill!\ncat").lines().any(|line| line == "X"));
	assert!(printed("print 'Y'; cat{a:1}; cat").lines().any(|line| line == "Y"));
}
