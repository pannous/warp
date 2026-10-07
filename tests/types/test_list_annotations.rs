//! card float-list: the postfix list annotation `xs:float list = […]` declares a list of floats like `xs:list of float`
//! and `xs:floats`; it assigned the list to a variable named `list` and left xs a symbol (f(1) gave 'x')
use crate::is;

#[test]
fn a_postfix_list_annotation_declares_the_list() {
	is!("xs:float list = [0.5 as float, 0.25 as float]; f(j) := xs#j; f(1)", 0.5);
	is!("xs:float list = [0.5, 0.25]; xs#2", 0.25);
	is!("xs:int list = [1, 2]; count(xs)", 2);
}
