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
