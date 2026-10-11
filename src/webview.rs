//! `warp app page.warp` and `warp page.html` (card read-chron): the page in a window of the system's web view
//! (WKWebView through wry), what `warp build --site` writes served from memory under warp://localhost/, an HTML file
//! with the files beside it. `--check` loads the page in a hidden window and prints its text instead, then ends
//! (probes and tests: no window on the user's screen).

use crate::site::SiteFile;
use std::borrow::Cow;
use std::path::{Path, PathBuf};
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowId};
use wry::http::{header::CONTENT_TYPE, Request, Response, StatusCode};
use wry::{WebView, WebViewBuilder};

const PROTOCOL: &str = "warp";
const START_URL: &str = "warp://localhost/";
/// How long the checked page runs after it loaded before its text is read: its program hydrates it meanwhile
const CHECK_DELAY_MS: u32 = 500;
/// A checked page that never reports its text ends the check as a failure after this long
const CHECK_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(20);
const DEFAULT_WIDTH: f64 = 900.0;
const DEFAULT_HEIGHT: f64 = 700.0;

/// The bytes a path of the page names and their content type, None when there is no such file
type Files = Box<dyn Fn(&str) -> Option<(Vec<u8>, &'static str)> + Send + Sync>;

/// The page of the program `code` (its site, rendered and hydrated) in a window titled `title`
pub fn show_program(code: &str, title: &str, check: bool) -> Result<(), String> {
	let files = crate::site::files(code, title, false)?;
	show(title, check, Box::new(move |path| site_file(&files, path)))
}

/// The HTML file in a window, the files beside it at their relative paths
pub fn show_file(page: &Path, check: bool) -> Result<(), String> {
	let page = page.canonicalize().map_err(|failure| format!("cannot open {}: {failure}", page.display()))?;
	let folder = page.parent().map(Path::to_path_buf).unwrap_or_default();
	let title = page.file_stem().map(|stem| stem.to_string_lossy().into_owned()).unwrap_or_default();
	show(&title, check, Box::new(move |path| folder_file(&page, &folder, path)))
}

fn site_file(files: &[SiteFile], path: &str) -> Option<(Vec<u8>, &'static str)> {
	crate::site::file_at(files, path).map(|(name, bytes)| (bytes.clone(), crate::site::content_type(name)))
}

/// "/" is the page, any other path a file inside its folder (never outside: `..` is refused by the containment check)
fn folder_file(page: &Path, folder: &Path, path: &str) -> Option<(Vec<u8>, &'static str)> {
	let file: PathBuf = if path == "/" { page.to_path_buf() } else { folder.join(path.trim_start_matches('/')).canonicalize().ok()? };
	file.starts_with(folder).then_some(())?;
	let name = file.to_string_lossy().into_owned();
	std::fs::read(&file).ok().map(|bytes| (bytes, crate::site::content_type(&name)))
}

fn show(title: &str, check: bool, files: Files) -> Result<(), String> {
	use winit::platform::macos::{ActivationPolicy, EventLoopBuilderExtMacOS};
	let mut builder = EventLoop::builder();
	if check {
		// no Dock icon and no focus taken from the user's app (tests run beside them)
		builder.with_activation_policy(ActivationPolicy::Prohibited).with_activate_ignoring_other_apps(false);
	}
	if check {
		std::thread::spawn(|| {
			std::thread::sleep(CHECK_TIMEOUT);
			eprintln!("the page did not load within {CHECK_TIMEOUT:?}");
			std::process::exit(1);
		});
	}
	let event_loop = builder.build().map_err(|failure| format!("no window: {failure}"))?;
	let mut viewer = Viewer { title: title.to_string(), check, files: Some(files), window: None, webview: None, failure: None };
	event_loop.run_app(&mut viewer).map_err(|failure| failure.to_string())?;
	viewer.failure.map_or(Ok(()), Err)
}

struct Viewer {
	title: String,
	check: bool,
	files: Option<Files>,
	window: Option<Window>,
	webview: Option<WebView>,
	failure: Option<String>,
}

impl Viewer {
	fn open(&mut self, event_loop: &ActiveEventLoop) -> Result<(), String> {
		let attributes = Window::default_attributes().with_title(&self.title).with_visible(!self.check)
			.with_inner_size(winit::dpi::LogicalSize::new(DEFAULT_WIDTH, DEFAULT_HEIGHT));
		let window = event_loop.create_window(attributes).map_err(|failure| failure.to_string())?;
		let files = self.files.take().expect("the window opens once");
		let mut builder = WebViewBuilder::new().with_url(START_URL)
			.with_custom_protocol(PROTOCOL.to_string(), move |_, request| answer(&files, &request));
		if self.check {
			builder = builder.with_initialization_script(format!(
				"addEventListener('load', () => setTimeout(() => window.ipc.postMessage(document.body.innerText), {CHECK_DELAY_MS}))"))
				.with_ipc_handler(|text: Request<String>| {
					println!("{}", text.body());
					std::process::exit(0);
				});
		}
		self.webview = Some(builder.build(&window).map_err(|failure| failure.to_string())?);
		self.window = Some(window);
		Ok(())
	}
}

fn answer(files: &Files, request: &Request<Vec<u8>>) -> Response<Cow<'static, [u8]>> {
	let found = files(request.uri().path());
	let status = if found.is_some() { StatusCode::OK } else { StatusCode::NOT_FOUND };
	let (bytes, content_type) = found.unwrap_or_default();
	Response::builder().status(status).header(CONTENT_TYPE, content_type).body(Cow::Owned(bytes)).expect("a valid response")
}

impl ApplicationHandler for Viewer {
	fn resumed(&mut self, event_loop: &ActiveEventLoop) {
		if self.window.is_some() {
			return;
		}
		if let Err(failure) = self.open(event_loop) {
			self.failure = Some(failure);
			event_loop.exit();
		}
	}

	fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
		if matches!(event, WindowEvent::CloseRequested) {
			event_loop.exit();
		}
	}
}
