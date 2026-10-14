//! A page in a window of the system's web view (WKWebView through wry), its files answered from memory under
//! warp://localhost/: `warp app` and `warp page.html` (src/web/webview.rs in warp), and an app `warp build` made, the stub
//! carrying a page's site (standalone.rs, card build-links). `check` loads the page in a hidden window and prints its
//! text instead, then ends (probes and tests: no window on the user's screen).

use std::borrow::Cow;
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
/// What the page posts when its program ran `exit(code)`, the code after it
const EXIT_MESSAGE: &str = "warp-exit ";
const DEFAULT_HEIGHT: f64 = 700.0;

/// The bytes a path of the page names and their content type, None when there is no such file
pub type Files = Box<dyn Fn(&str) -> Option<(Vec<u8>, &'static str)> + Send + Sync>;

/// The page whose files `files` answers in a window titled `title`
pub fn show(title: &str, check: bool, files: Files) -> Result<(), String> {
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
	let mut viewer =
		Viewer { title: title.to_string(), check, files: Some(files), window: None, webview: None, failure: None };
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
		let attributes = Window::default_attributes()
			.with_title(&self.title)
			.with_visible(!self.check)
			.with_inner_size(winit::dpi::LogicalSize::new(DEFAULT_WIDTH, DEFAULT_HEIGHT));
		let window = event_loop.create_window(attributes).map_err(|failure| failure.to_string())?;
		let files = self.files.take().expect("the window opens once");
		let mut builder = WebViewBuilder::new()
			.with_url(START_URL)
			.with_custom_protocol(PROTOCOL.to_string(), move |_, request| answer(&files, &request));
		// `exit` in a handler of the page's program (site.js endApp) ends the app with its code
		builder = builder.with_initialization_script(format!(
			"self.endApp = code => window.ipc.postMessage('{EXIT_MESSAGE}' + code)"
		));
		if self.check {
			builder = builder.with_initialization_script(format!(
				"addEventListener('load', () => setTimeout(() => window.ipc.postMessage(document.body.innerText), {CHECK_DELAY_MS}))"));
		}
		let check = self.check;
		builder = builder.with_ipc_handler(move |message: Request<String>| {
			if let Some(code) = message.body().strip_prefix(EXIT_MESSAGE) {
				std::process::exit(code.trim().parse().unwrap_or(0));
			}
			if check {
				println!("{}", message.body());
				std::process::exit(0);
			}
		});
		// a child of winit's view, never `build`: that swaps wry's view in as the window's content, which winit still
		// reads as its own (a crash when the window loses focus, card focus-crash)
		builder = builder.with_bounds(window_bounds(&window)).with_focused(true);
		let webview = builder.build_as_child(&window).map_err(|failure| failure.to_string())?;
		if self.check {
			lose_focus(&window);
		}
		self.webview = Some(webview);
		self.window = Some(window);
		Ok(())
	}
}

/// What AppKit sends when the user switches to another app (Cmd+Tab), sent directly so a hidden checked window goes
/// through it too: it crashed winit while wry's view had replaced winit's (card focus-crash)
fn window_bounds(window: &Window) -> wry::Rect {
	let size = window.inner_size().to_logical::<f64>(window.scale_factor());
	wry::Rect { position: winit::dpi::LogicalPosition::new(0.0, 0.0).into(), size: size.into() }
}

fn lose_focus(window: &Window) {
	use objc2::{msg_send, runtime::AnyObject};
	use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
	let Ok(handle) = window.window_handle() else { return };
	let RawWindowHandle::AppKit(appkit) = handle.as_raw() else {
		return;
	};
	// SAFETY: ns_view is the window's live NSView; window and delegate are AppKit's own getters
	unsafe {
		let view: &AnyObject = appkit.ns_view.cast().as_ref();
		let ns_window: *mut AnyObject = msg_send![view, window];
		let delegate: *mut AnyObject = msg_send![ns_window, delegate];
		let _: () = msg_send![delegate, windowDidResignKey: std::ptr::null::<AnyObject>()];
	}
}

fn answer(files: &Files, request: &Request<Vec<u8>>) -> Response<Cow<'static, [u8]>> {
	let found = files(request.uri().path());
	let status = if found.is_some() { StatusCode::OK } else { StatusCode::NOT_FOUND };
	let (bytes, content_type) = found.unwrap_or_default();
	Response::builder()
		.status(status)
		.header(CONTENT_TYPE, content_type)
		.body(Cow::Owned(bytes))
		.expect("a valid response")
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
		match event {
			WindowEvent::CloseRequested => event_loop.exit(),
			WindowEvent::Resized(_) => {
				if let (Some(window), Some(webview)) = (&self.window, &self.webview) {
					let _ = webview.set_bounds(window_bounds(window));
				}
			}
			_ => {}
		}
	}
}
