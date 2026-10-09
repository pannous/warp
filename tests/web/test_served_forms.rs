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

// without a server (`warp run`, the playground) the routes wait for the page's forms (page·submitted): the program
// runs, its routes do not
#[test]
fn routes_without_a_server_wait_for_the_page() {
	crate::is!(&format!("items = [1]\n{PROGRAM}\npost \"/items\" {{ items.add(2) }}\ncount(items)"), 1);
}

// samples/todo_app.warp served, as the samples check asks it with curl: a todo is created, toggled and listed
#[test]
fn the_todo_sample_creates_toggles_and_lists() {
	const TODO_PORT: u16 = 18647;
	const TODO_APP: &str = include_str!("../../samples/todo_app.warp");
	let program = program_with_rows("served_todo_app", TODO_APP, TODO_APP);
	let server = served_from(TODO_PORT, TODO_APP, Some(program), 5);
	let at = |path: &str| format!("http://127.0.0.1:{TODO_PORT}{path}");
	let posted = |path: &str, body: &str| agent().post(&at(path)).header("Content-Type", FORM_TYPE).send(body).expect("an answer").body_mut().read_to_string().expect("a text");
	let got = |path: &str| agent().get(&at(path)).call().expect("an answer").body_mut().read_to_string().expect("a text");
	assert_eq!(posted("/todos", "title=tea"), r#"{"Todo":{"title":"tea","done":false,"id":1}}"#);
	assert_eq!(posted("/todos", "title=buy+milk"), r#"{"Todo":{"title":"buy milk","done":false,"id":2}}"#);
	assert_eq!(posted("/todos/1/toggle", ""), r#"{"Todo":{"title":"tea","done":true,"id":1}}"#);
	assert_eq!(got("/api/todos"), r#"[{"Todo":{"title":"tea","done":true,"id":1}},{"Todo":{"title":"buy milk","done":false,"id":2}}]"#);
	assert_eq!(got("/api/open"), r#"[{"Todo":{"title":"buy milk","done":false,"id":2}}]"#);
	server.join().expect("the server thread");
}

// a filter of a table that keeps no row answers [] as the table itself does (card served-empty), not null
#[test]
fn an_empty_filter_answers_an_empty_list() {
	const FILTER_PORT: u16 = 18649;
	const TABLE: &str = "class Todo{title: text; done: bool}\nstored todos: [Todo]\nget \"/open\" { todos where not done }";
	let program = program_with_rows("served_empty_filter", TABLE, "class Todo{title: text; done: bool}\nstored todos: [Todo]\ntodos.add(Todo(\"tea\", true))");
	let server = served_from(FILTER_PORT, TABLE, Some(program), 1);
	let open = agent().get(&format!("http://127.0.0.1:{FILTER_PORT}/open")).call().expect("an answer").body_mut().read_to_string().expect("a text");
	assert_eq!(open, "[]");
	server.join().expect("the server thread");
}

// the edits a user makes to the todo sample (card todo-app robust): a priority field the old table gains, delete, a
// title edit, the list sorted by priority, the open ones counted in the header, an empty title refused with a message
const EDITED_TODO_APP: &str = r#"class Todo{title: text; done: bool; priority: int}
stored todos: [Todo]
get "/api/todos" { todos sorted by priority }
post "/todos" {
	if not request.body.title { raise "a todo needs a title" }
	todos.add(Todo(request.body.title, false, int(request.body.priority)))
}
post "/todos/:id:int/toggle" {
	todo = (todos where it.id == id)#1
	todo.done = not todo.done
	todo
}
post "/todos/:id:int/title" {
	todo = (todos where it.id == id)#1
	todo.title = request.body.title
	todo
}
post "/todos/:id:int/delete" { todos.remove((todos where it.id == id)#1) }
route "/" {
	div{
		h1{ "Todos (" + count(todos where not done) + " open)" }
		ul{ for todo in todos sorted by priority { li{ todo.title } } }
	}
}"#;

#[test]
fn the_todo_sample_takes_a_users_edits() {
	const EDITED_PORT: u16 = 18651;
	let program = program_with_rows("served_todo_edits", EDITED_TODO_APP, "class Todo{title: text; done: bool}\nstored todos: [Todo]\ntodos.add(Todo(\"tea\", false))");
	let server = served_from(EDITED_PORT, EDITED_TODO_APP, Some(program), 8);
	let answer = |path: &str, body: &str| {
		let mut answer = agent().post(&format!("http://127.0.0.1:{EDITED_PORT}{path}")).header("Content-Type", FORM_TYPE).send(body).expect("an answer");
		(answer.status().as_u16(), answer.body_mut().read_to_string().expect("a text"))
	};
	let got = |path: &str| agent().get(&format!("http://127.0.0.1:{EDITED_PORT}{path}")).call().expect("an answer").body_mut().read_to_string().expect("a text");
	let (_, milk) = answer("/todos", "title=milk&priority=2");
	assert!(milk.contains(r#""title":"milk""#) && milk.contains(r#""priority":2"#), "{milk}");
	let (_, bread) = answer("/todos", "title=bread&priority=1");
	assert!(bread.contains(r#""id":3"#), "{bread}");
	let (status, refused) = answer("/todos", "title=&priority=1");
	assert_eq!((status, refused.contains("a todo needs a title")), (500, true), "{refused}");
	let (_, toggled) = answer("/todos/1/toggle", "");
	assert!(toggled.contains(r#""title":"tea","done":true"#) && toggled.contains(r#""priority":0"#), "{toggled}");
	let (_, renamed) = answer("/todos/2/title", "title=oat+milk");
	assert!(renamed.contains(r#""title":"oat milk""#), "{renamed}");
	answer("/todos/3/delete", "");
	let listed = got("/api/todos");
	let position = |title: &str| listed.find(&format!(r#""title":"{title}""#));
	assert!(position("tea") < position("oat milk") && position("bread").is_none(), "{listed}");
	let page = got("/");
	assert!(page.contains("Todos (1 open)"), "{page}");
	server.join().expect("the server thread");
}

// a route's refusal answers by its cause (supervisor default 2026-10-09): a raise to a browser's form 400 with the page it
// came from and the message, a form field the request lacks 400 "missing field …", a path parameter's lookup that finds
// nothing 404; a real bug stays 500
#[test]
fn a_refused_request_answers_400_or_404() {
	const REFUSING_PORT: u16 = 18654;
	let program = program_with_rows("served_refusals", EDITED_TODO_APP, "class Todo{title: text; done: bool; priority: int}\nstored todos: [Todo]\ntodos.add(Todo(\"tea\", false, 1))");
	let server = served_from(REFUSING_PORT, EDITED_TODO_APP, Some(program), 4);
	let at = |path: &str| format!("http://127.0.0.1:{REFUSING_PORT}{path}");
	let answer = |path: &str, body: &str, browser: bool| {
		let request = agent().post(&at(path)).header("Content-Type", FORM_TYPE);
		let request = if browser { request.header("Accept", "text/html,*/*").header("Referer", &at("/")) } else { request };
		let mut answer = request.send(body).expect("an answer");
		(answer.status().as_u16(), answer.body_mut().read_to_string().expect("a text"))
	};
	let (status, page) = answer("/todos", "title=&priority=1", true);
	assert_eq!(status, 400, "{page}");
	assert!(page.contains("a todo needs a title") && page.contains("Todos (1 open)"), "{page}");
	assert_eq!(answer("/todos", "priority=1", false), (400, "missing field title".to_string()));
	assert_eq!(answer("/todos/9/delete", "", false).0, 404);
	assert_eq!(answer("/todos", "title=tea&priority=x", false).0, 500);
	server.join().expect("the server thread");
}
