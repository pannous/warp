// card web-ssr: `warp build --site` writes a directory a web server serves: index.html with the program's markup
// rendered at build time (readable without JavaScript), app.wasm, which the loader runs in the page to hydrate it, and
// the playground's scripts the loader needs
use crate::common::scratch_directory;

const COUNTER: &str = "count = 0\ndiv{ button{ on click { count += 1 } \"Add\" } p{ \"clicked \" + count } }";

#[test]
fn a_site_holds_the_rendered_page_the_module_and_its_scripts() {
	let directory = scratch_directory("counter-site");
	let site = warp::site::build(COUNTER, "counter", &directory).expect("the site is built");
	assert_eq!(site.files, ["index.html", "app.wasm", "reader.js", "host.js", "markup.js", "site.js"]);
	let page = std::fs::read_to_string(directory.join("index.html")).unwrap();
	assert!(page.contains(r#"<div id="wasp-root"><div><button data-wasp-click="1">Add</button><p>clicked 0</p></div></div>"#), "{page}");
	assert!(page.contains("<title>counter</title>") && page.contains(r#"<script src="site.js"></script>"#), "{page}");
	let module = std::fs::read(directory.join("app.wasm")).unwrap();
	let names = String::from_utf8_lossy(&module);
	// the page's host reads values through the reflection getters, calls the button's handler and renders the page anew
	assert!(names.contains("reflect_data") && names.contains("on·click·1·node") && names.contains("page·html"));
	std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn a_program_failing_at_build_time_builds_no_site() {
	let directory = scratch_directory("failing-site");
	let failure = warp::site::build("div{ p{ 1 / undefined_name } }", "failing", &directory).err().expect("no site");
	assert!(failure.contains("undefined_name"), "{failure}");
	assert!(!directory.join("index.html").exists());
}

/// A page without handlers is static: its last line, rendered by the program at build time
#[test]
fn a_static_page_is_its_last_line_rendered() {
	let directory = scratch_directory("static-site");
	warp::site::build("title = \"Docs\"\narticle{ h1{ title } p{ \"1 < 2\" } }", "docs", &directory).expect("the site is built");
	let page = std::fs::read_to_string(directory.join("index.html")).unwrap();
	assert!(page.contains(r#"<div id="wasp-root"><article><h1>Docs</h1><p>1 &lt; 2</p></article></div>"#), "{page}");
	std::fs::remove_dir_all(directory).unwrap();
}
