//! `warp build --site app.warp` (card web-ssr, notes/web_framework.md "Built sites"): a directory a static web server
//! serves as it is. index.html holds the HTML of what the program shows, rendered at build time by the program itself
//! (its export page·html, lowering/page_html.rs, run natively after main), so the page reads without JavaScript; app.wasm is the program, which the loader (web/playground/site.js)
//! runs in the page to hydrate it: the DOM stays, the handlers of its elements run, and what they change is shown anew.
//! The page loads the playground's own reader.js, host.js and markup.js, carried in the warp binary, and the parts of
//! host.js its module imports words of (HOST_PARTS, card web-bundle).

use crate::node::Node;
pub use crate::host_parts::{HostPart, Script, HOST_PARTS};
pub(crate) use crate::host_parts::{exports, host_scripts_of, imports_of, message_of, with_excerpt, GPU_WORDS, TASK_WORD_PREFIXES};
use crate::host::{FOREIGN_CALL, HOST_LIBRARY, PAINT};
use std::path::{Path, PathBuf};

const PAGE_FILE: &str = "index.html";
const MODULE_FILE: &str = "app.wasm";
/// The scripts every page loads after the host's (host_parts.rs HOST_SCRIPTS)
const PAGE_SCRIPTS: [Script; 2] = [("markup.js", include_str!("../web/playground/markup.js")), ("site.js", include_str!("../web/playground/site.js"))];
/// The part of markup.js for elements with a CSS transition, after it: only a module that names a transition has them
const TRANSITIONS_SCRIPT: Script = ("markup-transitions.js", include_str!("../web/playground/markup-transitions.js"));
const TRANSITION_WORD: &[u8] = b"transition";
/// The canvas a module that paints shows its paintings and animation frames in, and the pointer over it (mouse_x, …)
const PAINT_SCRIPT: Script = ("canvas.js", include_str!("../web/playground/canvas.js"));
/// A page whose program runs in a Worker (card site-worker) loads this before them, and site.js hands over to it
const THREAD_SCRIPT: Script = ("site-thread.js", include_str!("../web/playground/site-thread.js"));
/// The page makes the task Workers of the program's Worker, which asks for them (Firefox, card tour-firefox-stall), when
/// the Worker has the task pool (TASK_PART: not a site that only paints)
const TASK_WORKERS_SCRIPT: Script = ("task-workers.js", include_str!("../web/playground/task-workers.js"));
const TASK_PART: &str = "host-tasks.js";
/// What such a site ships besides: the program's Worker, the task Workers it starts, and the service worker that gives a
/// static host's page the cross-origin isolation shared memory needs
const WORKER_FILES: [Script; 3] = [
	("site-worker.js", include_str!("../web/playground/site-worker.js")),
	("task-worker.js", include_str!("../web/playground/task-worker.js")),
	("coi-serviceworker.js", include_str!("../web/playground/coi-serviceworker.js")),
];
/// The root's attribute listing the scripts of the program's Worker (site-thread.js WORKER_ATTRIBUTE)
const WORKER_ATTRIBUTE: &str = "data-warp-worker";
const LIST_SEPARATOR: &str = ",";

/// The parts the page keeps when the program runs in a Worker: host-routes.js follows links and the back button and
/// moves the focus, which only the page can (site-thread.js)
const PAGE_SIDE_PARTS: [&str; 1] = ["host-routes.js"];
const LINE_COMMENT: &str = "//";
/// The element holding the program's markup (site.js SITE_ROOT)
const ROOT_ID: &str = "warp-root";
/// The element holding the replies of a rendered page's server calls (host-tasks.js SERVER_REPLIES_ID)
const REPLIES_ID: &str = "warp-replies";
const PAGE_TEMPLATE: &str = r#"<!doctype html>
<html>
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{{title}}</title>
{{base}}</head>
<body>
<div id="{{root}}"{{worker}}>{{body}}</div>
{{replies}}{{scripts}}</body>
</html>
"#;

/// `warp dev` adds the script asking the dev server for new builds (src/dev_server.rs)
const DEV_SCRIPT: (&str, &str) = ("dev.js", include_str!("../web/playground/dev.js"));

/// The content types of a site's files, by their extension
const CONTENT_TYPES: [(&str, &str); 3] = [("html", "text/html; charset=utf-8"), ("js", "text/javascript; charset=utf-8"), ("wasm", "application/wasm")];
const OTHER_CONTENT: &str = "application/octet-stream";
const INDEX_PATH: &str = "/";
/// The page of a program with routes for any other path (card single-page): a static host such as GitHub Pages serves
/// 404.html for a path it has no file of, `warp dev` and `serve` do the same; the page's router shows that path's route
const ROUTED_PAGE_FILE: &str = "404.html";
/// Where the page finds the site's files: index.html beside them, 404.html at any depth from the site's root
const BESIDE: &str = "";
const SITE_ROOT: &str = "/";

/// What `warp build --site` wrote
pub struct BuiltSite {
	pub directory: PathBuf,
	pub files: Vec<String>,
}

/// A file of a site: its name and content
pub type SiteFile = (String, Vec<u8>);

/// The site of the program `code` in `directory`, titled `title`: the page, the module and the scripts
pub fn build(code: &str, title: &str, directory: &Path) -> Result<BuiltSite, String> {
	write_files(&files(code, title, false)?, directory)
}

/// The files written into the directory, made if missing
pub fn write_files(files: &[SiteFile], directory: &Path) -> Result<BuiltSite, String> {
	std::fs::create_dir_all(directory).map_err(|failure| format!("cannot make {}: {failure}", directory.display()))?;
	for (name, bytes) in files {
		std::fs::write(directory.join(name), bytes).map_err(|failure| format!("cannot write {name}: {failure}"))?;
	}
	Ok(BuiltSite { directory: directory.to_path_buf(), files: files.iter().map(|(name, _)| name.clone()).collect() })
}

/// The files of the site of `code`; `dev` adds dev.js. A failure names its position with the source line
pub fn files(code: &str, title: &str, dev: bool) -> Result<Vec<SiteFile>, String> {
	site_files(code, title, dev)?.map(|site| site.files).ok_or_else(|| format!("the program shows no page: it exports no {}", crate::page_html::PAGE_HTML))
}

/// The site a program serving its page serves (src/web_server.rs), none when its last line shows nothing
pub fn served_files(code: &str, title: &str) -> Result<Option<ServedSite>, String> {
	site_files(code, title, false)
}

/// A site as a server serves it: its files, and the page of a program asking the server rendered anew for each request
#[derive(Default)]
pub struct ServedSite {
	pub files: Vec<SiteFile>,
	renderer: Option<Renderer>,
	/// the patterns of the page's routes in order (page·routes)
	page_routes: Vec<String>,
}

/// P221 (user: the first visit gets finished HTML): the prerender's module run for the request's path gives the page's
/// markup and the replies of its server calls for that path, which the page answers its first fetches from
/// (host-tasks.js SERVER_REPLIES), so it starts showing what the server rendered
struct Renderer {
	bytes: Vec<u8>,
	imports: crate::wasm_reader::Imports,
	title: String,
	scripts: Vec<Script>,
	worker_scripts: Vec<Script>,
}

impl ServedSite {
	/// Whether no route of the page matches `path`, a path of no file (`route "*"` matches any: the single page answers
	/// every path, user 2026-10-07)
	pub fn matches_no_route_at(&self, path: &str) -> bool {
		file_at(&self.files, path).is_some_and(|(name, _)| name == ROUTED_PAGE_FILE) && !self.page_routes.iter().any(|pattern| crate::routes::path_matches(pattern, path))
	}

	/// The file a request path names (file_at); a page of a program asking the server is rendered for the path
	pub fn file_at(&self, path: &str) -> Option<Result<SiteFile, String>> {
		let (name, bytes) = file_at(&self.files, path)?;
		match &self.renderer {
			Some(renderer) if name == PAGE_FILE || name == ROUTED_PAGE_FILE => {
				let root = if name == PAGE_FILE { BESIDE } else { SITE_ROOT };
				Some(renderer.page_at(path, root).map(|page| (name.clone(), page.into_bytes())))
			}
			_ => Some(Ok((name.clone(), bytes.clone()))),
		}
	}
}

impl Renderer {
	fn page_at(&self, path: &str, root: &str) -> Result<String, String> {
		let names = [crate::page_html::PAGE_HTML, crate::serve::RPC_REQUESTS, crate::serve::RPC_VALUES];
		let read = crate::host::with_page_path(path, || crate::wasm_reader::read_exports_after_main(&self.bytes, self.imports, &names));
		let failed = |failure| format!("the page of {path} failed: {}", message_of(&crate::wasm_emitter::failed_run(failure)));
		let [html, requests, values]: [Node; 3] = read.map_err(failed)?.try_into().expect("three exports");
		Ok(page(&self.title, page_html_text(&html)?, &replies_script(&items_of(&requests), &items_of(&values)), &self.scripts, &self.worker_scripts, root))
	}
}

/// The page·html export's value, the page's HTML; ø when the prerender ended at an animation's first frame
/// (until_first_frame): the page starts empty and shows the frames
fn page_html_text(rendered: &Node) -> Result<&str, String> {
	match rendered.drop_meta() {
		Node::Text(html) => Ok(html),
		Node::Empty => Ok(""),
		_ => Err(format!("{} gave no text: {}", crate::page_html::PAGE_HTML, rendered.serialize())),
	}
}

/// A list's items, a single value as one
fn items_of(list: &Node) -> Vec<Node> {
	match list.drop_meta() {
		Node::List(items, _, _) => items.clone(),
		single => vec![single.clone()],
	}
}

/// The replies of the page's server calls as host-tasks.js reads them: each request's url and arguments with the reply
/// the server gives it (web_server.rs Answer: a text as text/plain, any other value as JSON)
fn replies_script(requests: &[Node], values: &[Node]) -> String {
	let replies: Vec<serde_json::Value> = requests.iter().zip(values).filter_map(|(request, value)| {
		let [url, arguments] = items_of(request).try_into().ok()?;
		let answer = crate::web_server::Answer::of(value);
		Some(serde_json::json!({
			"url": url.name(),
			"arguments": crate::foreign::json_of(&arguments),
			"body": String::from_utf8_lossy(&answer.body),
			"text": answer.content_type.starts_with(crate::web_server::TEXT_REPLY_TYPE),
		}))
	}).collect();
	// `</` would end the script element early
	let json = serde_json::Value::Array(replies).to_string().replace("</", "<\\/");
	format!("<script type=\"application/json\" id=\"{REPLIES_ID}\">{json}</script>\n")
}

fn site_files(code: &str, title: &str, dev: bool) -> Result<Option<ServedSite>, String> {
	let compile = |build: &dyn Fn() -> Result<crate::pipeline::CompiledModule, Node>| {
		let module = crate::pipeline::for_a_page(|| if dev { crate::pipeline::for_dev(build) } else { build() });
		module.map_err(|value| format!("nothing to compile: {}", with_excerpt(code, message_of(&value))))
	};
	let rendering = compile(&|| crate::pipeline::prerendering(|| crate::pipeline::compile(code)))?;
	if !exports(&rendering.bytes, crate::page_html::PAGE_HTML) {
		return Ok(None);
	}
	let imports = crate::wasm_reader::Imports { host: rendering.needs_host, wasi: rendering.needs_wasi, ffi: rendering.needs_ffi };
	let read_after_main = |name: &str| {
		warp_runtime::system_signals::until_first_frame(|| crate::wasm_reader::read_export_after_main(&rendering.bytes, imports, name)).map_err(|failure| format!("the program failed at build time: {}", with_excerpt(code, failure.to_string())))
	};
	// card page-dom: a main calling into the page (`use js document`) fails here, natively, where there is no page; the
	// page then starts empty and shows what main renders in the browser
	let rendered_here = read_after_main(crate::page_html::PAGE_HTML);
	let rendered = match rendered_here {
		Err(_) if calls_foreign_code(&rendering.bytes)? => {
			eprintln!("warning: main calls into the page (use js), which exists only in the browser: the page is rendered there, without JavaScript it is empty");
			Node::Text(String::new())
		}
		other => other?,
	};
	let page_routes: Vec<String> = match exports(&rendering.bytes, crate::routes::PAGE_ROUTES) {
		true => items_of(&read_after_main(crate::routes::PAGE_ROUTES)?).iter().map(crate::routes::pattern_text).collect(),
		false => vec![],
	};
	// a page calling server functions ships without them, starting from the values they gave here (lowering/serve.rs)
	let asks_the_server = exports(&rendering.bytes, crate::serve::RPC_VALUES);
	let rendering_bytes = rendering.bytes.clone();
	let module = if asks_the_server {
		let values = items_of(&read_after_main(crate::serve::RPC_VALUES)?);
		compile(&|| crate::pipeline::with_server_values(values.clone(), || crate::pipeline::compile(code)))?
	} else {
		rendering
	};
	let html = page_html_text(&rendered)?;
	let host_scripts = host_scripts_of(&module.bytes)?;
	let page_scripts = page_scripts_of(&module.bytes)?.into_iter().chain(dev.then_some(DEV_SCRIPT));
	let (scripts, worker_scripts): (Vec<Script>, Vec<Script>) = if runs_in_a_worker(&imports_of(&module.bytes)?) {
		let page_parts = host_scripts.iter().filter(|(name, _)| PAGE_SIDE_PARTS.contains(name)).copied();
		let task_workers = host_scripts.iter().any(|(name, _)| *name == TASK_PART).then_some(TASK_WORKERS_SCRIPT);
		(task_workers.into_iter().chain([THREAD_SCRIPT]).chain(page_parts).chain(page_scripts).collect(), host_scripts)
	} else {
		(host_scripts.into_iter().chain(page_scripts).collect(), vec![])
	};
	let page_file = |root: &str| page(title, html, "", &scripts, &worker_scripts, root).into_bytes();
	let routed = exports(&module.bytes, crate::routes::PAGE_ROUTES);
	// each route's own functions in a module the page loads when it shows the route; a dev page reloads whole anyway
	let split = if dev { None } else { crate::route_split::split_by_route(&module.bytes)? };
	let (primary, route_modules) = match split {
		Some(split) => (split.primary, split.routes),
		None => (module.bytes, vec![]),
	};
	let mut files = vec![(PAGE_FILE.to_string(), page_file(BESIDE)), (MODULE_FILE.to_string(), primary)];
	if routed {
		files.push((ROUTED_PAGE_FILE.to_string(), page_file(SITE_ROOT)));
	}
	files.extend(route_modules);
	let worker_files = if worker_scripts.is_empty() { &[][..] } else { &WORKER_FILES[..] };
	let shipped = scripts.iter().chain(worker_scripts.iter().filter(|script| !scripts.contains(script))).chain(worker_files);
	files.extend(shipped.map(|(name, text)| (name.to_string(), compacted(text).into_bytes())));
	let renderer = asks_the_server.then(|| Renderer { bytes: rendering_bytes, imports, title: title.to_string(), scripts, worker_scripts });
	Ok(Some(ServedSite { files, renderer, page_routes }))
}

/// The file of a site a request path names: "/" is the page, "/app.wasm" the module, …, any other path the page of a
/// program with routes (404.html)
pub fn file_at<'a>(files: &'a [SiteFile], path: &str) -> Option<&'a SiteFile> {
	let name = if path == INDEX_PATH { PAGE_FILE } else { path.trim_start_matches('/') };
	let named = |wanted: &str| files.iter().find(|(file, _)| file == wanted);
	named(name).or_else(|| named(ROUTED_PAGE_FILE))
}

/// The content type of a site's file, by its extension
pub fn content_type(name: &str) -> &'static str {
	let extension = Path::new(name).extension().and_then(|extension| extension.to_str()).unwrap_or_default();
	CONTENT_TYPES.iter().find(|(known, _)| *known == extension).map_or(OTHER_CONTENT, |(_, content_type)| content_type)
}

/// The page `warp dev` shows before any build succeeded: only dev.js, which shows the failure
pub fn dev_shell(title: &str) -> Vec<SiteFile> {
	let page = page(title, "", "", &[DEV_SCRIPT], &[], BESIDE);
	vec![(PAGE_FILE.to_string(), page.into_bytes()), (DEV_SCRIPT.0.to_string(), DEV_SCRIPT.1.as_bytes().to_vec())]
}

/// The scripts of the page of a module, in load order: the parts of the host it imports words of, with the parts they
/// need, and markup-transitions.js when it names a transition; `dev` adds dev.js
pub fn scripts_of(module: &[u8], dev: bool) -> Result<Vec<Script>, String> {
	Ok(host_scripts_of(module)?.into_iter().chain(page_scripts_of(module)?).chain(dev.then_some(DEV_SCRIPT)).collect())
}

/// canvas.js when the module paints, markup.js, its transitions part when the module names a transition, and site.js
fn page_scripts_of(module: &[u8]) -> Result<Vec<Script>, String> {
	let [markup, site] = PAGE_SCRIPTS;
	let canvas = paints(&imports_of(module)?).then_some(PAINT_SCRIPT);
	let transitions = module.windows(TRANSITION_WORD.len()).any(|window| window == TRANSITION_WORD).then_some(TRANSITIONS_SCRIPT);
	Ok(canvas.into_iter().chain([markup]).chain(transitions).chain([site]).collect())
}

fn paints(imports: &[(String, String)]) -> bool {
	imports_any(imports, &[PAINT])
}

fn imports_any(imports: &[(String, String)], words: &[&str]) -> bool {
	imports.iter().any(|(module, name)| module == HOST_LIBRARY && words.contains(&name.as_str()))
}

/// A module that starts tasks, uses channels or shared memory runs in a Worker, where a blocking `await` may wait and
/// its tasks run together (card site-worker); a plain page stays on the page's thread. Routes go along: the page sends
/// the Worker the path of each link followed (site-thread.js). A module that paints runs there too (card site-frames):
/// an animation sleeps between its frames, which the page shows meanwhile, and reads the pointer at once; so does one
/// using the GPU, whose jobs run on the task Workers (card site-gpu)
fn runs_in_a_worker(imports: &[(String, String)]) -> bool {
	paints(imports) || imports_any(imports, &GPU_WORDS) || imports.iter().any(|(module, name)| module == HOST_LIBRARY && TASK_WORD_PREFIXES.iter().any(|prefix| name.starts_with(prefix)))
}

fn calls_foreign_code(module: &[u8]) -> Result<bool, String> {
	Ok(imports_of(module)?.iter().any(|(module, name)| module == HOST_LIBRARY && name == FOREIGN_CALL))
}

/// A page served at a deeper path than the site's files finds them, and the module and route modules they load,
/// through its <base> (`root`)
fn page(title: &str, body: &str, replies: &str, scripts: &[Script], worker_scripts: &[Script], root: &str) -> String {
	let scripts: String = scripts.iter().map(|(name, _)| format!("<script src=\"{name}\"></script>\n")).collect();
	let worker_list: Vec<&str> = worker_scripts.iter().map(|(name, _)| *name).collect();
	let worker = if worker_list.is_empty() { String::new() } else { format!(" {WORKER_ATTRIBUTE}=\"{}\"", worker_list.join(LIST_SEPARATOR)) };
	let base = if root == BESIDE { String::new() } else { format!("<base href=\"{root}\">\n") };
	// the program's texts last, so nothing in them is read as a placeholder
	PAGE_TEMPLATE.replace("{{root}}", ROOT_ID).replace("{{scripts}}", &scripts).replace("{{worker}}", &worker).replace("{{base}}", &base).replace("{{title}}", &escaped(title)).replace("{{replies}}", replies).replace("{{body}}", body)
}

/// A script without its comment lines, blank lines and indentation (card web-bundle: host.js gzipped 24 → 17 KB); the
/// lines of a template literal stay as written
pub fn compacted(script: &str) -> String {
	let mut in_template = false;
	let mut kept = String::new();
	for line in script.lines() {
		let shown = if in_template { line } else { line.trim_start() };
		if in_template || !(shown.is_empty() || shown.starts_with(LINE_COMMENT)) {
			kept.push_str(shown);
			kept.push('\n');
		}
		in_template ^= line.matches('`').count() % 2 == 1;
	}
	kept
}

/// The program file's text, a failure naming the file
pub(crate) fn read_program(program: &Path) -> Result<String, String> {
	std::fs::read_to_string(program).map_err(|failure| format!("cannot read {}: {failure}", program.display()))
}

/// app.warp → app, the name of its page and Worker
pub(crate) fn program_stem(program: &Path) -> String {
	program.file_stem().map(|stem| stem.to_string_lossy().into_owned()).unwrap_or_default()
}

pub(crate) fn escaped(text: &str) -> String {
	text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}
