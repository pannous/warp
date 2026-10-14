// P22 (user, 2026-10-05): `p:photo = pic{width:3}` with pic a known other type lacking a field of photo is an error that
// teaches `pic like photo`, never a got-it warning; after `pic like photo` it is judged by its uses
use crate::common::fails_with;
use crate::is;

const TYPES: &str = "class photo{width:int height:int}; class pic{width:int}; ";

#[test]
fn a_known_other_type_lacking_a_field_teaches_like() {
	fails_with(&format!("{TYPES}p:photo = pic{{width:3}}; p.width"), "declare `pic like photo`");
	fails_with(&format!("{TYPES}keep(p:photo) := p.width; keep(pic{{width:3}})"), "declare `pic like photo`");
	is!(&format!("{TYPES}pic like photo; p:photo = pic{{width:3}}; p.width"), 3);
	fails_with(&format!("{TYPES}pic like photo; p:photo = pic{{width:3}}; p.height"), "no field height");
}
