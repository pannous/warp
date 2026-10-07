//! Lists in markup (card web-keyed, notes/web_framework.md step 5): a comprehension or map inside an element gives its
//! children; `key:` names an item, so the page moves its element instead of rewriting it (playground.js morphChildren,
//! tour example "keyed list")
use warp::markup::to_html;
use warp::wasm_emitter::eval;

fn html_of(code: &str) -> String {
	to_html(&eval(code))
}

#[test]
fn a_list_inside_an_element_gives_its_children() {
	let items = "<li>a</li><li>b</li>";
	assert_eq!(html_of("todos = [\"a\" \"b\"]\nul{ [li{ t } for t in todos] }"), format!("<ul>{items}</ul>"));
	assert_eq!(html_of("todos = [\"a\" \"b\"]\nul{ todos.map(t => li{ t }) }"), format!("<ul>{items}</ul>"));
	assert_eq!(html_of("todos = [\"a\" \"b\"]\nul{ h2{\"todo\"} [li{ t } for t in todos] }"), format!("<ul><h2>todo</h2>{items}</ul>"));
}

#[test]
fn a_key_names_the_element_of_an_item() {
	let html = html_of("todos = [{id:7 text:\"a\"} {id:9 text:\"b\"}]\nul{ [li{ key: todo.id todo.text } for todo in todos] }");
	assert_eq!(html, "<ul><li data-wasp-key=\"7\">a</li><li data-wasp-key=\"9\">b</li></ul>");
}
