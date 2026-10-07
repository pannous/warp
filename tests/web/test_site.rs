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

// card route-prerender: each route without parameters is rendered at build time into its own <path>/index.html, which
// names the site's files from its depth, so a deep link reads without JavaScript and hydrates; routes with parameters
// and "*" are rendered by the page itself
#[test]
fn a_site_prerenders_each_static_route() {
	let directory = scratch_directory("routes-site");
	let program = "route \"/\" { h1{ \"Home\" } }\nroute \"/about\" { p{ \"About us\" } }\nroute \"/docs/intro\" { p{ \"Intro\" } }\nroute \"/users/:id:int\" { p{ \"User \" + id } }\nroute \"*\" { p{ \"no such page\" } }";
	let site = warp::site::build(program, "routes", &directory).expect("the site is built");
	assert!(site.files.contains(&"about/index.html".to_string()) && site.files.contains(&"docs/intro/index.html".to_string()), "{:?}", site.files);
	assert!(!site.files.iter().any(|file| file.contains(':') || file.contains('*')), "{:?}", site.files);
	let page = |file: &str| std::fs::read_to_string(directory.join(file)).unwrap();
	assert!(page("index.html").contains("<h1>Home</h1>"));
	assert!(page("about/index.html").contains("<p>About us</p>") && page("about/index.html").contains(r#"<script src="../site.js"></script>"#), "{}", page("about/index.html"));
	assert!(page("docs/intro/index.html").contains(r#"<script src="../../site.js"></script>"#));
	std::fs::remove_dir_all(directory).unwrap();
}
