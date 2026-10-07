// fetch of a URL computed at run time: a variable, a parameter, a concatenation (it fetched the variable's name,
// `fetch u failed: bad uri: u is missing scheme`; found writing lib/netbase.wasp, card netbase-package)
use crate::common::serve;
use crate::is;

#[test]
fn fetch_of_a_text_variable() {
	is!(&format!("u = \"{}\"; fetch(u)", serve("200 OK", "hello")), "hello\n");
}

#[test]
fn fetch_of_a_parameter_and_a_concatenation() {
	let url = serve("200 OK", "hi");
	let server = url.trim_end_matches("/data");
	is!(&format!("get(u) := fetch(u); get(\"{url}\")"), "hi\n");
	is!(&format!("get(server) := fetch(server + \"/data\"); get(\"{server}\")"), "hi\n");
}
