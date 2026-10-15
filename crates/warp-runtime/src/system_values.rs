//! System values (notes/system_signals.md, P136): `battery` (percent), `charging`, `online` and `dark mode` read the
//! machine's state through the host word `system_value(name)`, an i64 (booleans 0 or 1). Listeners on them
//! (`whenever battery < 20% {…}`) compare readings at the program's check points, so a reading is kept for
//! READING_LIFETIME: a sleeping program polls often, the platform is asked at most once a second per value.
//! macOS asks `pmset` and `defaults`, Linux /sys and `gsettings`; `online` is whether a route to the internet exists
//! (a UDP connect sends nothing). A value the platform cannot give is a loud error, never a made-up reading.
//! `mouse_x`, `mouse_y`, `mouse_down`: natively the pointer over the paint window (window_input), read each time;
//! `key_down` whether a key is held over it, `key code held` which (lowering/signals/system_values.rs names it `held_key`);
//! `window_open` whether that window shows; read before any paint, it opens the window (blank) where one can show.
//! `window_width`, `window_height` (window.width, card make-width): the paint window's inner size in points as it reports
//! it, else as last written, else the view's; writing one resizes the window through the warp binary's resizer.
//! `display_width`, `display_height`: the main screen's size in points, read only.
//! The clipboard is watched by its change count, which reads no content: macOS asks the user before a program reads
//! what another one copied, so the text is read only when the program asks for it (`clipboard`, clipboard_text).
use crate::host_words::{
	BATTERY, CHARGING, CLICK_PRESSED, CLICK_X, CLICK_Y, CLIPBOARD_COUNT, DARK_MODE, DISPLAY_HEIGHT, DISPLAY_WIDTH,
	GPU_DRAWING, KEY_CODE_HELD, KEY_DOWN, KEY_PRESSED, MOUSE_DOWN, MOUSE_X, MOUSE_Y, ONLINE, SYSTEM_VALUES,
	TIME_OF_DAY, VIEW_HEIGHT, VIEW_WIDTH, WINDOW_HEIGHT, WINDOW_OPEN, WINDOW_WIDTH,
};
use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

const NOTIFICATION_TITLE: &str = "warp";
const READING_LIFETIME: Duration = Duration::from_secs(1);
/// view_width × view_height natively: the size of a picture before any window shows it (host.js VIEW_DEFAULTS)
const NATIVE_VIEW: (i64, i64) = (640, 480);
/// Any public address: connecting a UDP socket only asks the routing table
const INTERNET_ADDRESS: &str = "1.1.1.1:53";

/// The last input over the paint window (src/graphics/paint_window.rs follow_input), each an f32's bits: pointer x, y in the
/// painted frame's pixels, a button down (1), the key held; all 0 before a window
static WINDOW_INPUT: [AtomicU32; 4] = [const { AtomicU32::new(0) }; 4];

/// Whether the paint window shows: set as it gets its first frame, cleared when it closes (src/graphics/paint_window.rs)
static WINDOW_SHOWN: AtomicBool = AtomicBool::new(false);

pub fn tell_window_open(open: bool) {
	WINDOW_SHOWN.store(open, Ordering::Relaxed);
}

pub fn window_shows() -> bool {
	WINDOW_SHOWN.load(Ordering::Relaxed)
}

/// Opens the paint window before any frame: the warp binary's (src/graphics/paint.rs allow_windows), none headless
static WINDOW_OPENER: OnceLock<fn()> = OnceLock::new();
/// Whether window_open asked the opener: once a run, so a window the user closed stays closed (card window-loop)
static WINDOW_ASKED: AtomicBool = AtomicBool::new(false);

pub fn tell_window_opener(open: fn()) {
	let _ = WINDOW_OPENER.set(open);
}

/// `while window_open { … }` before any paint: the window opens as the loop asks, else the loop would never run
fn window_open() -> bool {
	if let Some(open) = WINDOW_OPENER.get() {
		if !window_shows() && !WINDOW_ASKED.swap(true, Ordering::Relaxed) {
			open();
		}
	}
	window_shows()
}

/// view_width × view_height before any window shows a picture
pub fn native_view() -> (i64, i64) {
	NATIVE_VIEW
}

/// The paint window's inner size in points: as it reported it, or as the program wrote it since; none before either
static WINDOW_SIZE: Mutex<Option<(i64, i64)>> = Mutex::new(None);
/// Resizes the paint window, opened or not yet: the warp binary's (src/graphics/paint.rs allow_windows), none headless
static WINDOW_RESIZER: OnceLock<fn(i64, i64)> = OnceLock::new();
const DISPLAY_READ_ONLY: &str = "a program cannot resize the display: set window.width or canvas.width instead";

pub fn tell_window_resizer(resize: fn(i64, i64)) {
	let _ = WINDOW_RESIZER.set(resize);
}

/// The window's size as it reports it (src/graphics/paint_window.rs follow_input)
pub fn tell_window_size(width: i64, height: i64) {
	*WINDOW_SIZE.lock().unwrap() = Some((width, height));
}

/// window.width, window.height: before any window, the size one opens at for the view
fn window_size() -> (i64, i64) {
	WINDOW_SIZE.lock().unwrap().unwrap_or(NATIVE_VIEW)
}

/// `set_system_value(name, value)`: window.width = 800 resizes the window; the others are read only
pub fn write(name: &str, value: i64) -> Result<(), String> {
	let (width, height) = match name {
		WINDOW_WIDTH => (value, window_size().1),
		WINDOW_HEIGHT => (window_size().0, value),
		DISPLAY_WIDTH | DISPLAY_HEIGHT => return Err(DISPLAY_READ_ONLY.to_string()),
		other => return Err(format!("{other} cannot be set; settable: {WINDOW_WIDTH}, {WINDOW_HEIGHT}")),
	};
	if width <= 0 || height <= 0 {
		return Err(format!("a window of {width}×{height} points cannot show: its sides must be positive"));
	}
	tell_window_size(width, height);
	if let Some(resize) = WINDOW_RESIZER.get() {
		resize(width, height);
	}
	Ok(())
}

/// The keys pressed over the paint window not read yet (KEY_PRESSED), oldest first; a program that never reads them
/// keeps the last KEYS_KEPT
static KEY_PRESSES: Mutex<VecDeque<i64>> = Mutex::new(VecDeque::new());
const KEYS_KEPT: usize = 64;
const KEY_INPUT: usize = 3;
const BUTTON_INPUT: usize = 2;
/// The clicks over the paint window not read yet (CLICK_PRESSED), each where the button went down, oldest first, the
/// last KEYS_KEPT; and the click read last, which CLICK_X and CLICK_Y tell
static CLICKS: Mutex<VecDeque<(f32, f32)>> = Mutex::new(VecDeque::new());
static CLICK_READ: Mutex<(f32, f32)> = Mutex::new((0.0, 0.0));

/// Whether `use draw` draws on the GPU: the warp binary decides at its start (src/graphics/paint.rs allow_windows)
static DRAWING_ON_GPU: AtomicBool = AtomicBool::new(false);

pub fn tell_gpu_drawing(on_gpu: bool) {
	DRAWING_ON_GPU.store(on_gpu, Ordering::Relaxed);
}

pub fn window_input() -> [f32; 4] {
	WINDOW_INPUT.each_ref().map(|value| f32::from_bits(value.load(Ordering::Relaxed)))
}

pub fn tell_window_input(input: [f32; 4]) {
	tell_window_key(input[KEY_INPUT]);
	// a click: the button down that was up
	if input[BUTTON_INPUT] != 0.0 && window_input()[BUTTON_INPUT] == 0.0 {
		kept_push(&mut CLICKS.lock().unwrap(), (input[0], input[1]));
	}
	for (value, number) in WINDOW_INPUT.iter().zip(input) {
		value.store(number.to_bits(), Ordering::Relaxed);
	}
}

/// A press queued, the oldest dropped beyond KEYS_KEPT: a program that never reads them keeps the last ones
fn kept_push<T>(presses: &mut VecDeque<T>, press: T) {
	if presses.len() == KEYS_KEPT {
		presses.pop_front();
	}
	presses.push_back(press);
}

/// Every key held over the paint window, in the order they went down (card allow-diagonals): the last is KEY_INPUT
static KEYS_HELD: Mutex<Vec<i64>> = Mutex::new(Vec::new());

pub fn tell_keys_held(codes: &[i64]) {
	*KEYS_HELD.lock().unwrap() = codes.to_vec();
}

/// Whether the key of that code is held now, among others (KEY_HELD); code 0: whether none is
pub fn key_held(code: i64) -> bool {
	let held = KEYS_HELD.lock().unwrap();
	match code {
		0 => window_input()[KEY_INPUT] == 0.0 && held.is_empty(),
		_ => window_input()[KEY_INPUT] as i64 == code || held.contains(&code),
	}
}

/// The key held over the paint window alone, the pointer left as it is
pub fn tell_window_key(key: f32) {
	// a press: a key held that was not held before (another key, or none)
	if key != 0.0 && key != window_input()[KEY_INPUT] {
		kept_push(&mut KEY_PRESSES.lock().unwrap(), key as i64);
	}
	WINDOW_INPUT[KEY_INPUT].store(key.to_bits(), Ordering::Relaxed);
}

thread_local! {
	static READINGS: RefCell<HashMap<String, (Instant, i64)>> = RefCell::new(HashMap::new());
}

/// The current value of a system value, read anew at most once per READING_LIFETIME
pub fn read(name: &str) -> Result<i64, String> {
	// the clock and the pointer move on: never a kept reading
	if name == TIME_OF_DAY {
		return Ok(crate::system_signals::millisecond_of_day());
	}
	if name == WINDOW_OPEN {
		return Ok(window_open() as i64);
	}
	if name == KEY_DOWN {
		return Ok((window_input()[KEY_INPUT] != 0.0) as i64);
	}
	if name == KEY_CODE_HELD {
		return Ok(window_input()[KEY_INPUT] as i64);
	}
	if name == CLICK_PRESSED {
		let click = CLICKS.lock().unwrap().pop_front();
		if let Some(click) = click {
			*CLICK_READ.lock().unwrap() = click;
		}
		return Ok(click.is_some() as i64);
	}
	if name == CLICK_X || name == CLICK_Y {
		let (x, y) = *CLICK_READ.lock().unwrap();
		return Ok(if name == CLICK_X { x } else { y } as i64);
	}
	if name == KEY_PRESSED {
		return Ok(KEY_PRESSES.lock().unwrap().pop_front().unwrap_or(0));
	}
	if name == WINDOW_WIDTH || name == WINDOW_HEIGHT {
		let (width, height) = window_size();
		return Ok(if name == WINDOW_WIDTH { width } else { height });
	}
	if name == GPU_DRAWING {
		return Ok(DRAWING_ON_GPU.load(Ordering::Relaxed) as i64);
	}
	if let Some(at) = [MOUSE_X, MOUSE_Y, MOUSE_DOWN].iter().position(|pointer| *pointer == name) {
		return Ok(window_input()[at] as i64);
	}
	let kept = READINGS.with(|readings| {
		readings.borrow().get(name).filter(|(when, _)| when.elapsed() < READING_LIFETIME).map(|(_, value)| *value)
	});
	if let Some(value) = kept {
		return Ok(value);
	}
	let value = read_now(name)?;
	READINGS.with(|readings| readings.borrow_mut().insert(name.to_string(), (Instant::now(), value)));
	Ok(value)
}

fn read_now(name: &str) -> Result<i64, String> {
	match name {
		BATTERY => battery().map(|(percent, _)| percent),
		CHARGING => battery().map(|(_, charging)| charging as i64),
		ONLINE => Ok(online() as i64),
		DARK_MODE => dark_mode().map(|dark| dark as i64),
		CLIPBOARD_COUNT => clipboard_count(),
		VIEW_WIDTH => Ok(NATIVE_VIEW.0),
		VIEW_HEIGHT => Ok(NATIVE_VIEW.1),
		DISPLAY_WIDTH => display_size().map(|(width, _)| width),
		DISPLAY_HEIGHT => display_size().map(|(_, height)| height),
		other => Err(format!("{other} is no system value; known: {}", SYSTEM_VALUES.map(|(name, _)| name).join(", "))),
	}
}

/// The main screen's size in points (CoreGraphics: a Retina screen's scaled mode, as windows measure)
#[cfg(target_os = "macos")]
fn display_size() -> Result<(i64, i64), String> {
	#[link(name = "CoreGraphics", kind = "framework")]
	unsafe extern "C" {
		fn CGMainDisplayID() -> u32;
		fn CGDisplayPixelsWide(display: u32) -> usize;
		fn CGDisplayPixelsHigh(display: u32) -> usize;
	}
	// SAFETY: plain queries of the main display, which always exists (a headless Mac reports 0)
	let (width, height) = unsafe {
		let display = CGMainDisplayID();
		(CGDisplayPixelsWide(display) as i64, CGDisplayPixelsHigh(display) as i64)
	};
	match width {
		0 => Err("display.width: this Mac has no display".to_string()),
		_ => Ok((width, height)),
	}
}

/// The screen's size as `xrandr` tells it: "Screen 0: minimum 8 x 8, current 1920 x 1080, maximum …"
#[cfg(not(target_os = "macos"))]
fn display_size() -> Result<(i64, i64), String> {
	let report = output("xrandr", &["--current"])?;
	let current = report.split("current ").nth(1).ok_or("display.width: no display (xrandr tells no screen)")?;
	let mut sides = current.split(',').next().unwrap_or_default().split(" x ").map(|side| side.trim().parse::<i64>());
	match (sides.next(), sides.next()) {
		(Some(Ok(width)), Some(Ok(height))) => Ok((width, height)),
		_ => Err(format!("display.width: xrandr's screen is unreadable: {current}")),
	}
}

fn online() -> bool {
	std::net::UdpSocket::bind("0.0.0.0:0").and_then(|socket| socket.connect(INTERNET_ADDRESS)).is_ok()
}

fn output(program: &str, arguments: &[&str]) -> Result<String, String> {
	let output = std::process::Command::new(program)
		.args(arguments)
		.output()
		.map_err(|problem| format!("{program}: {problem}"))?;
	Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// The battery's percent and whether it charges
#[cfg(target_os = "macos")]
fn battery() -> Result<(i64, bool), String> {
	// "-InternalBattery-0 (id=…)	87%; charging; 1:02 remaining present: true"
	let report = output("pmset", &["-g", "batt"])?;
	let line = report.lines().find(|line| line.contains('%')).ok_or("battery: this Mac has no battery")?;
	let percent = line
		.split('%')
		.next()
		.and_then(|before| before.rsplit(|letter: char| !letter.is_ascii_digit()).next())
		.and_then(|digits| digits.parse().ok());
	let charging = line.contains("; charging") || line.contains("; charged") || report.contains("'AC Power'");
	Ok((percent.ok_or_else(|| format!("battery: cannot read {line:?}"))?, charging))
}

#[cfg(target_os = "linux")]
fn battery() -> Result<(i64, bool), String> {
	let supply = std::fs::read_dir("/sys/class/power_supply")
		.map_err(|problem| format!("battery: {problem}"))?
		.flatten()
		.map(|entry| entry.path())
		.find(|path| path.join("capacity").exists())
		.ok_or("battery: this machine has no battery")?;
	let read = |file: &str| {
		std::fs::read_to_string(supply.join(file))
			.map(|text| text.trim().to_string())
			.map_err(|problem| format!("battery: {problem}"))
	};
	let percent = read("capacity")?.parse().map_err(|_| "battery: cannot read its capacity".to_string())?;
	Ok((percent, matches!(read("status")?.as_str(), "Charging" | "Full")))
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
fn battery() -> Result<(i64, bool), String> {
	Err("battery: not readable on this platform yet".to_string())
}

#[cfg(target_os = "macos")]
fn dark_mode() -> Result<bool, String> {
	// the key exists only in dark mode
	Ok(output("defaults", &["read", "-g", "AppleInterfaceStyle"])?.trim() == "Dark")
}

#[cfg(target_os = "linux")]
fn dark_mode() -> Result<bool, String> {
	Ok(output("gsettings", &["get", "org.gnome.desktop.interface", "color-scheme"])?.contains("dark"))
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
fn dark_mode() -> Result<bool, String> {
	Err("dark mode: not readable on this platform yet".to_string())
}

/// macOS: `[[NSPasteboard generalPasteboard] changeCount]`, AppKit loaded on first use (a plain run never loads it)
#[cfg(target_os = "macos")]
fn clipboard_count() -> Result<i64, String> {
	use std::ffi::{c_char, c_void, CStr};
	type Object = *mut c_void;
	const APPKIT: &CStr = c"/System/Library/Frameworks/AppKit.framework/AppKit";
	let symbol = |name: &CStr| -> Result<*mut c_void, String> {
		let found = unsafe { libc::dlsym(libc::RTLD_DEFAULT, name.as_ptr()) };
		if found.is_null() {
			Err(format!("clipboard: no {name:?}"))
		} else {
			Ok(found)
		}
	};
	if unsafe { libc::dlopen(APPKIT.as_ptr(), libc::RTLD_LAZY) }.is_null() {
		return Err("clipboard: cannot load AppKit".to_string());
	}
	// objc_msgSend is called through the exact signature of each message
	let class: extern "C" fn(*const c_char) -> Object = unsafe { std::mem::transmute(symbol(c"objc_getClass")?) };
	let selector: extern "C" fn(*const c_char) -> Object = unsafe { std::mem::transmute(symbol(c"sel_registerName")?) };
	let send = symbol(c"objc_msgSend")?;
	let send_for_object: extern "C" fn(Object, Object) -> Object = unsafe { std::mem::transmute(send) };
	let send_for_count: extern "C" fn(Object, Object) -> isize = unsafe { std::mem::transmute(send) };
	let pasteboard = send_for_object(class(c"NSPasteboard".as_ptr()), selector(c"generalPasteboard".as_ptr()));
	if pasteboard.is_null() {
		return Err("clipboard: no pasteboard".to_string());
	}
	Ok(send_for_count(pasteboard, selector(c"changeCount".as_ptr())) as i64)
}

/// Elsewhere the content's hash stands for the count: no prompt guards it there
#[cfg(not(target_os = "macos"))]
fn clipboard_count() -> Result<i64, String> {
	use std::hash::{Hash, Hasher};
	let mut hasher = std::collections::hash_map::DefaultHasher::new();
	clipboard_text()?.hash(&mut hasher);
	Ok(hasher.finish() as i64)
}

/// `notify "text"`: a desktop notification, macOS through osascript, Linux notify-send; the text goes as an argument,
/// so no quoting can break it
pub fn notify(text: &str) -> Result<(), String> {
	#[cfg(target_os = "macos")]
	let shown = std::process::Command::new("osascript")
		.args([
			"-e",
			"on run argv",
			"-e",
			&format!("display notification (item 1 of argv) with title \"{NOTIFICATION_TITLE}\""),
			"-e",
			"end run",
			text,
		])
		.output();
	#[cfg(not(target_os = "macos"))]
	let shown = std::process::Command::new("notify-send").args([NOTIFICATION_TITLE, text]).output();
	match shown {
		Ok(output) if output.status.success() => Ok(()),
		Ok(output) => Err(format!("notify: {}", String::from_utf8_lossy(&output.stderr).trim())),
		Err(problem) => Err(format!("notify: no notifier on this machine ({problem})")),
	}
}

/// Puts text on the clipboard: macOS `pbcopy`, Linux `wl-copy` or `xclip`
pub fn write_clipboard(text: &str) -> Result<(), String> {
	#[cfg(target_os = "macos")]
	return input("pbcopy", &[], text);
	#[cfg(not(target_os = "macos"))]
	input("wl-copy", &[], text)
		.or_else(|_| input("xclip", &["-selection", "clipboard", "-i"], text))
		.map_err(|problem| format!("clipboard: needs wl-copy or xclip ({problem})"))
}

/// Runs a program with `text` as its input
fn input(program: &str, arguments: &[&str], text: &str) -> Result<(), String> {
	use std::io::Write;
	let mut child = std::process::Command::new(program)
		.args(arguments)
		.stdin(std::process::Stdio::piped())
		.spawn()
		.map_err(|problem| format!("{program}: {problem}"))?;
	child
		.stdin
		.take()
		.expect("a piped input")
		.write_all(text.as_bytes())
		.map_err(|problem| format!("{program}: {problem}"))?;
	let status = child.wait().map_err(|problem| format!("{program}: {problem}"))?;
	status.success().then_some(()).ok_or_else(|| format!("{program} failed: {status}"))
}

/// The clipboard's text: macOS `pbpaste`, Linux `wl-paste` or `xclip`
pub fn clipboard_text() -> Result<String, String> {
	#[cfg(target_os = "macos")]
	return output("pbpaste", &[]);
	#[cfg(not(target_os = "macos"))]
	output("wl-paste", &["--no-newline"])
		.or_else(|_| output("xclip", &["-selection", "clipboard", "-o"]))
		.map_err(|problem| format!("clipboard: needs wl-paste or xclip ({problem})"))
}
