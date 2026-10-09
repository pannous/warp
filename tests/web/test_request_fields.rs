// card g_mQ9U: in a served route `$title` is the request's field title: `request.body.title` of a post (its form or
// JSON body), `request.query.title` of a get (`/search?q=tea`)
use std::time::Duration;

const PROGRAM: &str = "post \"/todos\" { \"added \" + $title }\nget \"/search\" { \"looking for \" + $q }";
const PORT: u16 = 18658;
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

fn url(path: &str) -> String {
	format!("http://127.0.0.1:{PORT}{path}")
}

#[test]
fn a_dollar_name_is_the_request_s_field() {
	let server = served(2);
	let agent: ureq::Agent = ureq::Agent::config_builder().http_status_as_error(false).build().into();
	let posted = agent.post(&url("/todos")).header("Content-Type", FORM_TYPE).send("title=tea").expect("an answer").body_mut().read_to_string().expect("a text");
	assert_eq!(posted, "added tea");
	let searched = agent.get(&url("/search?q=jam")).call().expect("an answer").body_mut().read_to_string().expect("a text");
	assert_eq!(searched, "looking for jam");
	let _ = server.join();
}
