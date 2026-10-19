use warp::warp_parser::parse;

#[test]
fn attribute_names_may_be_kebab_case() {
	assert_eq!(parse("@deprecated-since(2) tee{a:1}")["@deprecated-since"], 2);
}

#[test]
fn attributes_serialize_in_front_and_valueless_ones_stay_bare() {
	let code = "@version(2) @draft tee{a:1}";
	assert_eq!(parse(code).serialize(), code);
}
