// card unix-words: `~`, `~/…` and `/abs/…` are file paths without quotes, like `./…`: `ls ~/`, `cd ~`, `x = /tmp`,
// `ls /tmp` after a function, `cp /a /b` after a path; `width /height` after a variable still divides, `a ~ b` compares
use crate::is;
use warp::{parse, Bracket, Node, Separator};

fn command(word: &str, path: &str) -> Node {
	Node::List(vec![Node::Symbol(word.into()), Node::Text(path.into())], Bracket::None, Separator::Space)
}

#[test]
fn a_tilde_starts_a_home_path() {
	assert_eq!(parse("ls ~/"), command("ls", "~/"));
	assert_eq!(parse("cat ~/notes/a.txt"), command("cat", "~/notes/a.txt"));
	assert_eq!(parse("cd ~"), command("cd", "~"));
	is!("x = ~/a.txt\nx", "~/a.txt");
}

#[test]
fn a_slash_starts_an_absolute_path_where_no_operand_divides() {
	assert_eq!(parse("ls /tmp"), command("ls", "/tmp"));
	assert_eq!(parse("cat /etc/hosts"), command("cat", "/etc/hosts"));
	is!("x = /etc/hosts\nx", "/etc/hosts");
	is!("[/a, /b/c.txt]", parse("[\"/a\" \"/b/c.txt\"]"));
}

#[test]
fn division_and_comparison_stay() {
	is!("width = 6\nheight = 2\nwidth /height", 3);
	is!("x = 1\nx ~ 1", true);
	is!("6 / 2", 3);
}

#[test]
fn a_path_follows_a_path_or_a_function() {
	let copy = Node::List(
		vec![Node::Symbol("cp".into()), Node::Text("/a/x".into()), Node::Text("/b".into())],
		Bracket::None,
		Separator::Space,
	);
	assert_eq!(parse("cp /a/x /b"), copy);
	is!("[/a /b]", parse("[\"/a\" \"/b\"]"));
	is!("show(p) := p\nshow /tmp", "/tmp");
	is!("area(w, h) := w /h\narea(6, 2)", 3);
}
