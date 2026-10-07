//! An element of a list held as Nodes (returned by a function) compares by its value decided at run time: a float
//! element orders as a float, a character as its code point (card nested-held-compare)

use crate::is;

#[test]
fn a_float_element_of_a_returned_list_orders_by_value() {
	is!("def f(){ [0.5 + random()*0, 2.5] }; f()#1 < 1.5", 1);
	is!("def f(){ [0.5 + random()*0] }; m = f(); m#1 < 1", 1);
	is!("def f(){ [0.5 + random()*0] }; m = f(); m#1 >= 0.5", 1);
	is!("def f(){ [0.5 + random()*0] }; m = f(); 1 > m#1", 1);
	is!("def f(){ [[0.5 + random()*0]] }; m = f(); m[0][0] < 1", 1);
	is!("def f(){ [[0.5 + random()*0]] }; m = f(); m#1#1 > 1", 0);
	is!("def f(){ [[random()]] }; m = f(); m[0][0] < 1", 1);
}

#[test]
fn a_float_element_of_a_returned_list_equals_its_value() {
	is!("def f(){ [[0.5 + random()*0]] }; m = f(); m#1#1 == 0.5", 1);
	is!("def f(){ [[0.5 + random()*0]] }; m = f(); m#1#1 != 0.5", 0);
}

#[test]
fn int_and_character_elements_keep_their_order() {
	is!("def f(){ [[3]] }; m = f(); m#1#1 < 4", 1);
	is!("def f(){ [[1,'a']] }; m = f(); m#1#2 > 50", 1);
	is!("x=[1,'a']; x#2 > 50", 1);
	is!("def f(){ [['a']] }; m = f(); m#1#1 < 'b'", 1);
}
