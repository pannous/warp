// card todo-app: an HTML form POSTs to a served `post` route. Its urlencoded body is the map of its fields
// (`request.body.title`), a browser's form (it accepts text/html) gets 303 See Other back to the page it came from
// (post/redirect/get, so a reload does not post again), any other client the route's value; a route's path may hold
// parameters as a page's route does (`post "/todos/:id:int/toggle"`, id bound)
use std::time::Duration;

const PROGRAM: &str = "post \"/todos\" { request.body.title }\npost \"/todos/:id:int/toggle\" { id * 2 }\nget \"/hello/:name\" { \"hi \" + name }";
const PORT: u16 = 18630;
const FORM_TYPE: &str = "application/x-www-form-urlencoded";

fn served(requests: usize) -> std::thread::JoinHandle<String> {
	let server = std::thread::spawn(move || {
		warp::web_server::stop_after(requests);
		warp::pipeline::serving_at(PORT, || warp::wasm_emitter::eval(PROGRAM)).serialize()
	});
	let started = std::time::Instant::now();
	while std::net::TcpStream::connect(("127.0.0.1", PORT)).is_err() {
		assert!(started.elapsed() < Duration::from_secs(60), "the server did not start");
		std::thread::sleep(Duration::from_millis(50));
	}
	server
}

fn agent() -> ureq::Agent {
	ureq::Agent::config_builder().max_redirects(0).http_status_as_error(false).build().into()
}

fn url(path: &str) -> String {
	format!("http://127.0.0.1:{PORT}{path}")
}

#[test]
fn a_form_posts_to_a_served_route() {
	let server = served(5);
	let posted = agent().post(&url("/todos")).header("Content-Type", FORM_TYPE).send("title=buy+milk+%26+eggs").expect("an answer").body_mut().read_to_string().expect("a text");
	assert_eq!(posted, "buy milk & eggs");
	let mut browser = agent().post(&url("/todos")).header("Content-Type", FORM_TYPE).header("Accept", "text/html,*/*").header("Referer", &url("/list?all=1")).send("title=tea").expect("an answer");
	assert_eq!(browser.status().as_u16(), 303);
	assert_eq!(browser.headers().get("Location").and_then(|location| location.to_str().ok()), Some("/list?all=1"));
	let _ = browser.body_mut().read_to_string();
	let toggled = agent().post(&url("/todos/3/toggle")).send("").expect("an answer").body_mut().read_to_string().expect("a text");
	assert_eq!(toggled, "6");
	let not_an_int = agent().post(&url("/todos/x/toggle")).send("").expect("an answer");
	assert_eq!(not_an_int.status().as_u16(), 404);
	let hello = agent().get(&url("/hello/Ann")).call().expect("an answer").body_mut().read_to_string().expect("a text");
	assert_eq!(hello, "hi Ann");
	server.join().expect("the server thread");
}
