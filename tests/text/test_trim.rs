// `trim(text)`, `text.trim()`: the text without the spaces, tabs and line breaks at either end
use warp::is;

#[test]
fn trim_drops_whitespace_at_both_ends() {
	is!("\"  hi \".trim()", "hi");
	is!("trim(\"\\t a b \\n\")", "a b");
	is!("s = \"   \"; #s.trim()", 0);
	is!("lines = \" x , y \".split(\",\"); lines.map(l => l.trim()).join(\"|\")", "x|y");
}

/// `strip` is Python's spelling of trim; `s.trim` needs no parentheses, like `s.upper`
#[test]
fn strip_and_trim_without_parentheses() {
	is!("\"  hi \".strip()", "hi");
	is!("strip(\" a \")", "a");
	is!("s = \" ab \"; strip s", "ab");
	is!("s = \" ab \"; s.trim", "ab");
	is!("s = \" ab \"; s.strip + \"!\"", "ab!");
}
