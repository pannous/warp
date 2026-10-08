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

// card single-page (user, 2026-10-07: one HTML page, not one per route): a site with routes is one page, index.html,
// and its copy 404.html, which a static host serves for any other path (a deep link); it finds the site's files through
// <base href="/"> and its router shows the path's route (in a browser: probes/site/deep_link_in_browser.sh)
#[test]
fn a_site_with_routes_is_one_page_for_every_path() {
	let directory = scratch_directory("routes-site");
	let program = "route \"/\" { h1{ \"Home\" } }\nroute \"/about\" { p{ \"About us\" } }\nroute \"/users/:id:int\" { p{ \"User \" + id } }\nroute \"*\" { p{ \"no such page\" } }";
	let site = warp::site::build(program, "routes", &directory).expect("the site is built");
	assert!(site.files.contains(&"404.html".to_string()) && !site.files.iter().any(|file| file.contains('/')), "{:?}", site.files);
	let page = |file: &str| std::fs::read_to_string(directory.join(file)).unwrap();
	assert!(page("index.html").contains("<h1>Home</h1>") && !page("index.html").contains("<base"));
	assert_eq!(page("404.html"), page("index.html").replace("</title>\n", "</title>\n<base href=\"/\">\n"));
	std::fs::remove_dir_all(directory).unwrap();
	let unrouted = warp::site::files("p{ \"hi\" }", "plain", false).expect("the site is built");
	assert!(!unrouted.iter().any(|(file, _)| file == "404.html"));
}

// card site-worker: a module that starts tasks runs in a Worker, where a blocking await may wait and its tasks run
// together: the page loads site-thread.js instead of the host, whose scripts its root lists for the Worker; a page
// without tasks (COUNTER) stays on the page's thread
#[test]
fn a_site_starting_tasks_runs_its_program_in_a_worker() {
	let directory = scratch_directory("tasks-site");
	let site = warp::site::build("nap(ms) := { sleep(ms); 1 }\nfirst = go nap(1)\np{ \"naps: \" + await first }", "tasks", &directory).expect("the site is built");
	assert_eq!(site.files, ["index.html", "app.wasm", "site-thread.js", "markup.js", "site.js", "reader.js", "host.js", "host-tasks.js", "site-worker.js", "task-worker.js", "coi-serviceworker.js"]);
	let page = std::fs::read_to_string(directory.join("index.html")).unwrap();
	assert!(page.contains(r#"<div id="wasp-root" data-wasp-worker="reader.js,host.js,host-tasks.js"><p>naps: 1</p></div>"#), "{page}");
	assert!(page.contains(r#"<script src="site-thread.js"></script>"#) && !page.contains(r#"<script src="host.js">"#), "{page}");
	std::fs::remove_dir_all(directory).unwrap();
}

// card site-worker-step (step 2): a program with routes that starts tasks runs in the Worker too; the page keeps
// host-routes.js for its links, the back button and the focus, and the Worker shows each route the page sends it
#[test]
fn a_site_with_routes_starting_tasks_runs_its_program_in_a_worker() {
	let directory = scratch_directory("routed-tasks-site");
	let program = "nap(ms) := { sleep(ms); 1 }\nfirst = go nap(1)\nnaps = await first\nroute \"/\" { p{ \"naps: \" + naps } }\nroute \"/other\" { p{ \"other\" } }";
	let site = warp::site::build(program, "routed", &directory).expect("the site is built");
	let page = std::fs::read_to_string(directory.join("index.html")).unwrap();
	assert!(page.contains(r#"<script src="site-thread.js"></script>"#) && page.contains(r#"<script src="host-routes.js"></script>"#) && !page.contains(r#"<script src="host.js">"#), "{page}");
	assert!(page.contains(r#"data-wasp-worker="reader.js,host.js,host-tasks.js,host-routes.js""#), "{page}");
	assert!(site.files.iter().any(|file| file == "site-worker.js") && site.files.iter().any(|file| file == "404.html"), "{:?}", site.files);
	std::fs::remove_dir_all(directory).unwrap();
}
