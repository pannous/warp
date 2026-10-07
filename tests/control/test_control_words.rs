use crate::is;

#[test]
fn until_loops_while_the_condition_is_false() {
	is!("i=0; until i>=3 {i++}; i", 3);
	is!("i=0; until i>=3: i++; i", 3);
}

#[test]
fn unless_runs_its_block_when_the_condition_is_false() {
	is!("unless 0 {5}", 5);
	is!("x=1; unless x==2 {x=7}; x", 7);
	is!("x=1; unless x==1 {x=7}; x", 1);
	is!("unless 1 {5} else {6}", 6);
}

#[test]
fn n_times_repeats_its_block() {
	is!("x=0; 3 times {x++}; x", 3);
	is!("x=0; 2 times {3 times {x++}}; x", 6);
	is!("n=4; x=0; n times {x+=2}; x", 8);
	is!("x=5; 0 times {x++}; x", 5);
}

#[test]
fn trailing_if_guards_the_statement() {
	is!("a = 2 if 1", 2);
	is!("a=1; a = 2 if 0; a", 1);
	is!("a=1; a = 2 if 1; a", 2);
	is!("a=1; a = 2 unless 1; a", 1);
	is!("a=1; a = 2 unless 0; a", 2);
}

#[test]
fn trailing_while_and_until_repeat_the_statement() {
	is!("i=0; i++ while i<3; i", 3);
	is!("i=0; i++ until i>=3; i", 3);
	is!("i=5; i++ while i<3; i", 5);
}

#[test]
fn bang_evaluates_a_block() {
	is!("a=6; {a*a}!", 36);
	is!("f:={1+2}; f!", 3);
}

// card return-type: `if a {x} if b {y}` on one line is two if statements, not the first guarded by a trailing `if b`
#[test]
fn an_if_with_a_block_after_a_statement_starts_a_statement() {
	let mixed = "f(k:any) := { if k == 1 { return 2.5 } if k == 2 { return 3 } \"sx\" }";
	is!(&format!("{mixed}; let p:float = f(1); p * 2"), 5.0);
	is!(&format!("{mixed}; f(2)"), 3);
	is!(&format!("{mixed}; f(3)"), "sx");
	is!("x=0; if 1 { x+=1 } if 1 { x+=10 }; x", 11);
	is!("x=0; if 0 { x+=1 } unless 0 { x+=10 }; x", 10);
	is!("a=1; a = 2 if 1 == 1; a", 2); // a trailing if without a block still guards
}
