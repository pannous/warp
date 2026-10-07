// card site-tasks: a site's program may start tasks; the build-time render (main, then page·html, natively) links the
// task words as a plain run does
use crate::common::scratch_directory;

#[test]
fn a_site_renders_what_its_tasks_computed() {
	let directory = scratch_directory("tasks-site");
	warp::site::build("twice(x) := x * 2\nanswer = await go twice(21)\np{ \"answer \" + answer }", "tasks", &directory).expect("the site is built");
	let page = std::fs::read_to_string(directory.join("index.html")).unwrap();
	assert!(page.contains("<p>answer 42</p>"), "{page}");
	std::fs::remove_dir_all(directory).unwrap();
}
