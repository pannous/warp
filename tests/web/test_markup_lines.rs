// card todo-app: the run-time renderer (lib/markup.warp, a page showing server data) takes a block's first line of
// attributes as attributes (`form{ method:"post" action:"/t"` and more lines; it rendered the pairs as text), and
// aria-* attributes (the accessibility hint suggests aria-label; it was dropped)
use crate::is;

#[test]
fn a_line_of_attributes_renders_as_attributes() {
	is!("use markup\nto_html(form{ method:\"post\" action:\"/t\"\n button{\"add\"} })", "<form method=\"post\" action=\"/t\"><button>add</button></form>");
}

#[test]
fn an_aria_attribute_renders() {
	is!("use markup\nto_html(input{ name:\"t\" aria-label:\"new todo\" })", "<input name=\"t\" aria-label=\"new todo\">");
}
