// Metadata tests
// Migrated from tests_*.rs files

use crate::{eq, skip};
use warp::exists;
use warp::warp_parser::parse;
use warp::Node;

#[test]
fn test_meta_at() {
	eq!(parse("tee{a:1}").name(), "tee");
	eq!(parse("tee{a:1}").serialize(), "tee{a:1}");
	let code = "@attrib tee{a:1}";
	let node = parse(code);
	assert!(node.name() == "tee");
	assert!(node.length() == 1);
	assert!(node["a"] == 1);
	exists!(node["@attrib"]);
}

#[test]
fn test_meta_at2() {
	let code = "@attrib(1) @attrib2(42) tee{a:1}";
	let node = parse(code);
	assert!(node.name() == "tee");
	assert!(node.length() == 1);
	assert!(node["a"] == 1);
	// eq!(node.serialize(),code); // todo ok except order!
	exists!(node["@attrib"]);
	exists!(node["@attrib2"]);
	eq!(node["@attrib"], 1);
	eq!(node["@attrib2"], 42);
}

#[test]
fn test_parent_context() {
	//     chars
	let source = "{a:'HIO' d:{} b:3 c:ø}";
	let result = parse(source);
	result.print();
	let a: Node = result["a"].clone();
	a.print();
	// eq!(a.kind(), strings);
	eq!(a.value(), "HIO"); // we can't be sure it's to string
	eq!(a.name(), "HIO"); // keyNodes go to values!
	assert!(a == "HIO");
	//	eq!(a.name(),"a" or"HIO");// keyNodes go to values!
	skip!(

		eq!(a.kind(), key);
		assert!(a.name == "HIO");
	);
}
