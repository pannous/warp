//! Card json5-parse (user: "our parser just eats JSON5"): json.parse reads JSON5, single quotes, comments, trailing
//! commas and bare keys, as warp's own literals do; `json(t)` is json.parse(t) without a `use json`
use crate::is;

#[test]
fn json_of_a_text_parses_it() {
	is!("x = json(\"{'a':1}\"); x.a", 1);
	is!("x = json(\"[1, 2, 3]\"); count(x)", 3);
}

#[test]
fn json_parse_reads_json5() {
	is!("use json; x = json.parse(\"{'a': 1, // note\n b: [2, 3,], /* more */ }\"); x.b", warp::warp_parser::parse("[2 3]"));
	is!("use json; json.parse(\"{'quote': 'say \\\"hi\\\"'}\").quote", "say \"hi\"");
	is!("use json; json.parse(\"{'a': 'it\\\\'s'}\").a", "it's");
}

#[test]
fn json_of_warp_data_is_that_data() {
	is!("x = json{ 'a':1 }; x.a", 1);
}
