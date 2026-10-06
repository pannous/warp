// A program that only passes an instance to a function runs the function: the instance is no program value of its own
use crate::is;

#[test]
fn a_call_with_an_instance_argument_gives_the_call_result() {
	is!("class pt{nm:text}; f(p:pt) := p.nm; f(pt(\"rex\"))", "rex");
	is!("class dog{name:text}; bark(d:dog) := d.name + \" barks\"; bark(dog(\"rex\"))", "rex barks");
	is!("class dog{n:real}; dog(2).n", 2);
	is!("class dog{b:bool}; dog(1).b", 1);
}
