// Overloads by arity and by type together (card use-sound-play): f(x), f(x, y) and f(b: Box) are two arities, the
// one-argument pair dispatching on the type; a module word with several arities gains a method the same way
use crate::is;

#[test]
fn a_type_overload_beside_several_arities_keeps_them_all() {
	is!("class Box{v}\nf(x) := 1\nf(x, y) := f(x) + 1\nf(b: Box) := 3\nf(1, 2) * 10 + f(Box(5))", 23);
	is!("class Box{v}\nf(x) := 1\nf(x, y) := 2\nf(b: Box) := 3\nf(1) + f(1, 2)", 3);
}

#[test]
fn a_method_added_to_a_module_word_of_several_arities_keeps_them_all() {
	is!("use sound\nclass Box{v}\nplay(b: Box) := 7\nplay(Box(1))", 7);
	// the library's play gives a handle, a number (0 in the browser's test page), never the method's text
	is!("use sound\nclass Box{v}\nplay(b: Box) := \"box\"\nplay(C4, 0.01) is int", true);
	is!("use sound\nplay(x, d, b: Bus) := \"bus\"\nplay(C4, 0.01) is int", true);
}
