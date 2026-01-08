use wasp::*;
// test are their OWN crate!

macro_rules! s {
	($lit:literal) => {
		String::from($lit)
	};
}

#[test]
fn test_str_string() {
	let _ok: String = s!("hello");
	eq!("hello".to_string(), String::from("hello"));
	eq!(String::from("hello"), "hello");
	eq!(s!("hello"), "hello");
	eq!("hello".s(), "hello");
}

#[test]
fn test_string_substring() {
	// init_lib(); // TODO: implement or import init_lib
	let s = "hello 🌍";
	let sub = s.substring(3, 5);
	put!("substring ", sub);
	eq!(sub, "lo");
}

#[test]
fn test_string_substring_from() {
	// init_lib(); // TODO: implement or import init_lib
	let s = "hello 🌍";
	// let sub = s.from(3); // reserved for String.from("…") constructor
	// let sub = s.start(3); // ugly! just learn:
	let sub = &s[3..];
	put!("substring from 3", sub);
	eq!(sub, "lo 🌍");
}

#[test]
fn test_string_at() {
	// init_lib(); // TODO: implement or import init_lib
	let s = "hello 🌍";
	let sub = s.at(3);
	put!("substring from 3", sub);
	eq!(sub, 'l');
	// eq!(sub, "l");
}

#[test]
fn test_string_from() {
	// init_lib(); // TODO: implement or import init_lib
	let s = "hello 🌍";
	let sub = s.after("ell");
	eq!(sub, "o 🌍");
}

#[test]
fn test_string_set_at() {
	// init_lib(); // TODO: implement or import init_lib
	let s = "hello 🌍";
	let sub = s.set(1, 'a');
	eq!(sub, "hallo 🌍");
}

#[test]
fn test_first_char() {
	let s = "hello 🌍";
	let c = s.first_char();
	put!("first_char: ", c);
	eq!(c, 'h');
	eq!(s.at(1), 'e');
	eq!(s.char(1), 'e');
	eq!(s.last_char(), '🌍');
	// eq!(-1%3,2);
	eq!(s.at(-1), '🌍');
}

#[test]
fn test_reverse() {
	let s = "hello 🌍";
	let rev = s.reverse();
	put!("reverse ", &rev);
	eq!(rev, "🌍 olleh");
}

#[test]
fn test_interpolation() {
	let _world = "🌍";
	let s = format!("hello {_world}");
	eq!(s, "hello 🌍");
}

#[test]
fn test_map() {
	// custom .to_uppercase()
	let upper = "hello 🌍".map(|c| c.upper());
	put!("upper ", &upper);
	eq!(upper, "HELLO 🌍");
}

#[test]
fn test_primitive_float() {
	eq!(4, 4);
	is!("3.0", 3.0);
	// is!("'3.0'", 3.0);
}

#[test]
fn test_primitive_int() {
	is!("3", 3);
	// is!("'3'", 3); php style, really?
	// is!("\"3\"", 3);
}

#[test]
fn test_primitive_char() {
	is!("'🍏'", '🍏');
}

#[test]
fn test_primitive_string() {
	is!("\"🍏\"", '🍏'); // !
}
#[test]
fn test_primitive_hello() {
	is!("hello", "hello"); // goes through eval! may serialize and deserialize wasm ;)
}

// #[test]
// pub(crate) fn test_all(){
// JUST TEST ALL IN FILE
//     test_reverse();
//     test_map();
// }

#[test]
fn test_string_concatenation() {
	//	eq!(Node("✔️"), True);
	//	eq!(Node("✔"), True);
	//	eq!(Node("✖️"), False);
	//	eq!(Node("✖"), False);
	// let huh = "a".s() + 2; // TODO: implement string operator overloads
	//     assert!(_eq!(huh.length(), 2);
	//     assert!(_eq!(huh[0], 'a');
	//     assert!(_eq!(huh[1], '2');
	//     assert!(_eq!(huh[2],  0);
	is!("a2", "a2");

	// TODO: implement string operator overloads
	// eq!(huh, "a2");
	// eq!("a" + 2, "a2");
	// eq!("a" + 2.2, "a2.2");
	// eq!("a" + "2.2", "a2.2");
	// eq!("a" + 'b', "ab");
	// eq!("a" + "bc", "abc");
	// eq!("a" + true, "a✔️");
	// eq!("a%sb" % "hi", "ahib");

	// eq!("a%db" % 123, "a123b");
	// eq!("a%s%db" % "hi" % 123, "ahi123b");
}

// From test_strings.rs
#[test]
fn test_string_basics() {
	is!("'hello'", "hello");
}

#[test]
#[ignore]
fn test_string_operations() {
	is!("'say ' + 0.", "say 0.");
	is!("'hello'", "hello");
	is!("`${1+1}`", 2);
}
