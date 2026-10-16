//! card warp-format: `warp format` (= `warp lint`) writes the one canonical layout (tabs, `name: value`, spaced
//! operators, one space after a comma, at most one blank line) and only where the parsed tree stays the same
use warp::formatting::{format, soft_hints, Options};

fn formatted(code: &str) -> String {
	format(code, Options::default()).expect("it parses").text
}

#[test]
fn test_canonical_layout() {
	let messy = "m = {a:1,b:2}   // note\nprint( m )\n\n\nprint 1+2  \nf(x) := {\n    x*2\n}\n";
	let canonical = "m = {a: 1, b: 2}   // note\nprint(m)\n\nprint 1 + 2\nf(x) := {\n\tx * 2\n}\n";
	assert_eq!(formatted(messy), canonical);
	assert_eq!(formatted(canonical), canonical, "formatting twice changes nothing");
}

#[test]
fn test_layout_that_would_change_the_meaning_stays() {
	let glued = "f(x) := x * 2\nit = 5\nprint f it-1\nprint [1 -1]\nprint 7//2\n";
	assert_eq!(formatted(glued), glued);
}

#[test]
fn test_offside_block_indented_by_tabs() {
	assert_eq!(
		formatted("def f(x):\n    y = x*2\n    y + 1\nprint f(3)\n"),
		"def f(x):\n\ty = x * 2\n\ty + 1\nprint f(3)\n"
	);
}

#[test]
fn test_hash_comments_only_when_asked() {
	let code = "x = 7 /! 2 // half\n/// doc\nprint x\n";
	assert_eq!(formatted(code), code);
	let hashed = format(code, Options { hash_comments: true }).unwrap().text;
	assert_eq!(hashed, "x = 7 /! 2 # half\n## doc\nprint x\n");
}

#[test]
fn test_soft_hints_never_rewrite() {
	let code = "cnt = str(3) // three\n";
	let hints: Vec<String> =
		soft_hints(code).into_iter().map(|hint| format!("{}: {}", hint.line, hint.message)).collect();
	assert!(hints.iter().any(|hint| hint.starts_with("1: warp avoids abbreviations: cnt → count")), "{hints:?}");
	assert!(hints.iter().any(|hint| hint.contains("str → text")), "{hints:?}");
	assert!(hints.iter().any(|hint| hint.contains("prefer # comments")), "{hints:?}");
	assert_eq!(formatted(code), code);
}

#[test]
fn test_unparsed_code_is_not_formatted() {
	assert!(format("print (1 2", Options::default()).is_err());
}

#[test]
fn test_parameter_types_spaced() {
	assert_eq!(formatted("exec(command:any) := command\nm = {a:1}\n"), "exec(command: any) := command\nm = {a: 1}\n");
}

// card format-spaces: every binary operator of the operator table is spaced, Unicode spellings too; a generic type
// `list<int>` stays glued (the spacing would change the tree)
#[test]
fn test_every_binary_operator_spaced() {
	assert_eq!(
		formatted("a = 1<2\nb = a<3 and 4>a\nc = 3≤4\nd = a≠1\ne = 2×3\n"),
		"a = 1 < 2\nb = a < 3 and 4 > a\nc = 3 ≤ 4\nd = a ≠ 1\ne = 2 × 3\n"
	);
	let generic = "xs = list<int>\n";
	assert_eq!(formatted(generic), generic);
}

#[test]
fn test_no_soft_hints_inside_a_shader_block() {
	let code = "obj = shader {\n\t// WGSL\n\tfn main() -> f32 { return 1.0; }\n}\nwave = wgsl { fn f() {} }\n";
	let hints: Vec<String> =
		soft_hints(code).into_iter().map(|hint| format!("{}: {}", hint.line, hint.message)).collect();
	assert_eq!(hints, ["1: warp avoids abbreviations: obj → object (in obj)"]);
	let named = "point = {\n\t# a wgsl { block reads it\n\tcnt: 1\n}\nnote = \"shader {\"\nmsg = \"}\"\n";
	let hints: Vec<String> = soft_hints(named).into_iter().map(|hint| hint.message).collect();
	assert_eq!(hints.len(), 2, "{hints:?}");
}
