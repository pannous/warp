// A text built by appending is a rope (text_append): used as a key it names the entry by all its pieces, not only the
// last one (the self-hosted json_parser sample read keys this way into ø and lost all but the last letter)
use crate::is;

#[test]
fn a_rope_key_names_the_whole_text() {
	is!("k = \"\"; k = k + \"n\"; k = k + \"m\"; o = ø; o[k] = 1; o.nm", 1);
	is!("k = \"\"; k = k + \"n\"; k = k + \"m\"; o = {}; o[k] = 1; o.nm", 1);
	is!("k = \"\"; k = k + \"a\"; m = {a: 2}; m[k]", 2);
}
