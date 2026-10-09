// Card data-string: the text of quoted code is the code as written, its operators kept (a run-time entry joins by its
// operator, not always `:`)
use crate::is;

#[test]
fn quoted_code_reads_as_written() {
	is!("string(data x+1)", "x+1");
	is!("string(data(x+1))", "x+1");
	is!("f(x) := x+1\nstring(f.body)", "x+1");
	is!("q = data x*2\nstring(q)", "x*2");
	is!("string({a:1})", "{a:1}");
}
