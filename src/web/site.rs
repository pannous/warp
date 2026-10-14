//! `warp build --site app.warp` (card web-ssr, notes/web_framework.md "Built sites"): a directory a static web server
//! serves as it is. index.html holds the HTML of what the program shows, rendered at build time by the program itself
//! (its export page·html, lowering/web/page_html.rs, run natively after main), so the page reads without JavaScript; app.wasm is the program, which the loader (web/playground/site.js)
//! runs in the page to hydrate it: the DOM stays, the handlers of its elements run, and what they change is shown anew.
//! The page loads the playground's own reader.js, host.js and markup.js, carried in the warp binary, and the parts of
//! host.js its module imports words of (HOST_PARTS, card web-bundle).

pub(crate) use crate::host::host_parts::{compile_failure, exports, imports_of, message_of, with_excerpt};
pub use crate::host::host_parts::{HostPart, Script, HOST_PARTS};
pub use crate::host::page_parts::scripts_of;
use crate::host::page_parts::{page, page_layout, PageLayout, BESIDE, DEV_SCRIPT, SITE_ROOT};
use crate::host::{FOREIGN_CALL, HOST_LIBRARY};
use crate::node::Node;
use std::path::{Path, PathBuf};
pub use warp_runtime::site_files::{content_type, file_at, SiteFile};
use warp_runtime::site_files::{PAGE_FILE, ROUTED_PAGE_FILE};

const MODULE_FILE: &str = "app.wasm";
const LINE_COMMENT: &str = "//";
/// `const KEY = "data-warp-key"; // a list item's element`: the statement ends before the comment
const TRAILING_COMMENT: &str = "; //";
/// The element holding the replies of a rendered page's server calls (host-tasks.js SERVER_REPLIES_ID)
const REPLIES_ID: &str = "warp-replies";

/// What `warp build --site` wrote
pub struct BuiltSite {
	pub directory: PathBuf,
	pub files: Vec<String>,
}

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
	site_files(code, title, dev)?
		.map(|site| site.files)
		.ok_or_else(|| format!("the program shows no page: it exports no {}", crate::page_html::PAGE_HTML))
}

/// The site a program serving its page serves (src/web/web_server.rs), none when its last line shows nothing
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
	imports: crate::run::wasm_reader::Imports,
	title: String,
	layout: PageLayout,
}

impl ServedSite {
	/// Whether no route of the page matches `path`, a path of no file (`route "*"` matches any: the single page answers
	/// every path, user 2026-10-07)
	pub fn matches_no_route_at(&self, path: &str) -> bool {
		file_at(&self.files, path).is_some_and(|(name, _)| name == ROUTED_PAGE_FILE)
			&& !self.page_routes.iter().any(|pattern| crate::routes::path_matches(pattern, path))
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
		let read = crate::host::with_page_path(path, || {
			crate::run::wasm_reader::read_exports_after_main(&self.bytes, self.imports, &names)
		});
		let failed =
			|failure| format!("the page of {path} failed: {}", message_of(&crate::wasm_emitter::failed_run(failure)));
		let [html, requests, values]: [Node; 3] = read.map_err(failed)?.try_into().expect("three exports");
		let replies = replies_script(&requests.as_items(), &values.as_items());
		Ok(self.layout.page(&self.title, page_html_text(&html)?, &replies, root))
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

/// The replies of the page's server calls as host-tasks.js reads them: each request's url and arguments with the reply
/// the server gives it (web_server.rs Answer: a text as text/plain, any other value as JSON)
fn replies_script(requests: &[Node], values: &[Node]) -> String {
	let replies: Vec<serde_json::Value> = requests
		.iter()
		.zip(values)
		.filter_map(|(request, value)| {
			let [url, arguments] = request.as_items().try_into().ok()?;
			let answer = crate::web::web_server::Answer::of(value);
			Some(serde_json::json!({
				"url": url.name(),
				"arguments": crate::modules::foreign::json_of(&arguments),
				"body": String::from_utf8_lossy(&answer.body),
				"text": answer.content_type.starts_with(crate::web::web_server::TEXT_REPLY_TYPE),
			}))
		})
		.collect();
	// `</` would end the script element early
	let json = serde_json::Value::Array(replies).to_string().replace("</", "<\\/");
	format!("<script type=\"application/json\" id=\"{REPLIES_ID}\">{json}</script>\n")
}

fn site_files(code: &str, title: &str, dev: bool) -> Result<Option<ServedSite>, String> {
	let compile = |build: &dyn Fn() -> Result<crate::pipeline::CompiledModule, Node>| {
		let module = crate::pipeline::for_a_page(|| if dev { crate::pipeline::for_dev(build) } else { build() });
		module.map_err(|value| compile_failure(code, &value))
	};
	let rendering = compile(&|| crate::pipeline::prerendering(|| crate::pipeline::compile(code)))?;
	if !exports(&rendering.bytes, crate::page_html::PAGE_HTML) {
		return Ok(None);
	}
	let imports = crate::run::wasm_reader::Imports {
		host: rendering.needs_host,
		wasi: rendering.needs_wasi,
		ffi: rendering.needs_ffi,
	};
	let read_after_main = |name: &str| {
		warp_runtime::system_signals::until_first_frame(|| {
			crate::run::wasm_reader::read_export_after_main(&rendering.bytes, imports, name)
		})
		.map_err(|failure| format!("the program failed at build time: {}", with_excerpt(code, failure.to_string())))
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
		true => read_after_main(crate::routes::PAGE_ROUTES)?.as_items().iter().map(Node::text_or_name).collect(),
		false => vec![],
	};
	// a page calling server functions ships without them, starting from the values they gave here (lowering/web/serve.rs)
	let asks_the_server = exports(&rendering.bytes, crate::serve::RPC_VALUES);
	let rendering_bytes = rendering.bytes.clone();
	let module = if asks_the_server {
		let values = read_after_main(crate::serve::RPC_VALUES)?.as_items();
		compile(&|| crate::pipeline::with_server_values(values.clone(), || crate::pipeline::compile(code)))?
	} else {
		rendering
	};
	let html = page_html_text(&rendered)?;
	let layout = page_layout(&module.bytes, dev)?;
	let page_file = |root: &str| layout.page(title, html, "", root).into_bytes();
	let routed = exports(&module.bytes, crate::routes::PAGE_ROUTES);
	// each route's own functions in a module the page loads when it shows the route; a dev page reloads whole anyway
	let split = if dev { None } else { crate::web::route_split::split_by_route(&module.bytes)? };
	let (primary, route_modules) = match split {
		Some(split) => (split.primary, split.routes),
		None => (module.bytes, vec![]),
	};
	let mut files = vec![(PAGE_FILE.to_string(), page_file(BESIDE)), (MODULE_FILE.to_string(), primary)];
	if routed {
		files.push((ROUTED_PAGE_FILE.to_string(), page_file(SITE_ROOT)));
	}
	files.extend(route_modules);
	files.extend(layout.shipped().into_iter().map(|(name, text)| (name.to_string(), compacted(text).into_bytes())));
	let renderer =
		asks_the_server.then(|| Renderer { bytes: rendering_bytes, imports, title: title.to_string(), layout });
	Ok(Some(ServedSite { files, renderer, page_routes }))
}

/// The page `warp dev` shows before any build succeeded: only dev.js, which shows the failure
pub fn dev_shell(title: &str) -> Vec<SiteFile> {
	let page = page(title, "", "", &[DEV_SCRIPT], &[], BESIDE);
	vec![(PAGE_FILE.to_string(), page.into_bytes()), (DEV_SCRIPT.0.to_string(), DEV_SCRIPT.1.as_bytes().to_vec())]
}

fn calls_foreign_code(module: &[u8]) -> Result<bool, String> {
	Ok(imports_of(module)?.iter().any(|(module, name)| module == HOST_LIBRARY && name == FOREIGN_CALL))
}

/// A script without its comment lines, trailing comments after a statement, blank lines and indentation (card
/// web-bundle: host.js gzipped 24 → 17 KB); the lines of a template literal stay as written
pub fn compacted(script: &str) -> String {
	let mut in_template = false;
	let mut kept = String::new();
	for line in script.lines() {
		let shown = if in_template { line } else { line.trim_start() };
		let shown = match shown.find(TRAILING_COMMENT) {
			Some(end) if !in_template && !shown[..end].contains('`') => &shown[..=end],
			_ => shown,
		};
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
