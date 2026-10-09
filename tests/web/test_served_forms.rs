// card todo-app: an HTML form POSTs to a served `post` route. Its urlencoded body is the map of its fields
// (`request.body.title`), a browser's form (it accepts text/html) gets 303 See Other back to the page it came from
// (post/redirect/get, so a reload does not post again), any other client the route's value; a route's path may hold
// parameters as a page's route does (`post "/todos/:id:int/toggle"`, id bound)
use std::time::Duration;

const PROGRAM: &str = "post \"/todos\" { request.body.title }\npost \"/todos/:id:int/toggle\" { id * 2 }\nget \"/hello/:name\" { \"hi \" + name }";
const PORT: u16 = 18630;
const FORM_TYPE: &str = "application/x-www-form-urlencoded";

fn served(requests: usize) -> std::thread::JoinHandle<String> {
	served_from(PORT, PROGRAM, None, requests)
}

/// The program served at the port for `requests` requests, from the program file (its tables beside it) when given
fn served_from(port: u16, code: &'static str, file: Option<std::path::PathBuf>, requests: usize) -> std::thread::JoinHandle<String> {
	let server = std::thread::spawn(move || {
		warp::web_server::stop_after(requests);
		let serving = || warp::pipeline::serving_at(port, || warp::wasm_emitter::eval(code));
		match file {
			Some(file) => warp::modules::with_program_file(&file, serving),
			None => serving(),
		}.serialize()
	});
	let started = std::time::Instant::now();
	while std::net::TcpStream::connect(("127.0.0.1", port)).is_err() {
		assert!(started.elapsed() < Duration::from_secs(60), "the server did not start");
		std::thread::sleep(Duration::from_millis(50));
	}
	server
}

/// scratch/<folder>/app.warp holding `code`, its table beside it fresh and filled by running `adding`
fn program_with_rows(folder: &str, code: &str, adding: &str) -> std::path::PathBuf {
	let folder = std::path::Path::new("scratch").join(folder);
	std::fs::create_dir_all(&folder).expect("scratch folder");
	let _ = std::fs::remove_file(folder.join("app.database.sqlite"));
	let program = folder.join("app.warp");
	std::fs::write(&program, code).expect("the program file");
	warp::modules::with_program_file(&program, || warp::wasm_emitter::eval(adding));
	program
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

// a path parameter is a variable of the route's block for a table's filter too (it went to SQL as an unknown word)
#[test]
fn a_path_parameter_filters_a_table() {
	const TABLE_PORT: u16 = 18639;
	const TABLE: &str = "class Todo{title: text; done: bool}\nstored todos: [Todo]\npost \"/todos/:id:int\" { (todos where it.id == id)#1.title }";
	let program = program_with_rows("served_filter", TABLE, "class Todo{title: text; done: bool}\nstored todos: [Todo]\ntodos.add(Todo(\"milk\", false))\ntodos.add(Todo(\"tea\", false))");
	let server = served_from(TABLE_PORT, TABLE, Some(program), 1);
	let title = agent().post(&format!("http://127.0.0.1:{TABLE_PORT}/todos/2")).send("").expect("an answer").body_mut().read_to_string().expect("a text");
	assert_eq!(title, "tea");
	server.join().expect("the server thread");
}

// a route that traps answers 500 with what went wrong, as a failing run says it (it gave the wasm backtrace only)
#[test]
fn a_failing_route_answers_its_failure() {
	const FAILING_PORT: u16 = 18640;
	let server = served_from(FAILING_PORT, "get \"/boom\" { xs = [1, 2]; xs#5 }", None, 1);
	let mut answer = agent().get(&format!("http://127.0.0.1:{FAILING_PORT}/boom")).call().expect("an answer");
	assert_eq!(answer.status().as_u16(), 500);
	let message = answer.body_mut().read_to_string().expect("a text");
	assert!(message.contains("index out of range"), "{message}");
	server.join().expect("the server thread");
}

// the page leaves a server table out, but its rows keep their id: the shipped page starts from the server's rows (it
// failed "Todo has no field id" once the table had rows, so `warp serve` served no page)
#[test]
fn a_page_starts_from_rows_of_a_server_table() {
	const PAGE_PORT: u16 = 18646;
	const PAGE: &str = "class Todo{title: text}\nstored todos: [Todo]\nroute \"/\" { ul{ for todo in todos { li{ a{ href:\"/todos/\" + todo.id todo.title } } } } }";
	let program = program_with_rows("served_page_rows", PAGE, "class Todo{title: text}\nstored todos: [Todo]\ntodos.add(Todo(\"tea\"))");
	let server = served_from(PAGE_PORT, PAGE, Some(program), 1);
	let page = agent().get(&format!("http://127.0.0.1:{PAGE_PORT}/")).call().expect("an answer").body_mut().read_to_string().expect("a text");
	assert!(page.contains("<a href=\"/todos/1\">tea</a>"), "{page}");
	server.join().expect("the server thread");
}
