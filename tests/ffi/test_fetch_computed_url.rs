// fetch of a URL computed at run time: a variable, a parameter, a concatenation (it fetched the variable's name,
// `fetch u failed: bad uri: u is missing scheme`; found writing lib/extra/netbase.wasp, card netbase-package)
use crate::common::serve;
use crate::is;

#[test]
fn fetch_of_a_text_variable() {
	is!(&format!("u = \"{}\"; fetch(u)", serve("200 OK", "hello")), "hello\n");
}

#[test]
fn fetch_of_a_parameter_and_a_concatenation() {
	let url = serve("200 OK", "hi");
	// halves of the URL, not its path: in the browser the URL is the page's stub with the answer in its query
	let (server, path) = url.split_at(url.len() / 2);
	is!(&format!("get(u) := fetch(u); get(\"{url}\")"), "hi\n");
	is!(&format!("get(server) := fetch(server + \"{path}\"); get(\"{server}\")"), "hi\n");
}
