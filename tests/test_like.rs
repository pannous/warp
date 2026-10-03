// `image like photo` (user decision, notes/open_decisions.md 2026-10-03, a central philosophy): a value of unknown type
// passed for a typed parameter is duck typed, judged by its uses; a value of a known different type is an error that
// teaches `like`; after `image like photo` an image is usable wherever a photo is expected, still judged by its uses
use crate::common::fails_with;
use warp::is;

const PHOTO: &str = "class photo{width:int height:int}; ";
const IMAGE: &str = "class image{width:int height:int}; ";
const THUMB: &str = "class thumb{width:int}; ";
const KEEP: &str = "keep(p:photo) := p.width; ";
const AREA: &str = "area(p:photo) := p.width * p.height; ";

fn program(parts: &[&str], rest: &str) -> String {
	format!("{}{rest}", parts.concat())
}

#[test]
fn an_unknown_value_is_duck_typed_by_its_uses() {
	is!(&program(&[PHOTO, KEEP], "f(x) := keep(x); f({width:3})"), 3);
	is!(&program(&[PHOTO, IMAGE, KEEP], "f(x) := keep(x); f(image{width:4 height:1})"), 4);
	fails_with(&program(&[PHOTO, AREA], "f(x) := area(x); f({width:3})"), "no field height");
}

#[test]
fn a_known_other_type_teaches_like() {
	fails_with(&program(&[PHOTO, IMAGE, KEEP], "keep(image{width:3 height:4})"), "declare `image like photo`");
	fails_with(&program(&[PHOTO, IMAGE, KEEP], "i = image{width:3 height:4}; keep(i)"), "image is not a photo");
	fails_with(&program(&[PHOTO, THUMB, KEEP], "keep(thumb{width:3})"), "declare `thumb like photo`");
}

#[test]
fn like_makes_a_type_usable_as_another() {
	is!(&program(&[PHOTO, IMAGE, KEEP], "image like photo; keep(image{width:3 height:4})"), 3);
	is!(&program(&[PHOTO, IMAGE, AREA], "image like photo; i = image{width:3 height:4}; area(i)"), 12);
	is!(&program(&[PHOTO, IMAGE, KEEP], "x = keep(image{width:5 height:4}); image like photo; x"), 5);
}

#[test]
fn a_like_type_is_judged_by_its_uses() {
	is!(&program(&[PHOTO, THUMB, KEEP], "thumb like photo; keep(thumb{width:3})"), 3);
	fails_with(&program(&[PHOTO, THUMB, AREA], "thumb like photo; area(thumb{width:3})"), "no field height");
}

#[test]
fn like_goes_one_way() {
	let show = "show(i:image) := i.width; ";
	fails_with(&program(&[PHOTO, IMAGE, show], "image like photo; show(photo{width:3 height:4})"), "declare `photo like image`");
}

#[test]
fn a_type_can_be_like_several_and_likeness_chains() {
	let page = "class page{width:int}; turn(q:page) := q.width + 1; ";
	is!(&program(&[PHOTO, IMAGE, KEEP, page], "image like photo; image like page; keep(image{width:3 height:1}) + turn(image{width:3 height:1})"), 7);
	is!(&program(&[PHOTO, IMAGE, THUMB, KEEP], "thumb like image; image like photo; keep(thumb{width:6})"), 6);
}

#[test]
fn like_relates_declared_types_only() {
	fails_with(&program(&[PHOTO], "banana like photo; 1"), "banana is not a declared type");
	fails_with(&program(&[PHOTO], "photo like banana; 1"), "banana is not a declared type");
}
