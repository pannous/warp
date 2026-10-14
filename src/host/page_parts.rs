//! The page of a program's site (src/web/site.rs) and of a deployed browser program (card deploy-browser-site,
//! host_parts.rs deployable): which scripts the page loads, which run in the program's Worker, and the page itself.
//! Like host_parts.rs it builds on wasm32 too, where the playground's Deploy only names the scripts for warp-hosting.

use crate::host::host_parts::{host_scripts_of, imports_of, script_text, Script, GPU_WORDS, TASK_WORD_PREFIXES};
use crate::host::{HOST_LIBRARY, PAINT};

/// The scripts every page loads after the host's (host_parts.rs HOST_SCRIPTS)
const PAGE_SCRIPTS: [Script; 2] = [("markup.js", script_text!("markup.js")), ("site.js", script_text!("site.js"))];
/// The part of markup.js for elements with a CSS transition, after it: only a module that names a transition has them
const TRANSITIONS_SCRIPT: Script = ("markup-transitions.js", script_text!("markup-transitions.js"));
/// A transition, or a leave animation (`leave: pop 400ms`, data-warp-leave), needs the transitions part
const TRANSITION_WORDS: [&[u8]; 2] = [b"transition", b"data-warp-leave"];
/// The part of markup.js for forms with `on submit` (card web-submit): only a module with such a handler has them
const FORMS_SCRIPT: Script = ("markup-forms.js", script_text!("markup-forms.js"));
const SUBMIT_HANDLER_WORD: &[u8] = "on·submit".as_bytes();
/// The canvas a module that paints shows its paintings and animation frames in, and the pointer over it (mouse_x, …)
const PAINT_SCRIPT: Script = ("canvas.js", script_text!("canvas.js"));
/// A page whose program runs in a Worker (card site-worker) loads this before them, and site.js hands over to it
const THREAD_SCRIPT: Script = ("site-thread.js", script_text!("site-thread.js"));
/// The page makes the task Workers of the program's Worker, which asks for them (Firefox, card tour-firefox-stall), when
/// the Worker has the task pool (TASK_PART: not a site that only paints)
const TASK_WORKERS_SCRIPT: Script = ("task-workers.js", script_text!("task-workers.js"));
const TASK_PART: &str = "host-tasks.js";
/// What such a site ships besides: the program's Worker, the task Workers it starts, and the service worker that gives a
/// static host's page the cross-origin isolation shared memory needs
const WORKER_FILES: [Script; 3] = [
	("site-worker.js", script_text!("site-worker.js")),
	("task-worker.js", script_text!("task-worker.js")),
	("coi-serviceworker.js", script_text!("coi-serviceworker.js")),
];
/// The root's attribute listing the scripts of the program's Worker (site-thread.js WORKER_ATTRIBUTE)
const WORKER_ATTRIBUTE: &str = "data-warp-worker";
const LIST_SEPARATOR: &str = ",";

/// The parts the page keeps when the program runs in a Worker: host-routes.js follows links and the back button and
/// moves the focus, which only the page can (site-thread.js)
const PAGE_SIDE_PARTS: [&str; 1] = ["host-routes.js"];
/// The element holding the program's markup (site.js SITE_ROOT)
const ROOT_ID: &str = "warp-root";
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

/// `warp dev` adds the script asking the dev server for new builds (src/web/dev_server.rs)
pub const DEV_SCRIPT: Script = ("dev.js", script_text!("dev.js"));

/// Where the page finds the site's files: index.html beside them, 404.html at any depth from the site's root
pub const BESIDE: &str = "";
pub const SITE_ROOT: &str = "/";

/// The scripts of a module's page: those it loads, and those of the program's Worker when it runs in one (else none)
pub struct PageLayout {
	pub scripts: Vec<Script>,
	pub worker_scripts: Vec<Script>,
}

impl PageLayout {
	/// The scripts the site ships: the page's, the Worker's, and the Worker files when it has one
	pub fn shipped(&self) -> Vec<Script> {
		let worker_files = if self.worker_scripts.is_empty() { &[][..] } else { &WORKER_FILES[..] };
		let worker_only = self.worker_scripts.iter().filter(|script| !self.scripts.contains(script));
		self.scripts.iter().chain(worker_only).chain(worker_files).copied().collect()
	}

	/// The page showing `body`, titled `title`, its files found at `root`
	pub fn page(&self, title: &str, body: &str, replies: &str, root: &str) -> String {
		page(title, body, replies, &self.scripts, &self.worker_scripts, root)
	}
}

/// The layout of a module's page; `dev` adds dev.js
pub fn page_layout(module: &[u8], dev: bool) -> Result<PageLayout, String> {
	let host_scripts = host_scripts_of(module)?;
	let page_scripts = page_scripts_of(module)?.into_iter().chain(dev.then_some(DEV_SCRIPT));
	Ok(if runs_in_a_worker(&imports_of(module)?) {
		let page_parts = host_scripts.iter().filter(|(name, _)| PAGE_SIDE_PARTS.contains(name)).copied();
		let task_workers = host_scripts.iter().any(|(name, _)| *name == TASK_PART).then_some(TASK_WORKERS_SCRIPT);
		let scripts = task_workers.into_iter().chain([THREAD_SCRIPT]).chain(page_parts).chain(page_scripts).collect();
		PageLayout { scripts, worker_scripts: host_scripts }
	} else {
		PageLayout { scripts: host_scripts.into_iter().chain(page_scripts).collect(), worker_scripts: vec![] }
	})
}

/// The scripts of the page of a module, in load order: the parts of the host it imports words of, with the parts they
/// need, and markup-transitions.js when it names a transition; `dev` adds dev.js
pub fn scripts_of(module: &[u8], dev: bool) -> Result<Vec<Script>, String> {
	Ok(host_scripts_of(module)?.into_iter().chain(page_scripts_of(module)?).chain(dev.then_some(DEV_SCRIPT)).collect())
}

/// canvas.js when the module paints, markup.js, its transitions part when the module names a transition, its forms part
/// when it has an `on submit` handler, and site.js
fn page_scripts_of(module: &[u8]) -> Result<Vec<Script>, String> {
	let [markup, site] = PAGE_SCRIPTS;
	let canvas = paints(&imports_of(module)?).then_some(PAINT_SCRIPT);
	let transitions = TRANSITION_WORDS.iter().any(|word| names(module, word)).then_some(TRANSITIONS_SCRIPT);
	let forms = names(module, SUBMIT_HANDLER_WORD).then_some(FORMS_SCRIPT);
	Ok(canvas.into_iter().chain([markup]).chain(transitions).chain(forms).chain([site]).collect())
}

fn names(module: &[u8], word: &[u8]) -> bool {
	module.windows(word.len()).any(|window| window == word)
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
	paints(imports)
		|| imports_any(imports, &GPU_WORDS)
		|| imports.iter().any(|(module, name)| {
			module == HOST_LIBRARY && TASK_WORD_PREFIXES.iter().any(|prefix| name.starts_with(prefix))
		})
}

/// A page served at a deeper path than the site's files finds them, and the module and route modules they load,
/// through its <base> (`root`)
pub fn page(
	title: &str,
	body: &str,
	replies: &str,
	scripts: &[Script],
	worker_scripts: &[Script],
	root: &str,
) -> String {
	let scripts: String = scripts.iter().map(|(name, _)| format!("<script src=\"{name}\"></script>\n")).collect();
	let worker_list: Vec<&str> = worker_scripts.iter().map(|(name, _)| *name).collect();
	let worker = if worker_list.is_empty() {
		String::new()
	} else {
		format!(" {WORKER_ATTRIBUTE}=\"{}\"", worker_list.join(LIST_SEPARATOR))
	};
	let base = if root == BESIDE { String::new() } else { format!("<base href=\"{root}\">\n") };
	// the program's texts last, so nothing in them is read as a placeholder
	PAGE_TEMPLATE
		.replace("{{root}}", ROOT_ID)
		.replace("{{scripts}}", &scripts)
		.replace("{{worker}}", &worker)
		.replace("{{base}}", &base)
		.replace("{{title}}", &crate::web::markup::escaped(title))
		.replace("{{replies}}", replies)
		.replace("{{body}}", body)
}
