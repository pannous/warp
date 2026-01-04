// Metadata tests
// Migrated from tests_*.rs files

use wasp::Node;
use wasp::wasp_parser::parse;
use wasp::{eq, exists, skip};

#[test]
#[ignore]
fn test_meta_field() {
	let mut tee = parse("tee{a:1}");
	tee["a"]["@attrib"] = 42.into();
	tee["a"]["@attrib2"] = 43.into();
	// tee["a"].setMeta("attrib2",(Node) 43);
	// tee["a"].metas()["attrib2"]=(Node) 43;
	eq!(tee.name(), "tee");
	exists!(tee["a"]["@attrib"]);
	exists!(tee["a"]["@attrib2"]);
	exists!(tee["a"] == 1);
	assert!(tee.length() == 1);
	assert!(tee["a"]["@attrib"] == 42);
	assert!(tee["a"]["@attrib2"] == 43);
	eq!(tee.serialize(), "tee{@attrib(42) @attrib2(43) a:1}");
}

#[test]
#[ignore]
fn test_meta() {
	let mut tee = parse("tee{a:1}");
	tee["@attrib"] = 42.into();
	tee["@attrib2"] = 43.into();
	eq!(tee.name(), "tee");
	eq!(tee.serialize(), "@attrib(42) @attrib2(43) tee{a:1}");
	exists!(tee["@attrib"]);
	exists!(tee["@attrib2"]);
	assert!(tee["a"] == 1);
	assert!(tee.length() == 1);
	assert!(tee["@attrib"] == 42);
	assert!(tee["@attrib2"] == 43);
}

#[test]
#[ignore]
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
#[ignore]
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
#[ignore]
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

#[test]
fn test_parent() {
	skip!( // not in rust!
		//	chars source = "{a:'HIO' d:{} b:3 c:ø}";
	//     chars
		source = "{a:'HIO'}";
	let result = parse(source);
		let a : Node = result["a"];
		// print(a);
		assert!(a.kind == key or a.kind == strings);
		assert!(a == "HIO");
		assert!(a.parent == 0); // key is the highest level
	//     Node * parent = a.value.node -> parent;
		assert!(parent);
		// print(parent); // BROKEN, WHY?? let's find out:
		assert!(*parent == result);
		skip!(

			// pointer identity broken by flat() ?
			assert!(parent == &result);
		);
		testParentContext(); // make sure parsed correctly
	);
}
