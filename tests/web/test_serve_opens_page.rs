// card samples-serving: `warp samples/servers/route_headers.warp` from a terminal serves and opens its page in the browser;
// the CLI asks web_server::when_listening for that, which hands the address over once the server listens
use std::sync::Mutex;
use std::time::Duration;

const PORT: u16 = 18472;
const PROGRAM: &str = r#"serve 18472 { get "/" { "hi" } }"#;

static HEARD: Mutex<Vec<String>> = Mutex::new(vec![]);

fn heard(address: &str) {
	HEARD.lock().unwrap().push(address.to_string());
}

#[test]
fn a_listening_server_hands_its_address_over_once() {
	let server = std::thread::spawn(|| {
		warp::web::web_server::stop_after(1);
		warp::web::web_server::when_listening(heard);
		warp::wasm_emitter::eval(PROGRAM).serialize()
	});
	let started = std::time::Instant::now();
	while std::net::TcpStream::connect(("127.0.0.1", PORT)).is_err() {
		assert!(started.elapsed() < Duration::from_secs(60), "the server did not start");
		std::thread::sleep(Duration::from_millis(50));
	}
	let answer = ureq::get(&format!("http://127.0.0.1:{PORT}/")).call().expect("an answer").body_mut().read_to_string();
	assert_eq!(answer.expect("a text"), "hi");
	server.join().expect("the server thread");
	assert_eq!(*HEARD.lock().unwrap(), vec![format!("http://localhost:{PORT}/")]);
}
