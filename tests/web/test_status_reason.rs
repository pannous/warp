// card status-reason: an answer's status line carries the status's real reason phrase, also for codes tiny_http
// does not know (`status 418` answered "418 Unknown")
use std::io::{Read, Write};
use std::time::Duration;

const PORT: u16 = 18456;
const PROGRAM: &str = r#"serve 18456 {
	get "/teapot" { status 418; header "X-Warp" "brewed"; "short and stout" }
}"#;

#[test]
fn a_status_line_names_the_real_reason() {
	let server = std::thread::spawn(|| {
		warp::web::web_server::stop_after(1);
		warp::wasm_emitter::eval(PROGRAM).serialize()
	});
	let started = std::time::Instant::now();
	let mut stream = loop {
		if let Ok(stream) = std::net::TcpStream::connect(("127.0.0.1", PORT)) {
			break stream;
		}
		assert!(started.elapsed() < Duration::from_secs(60), "the server did not start");
		std::thread::sleep(Duration::from_millis(50));
	};
	stream.write_all(b"GET /teapot HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n").unwrap();
	let mut answer = String::new();
	stream.read_to_string(&mut answer).unwrap();
	assert!(answer.starts_with("HTTP/1.1 418 I'm a teapot\r\n"), "{answer}");
	assert!(answer.contains("X-Warp: brewed\r\n"), "{answer}");
	assert!(answer.ends_with("\r\n\r\nshort and stout"), "{answer}");
	server.join().expect("the server thread");
}
