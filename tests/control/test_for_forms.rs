use crate::is;

#[test]
fn c_style_for_with_a_space_or_without_braces() {
	is!("s=0; for (i=0;i<3;i++) {s+=i}; s", 3);
	is!("s=0; for(i=0;i<3;i++) s+=i; s", 3);
	is!("s=0; for (i=0;i<3;i++): s+=i; s", 3);
}

#[test]
fn block_after_a_chained_iterable() {
	is!("s=0; for (1…5).filter(x=>x%2) {s+=it}; s", 9);
}

#[cfg(feature = "native")]
#[test]
fn print_with_commas_in_a_loop_body() {
	use crate::common::printed;
	assert_eq!(printed("for i in 1…2: print i, i*2"), "1 2\n2 4\n");
	assert_eq!(printed("for i in 1…2 { print i, i*2 }"), "1 2\n2 4\n");
	assert_eq!(printed("for(i=1;i<3;i++) print i, i*2"), "1 2\n2 4\n");
}

#[cfg(feature = "native")]
#[test]
fn print_with_commas_in_an_if_branch() {
	use crate::common::printed;
	assert_eq!(printed("if 1: print 1, 2"), "1 2\n");
	assert_eq!(printed("if 1: print 1, 2 else print 3"), "1 2\n");
	assert_eq!(printed("if 0: print 1 else print 3, 4"), "3 4\n");
}

#[cfg(feature = "native")]
#[test]
fn print_of_a_call_without_parentheses_in_an_if_branch() {
	use crate::common::printed;
	assert_eq!(printed("if 1: print ord \"a\""), "97\n");
	assert_eq!(printed("if 0: print 1 else print ord \"b\""), "98\n");
	assert_eq!(printed("if 0 then print 1 else print ord \"b\""), "98\n");
	assert_eq!(printed("if 1 then print ord \"a\", 2 else print 3"), "97 2\n");
}

#[test]
fn else_after_a_block_takes_the_whole_expression() {
	is!("if 1 {1} else 3+1", 1);
	is!("if 0 {1} else 3+1", 4);
	is!("x = if 0 {1} else 3+1; x", 4);
}

#[cfg(feature = "native")]
#[test]
fn else_print_after_a_block() {
	assert_eq!(crate::common::printed("if 0 {print 1} else print 3+1"), "4\n");
}
