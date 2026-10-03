//! An element of a list of decimal literals is the exact number it is (0.5 is 1/2), usable in arithmetic

use warp::is;

#[test]
fn a_decimal_element_takes_part_in_arithmetic() {
	is!("xs=[1.5 2.5]; xs#1+1", 2.5);
	is!("xs=[1.5 2.5]; 1+xs#2", 3.5);
	is!("xs=[0.5]; xs#1*4", 2);
}

#[test]
fn a_decimal_element_can_be_assigned() {
	is!("xs=[1.5 2.5]; y=xs#1; y", 1.5);
	is!("xs=[1.25 2]; y=xs#2; y+y", 4);
}

#[test]
fn a_decimal_element_compares() {
	is!("xs=[1.5 2.5]; xs#1 < xs#2", true);
}

#[test]
fn the_extremum_of_a_decimal_list() {
	is!("xs=[1.5 2.5]; max(xs)", 2.5);
	is!("xs=[1.5 2.5]; min(xs)+1", 2.5);
	is!("max([1.5 2.5])", 2.5);
}

#[test]
fn a_loop_walks_decimal_elements() {
	is!("for i in [1.5 2] {i}", 2);
	is!("n=0; for i in [1.5 2.5] {n=n+i}; n", 4);
}
