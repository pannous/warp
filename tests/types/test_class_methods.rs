// Methods written inside a class body (issue #14, card g-1nug, notes/classes.md): a definition in the body is a function
// whose first parameter is the receiver; bare field names and `self`/`this` read it; the class keeps only its fields
use crate::is;

#[test]
fn a_method_in_the_class_body_reads_the_fields() {
	is!("class person{name age:int; greet() := \"hi \" + name}; p = person(\"Ann\" 40); p.greet()", "hi Ann");
	is!("class person{name age:int; older(years) := age + years}; person(\"Ann\" 40).older(2)", 42);
	is!("class person{name; greet() := \"hi \" + self.name}; greet(person(\"Bo\"))", "hi Bo");
	is!("class person{name; shout() := this.name + \"!\"}; person(\"Bo\").shout()", "Bo!");
}

#[test]
fn a_method_without_parentheses_is_read_like_a_field() {
	is!("class square{side:int; area := side*side}; square(3).area", 9);
	is!("class square{side:int; area := side*side}; s = square(4); s.area + s.side", 20);
}

#[test]
fn methods_are_no_fields() {
	is!("class square{side:int; area := side*side; perimeter() := 4*side}; square(2).perimeter() + square(2).area", 12);
}

#[test]
fn classes_with_a_method_of_the_same_name_each_call_their_own() {
	is!("class square{side:int; area := side*side}; class circle{r:int; area := 3*r*r}; square(2).area + circle(1).area", 7);
	is!("class square{side:int; area := side*side}; class circle{r:int; area := 3*r*r}; s = square(3); c = circle(2); s.area() + c.area", 21);
	is!("class square{side:int; scaled(k) := square(side*k)}; class circle{r:int; scaled(k) := circle(r*k)}; square(2).scaled(3).side + circle(1).scaled(5).r", 11);
}

#[test]
fn a_method_in_the_body_conforms_to_a_trait() {
	is!("class dot{x:int; compare(other:dot) := x - other.x}; (sort [dot(2) dot(1)])#1.x", 1);
}

#[test]
fn a_method_assigning_its_receiver_says_so() {
	crate::common::fails_with("class counter{n:int; inc() := n += 1}; c = counter(0); c.inc(); c.n", "inc changes a field of counter");
}
