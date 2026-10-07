// samples/polymorphism.wasp (matching by type name, D5): a type word repeated alone numbers its parameters, the body
// reads the n-th as `float#n` or positionally as `$0`, `$1`
use crate::is;

#[test]
fn repeated_type_words_number_their_parameters() {
	is!("combine float with float = float#1 + float#2\ncombine(1.5, 2.0)", 3.5);
	is!("combine int with int = $0 * $1\ncombine(2, 3)", 6);
	is!("combine float with float = float#1 + float#2\ncombine int with int = $0 * $1\ncombine(1.5, 2.0) + combine(2, 3)", 9.5);
}
