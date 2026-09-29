use warp::wasp_parser::parse;

#[test]
fn assigning_an_at_key_annotates_any_node_in_insertion_order() {
	let mut tee = parse("tee{a:1}");
	tee["@attrib"] = 42.into();
	tee["@attrib2"] = 43.into();
	tee["a"]["@unit"] = "cm".into();
	assert_eq!(tee["@attrib"], 42);
	assert_eq!(tee["@attrib2"], 43);
	assert_eq!(tee["a"]["@unit"], "cm");
	assert_eq!(tee["a"], 1);
	assert_eq!(tee.attributes().iter().map(|(name, _)| *name).collect::<Vec<_>>(), ["attrib", "attrib2"]);
	tee["@attrib"] = 7.into();
	assert_eq!(tee["@attrib"], 7);
	assert_eq!(tee.attributes().len(), 2);
}

#[test]
fn attribute_names_may_be_kebab_case() {
	assert_eq!(parse("@deprecated-since(2) tee{a:1}")["@deprecated-since"], 2);
}
