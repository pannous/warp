// card animal-dog: `x is Animal` holds for an instance of a class extending Animal, at any depth
use crate::is;

#[test]
fn an_instance_is_of_its_parent_classes() {
	let classes = "class Animal { name: text }\nclass Dog extends Animal { }\nclass Puppy extends Dog { }\n";
	is!(&format!("{classes}d = Dog(\"rex\"); d is Animal"), true);
	is!(&format!("{classes}p = Puppy(\"bit\"); p is Animal and p is Dog"), true);
	is!(&format!("{classes}a = Animal(\"any\"); a is Dog"), false);
}
