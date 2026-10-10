// card site-build: warp build --site prerenders by running main natively; an animation stops at its first frame or
// sleep there, the page shows the rest, so an endless `while yes { …; show(); sleep 16 ms }` builds too
use crate::common::scratch_directory;
use std::sync::mpsc;
use std::time::Duration;

const BUILD_LIMIT: Duration = Duration::from_secs(120);

fn built_in_time(code: &'static str, name: &'static str) -> Vec<String> {
	let (sent, built) = mpsc::channel();
	std::thread::spawn(move || {
		let directory = scratch_directory(name);
		let site = warp::site::build(code, name, &directory).map(|site| site.files);
		let _ = std::fs::remove_dir_all(&directory);
		sent.send(site)
	});
	let site = built.recv_timeout(BUILD_LIMIT).unwrap_or_else(|_| panic!("{name}: the build still runs after {BUILD_LIMIT:?}"));
	site.expect("the site is built")
}

#[test]
fn an_endless_animation_builds() {
	let files = built_in_time("use draw\ncanvas(8, 8)\nframes = 0\nwhile yes {\n\tframes += 1\n\tdot(frames % 8, 1, red)\n\tshow()\n\tsleep 16 ms\n}", "endless-animation");
	assert!(files.contains(&"index.html".to_string()) && files.contains(&"app.wasm".to_string()), "{files:?}");
}

#[test]
fn an_endless_sleeping_loop_builds() {
	let files = built_in_time("ticks = 0\nwhile yes {\n\tticks += 1\n\tsleep 16 ms\n}\np{ \"ticks\" }", "endless-sleep");
	assert!(files.contains(&"index.html".to_string()), "{files:?}");
}
