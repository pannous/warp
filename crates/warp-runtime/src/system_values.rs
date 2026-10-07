//! System values (notes/system_signals.md, P136): `battery` (percent), `charging`, `online` and `dark mode` read the
//! machine's state through the host word `system_value(name)`, an i64 (booleans 0 or 1). Listeners on them
//! (`whenever battery < 20% {…}`) compare readings at the program's check points, so a reading is kept for
//! READING_LIFETIME: a sleeping program polls often, the platform is asked at most once a second per value.
//! macOS asks `pmset` and `defaults`, Linux /sys and `gsettings`; `online` is whether a route to the internet exists
//! (a UDP connect sends nothing). A value the platform cannot give is a loud error, never a made-up reading.
//! The clipboard is watched by its change count, which reads no content: macOS asks the user before a program reads
//! what another one copied, so the text is read only when the program asks for it (`clipboard`, clipboard_text).
use std::cell::RefCell;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use crate::host_words::{BATTERY, CHARGING, CLIPBOARD_COUNT, DARK_MODE, MOUSE_DOWN, MOUSE_X, MOUSE_Y, ONLINE, SYSTEM_VALUES};

const READING_LIFETIME: Duration = Duration::from_secs(1);
/// Any public address: connecting a UDP socket only asks the routing table
const INTERNET_ADDRESS: &str = "1.1.1.1:53";

thread_local! {
	static READINGS: RefCell<HashMap<String, (Instant, i64)>> = RefCell::new(HashMap::new());
}

/// The current value of a system value, read anew at most once per READING_LIFETIME
pub fn read(name: &str) -> Result<i64, String> {
	let kept = READINGS.with(|readings| readings.borrow().get(name).filter(|(when, _)| when.elapsed() < READING_LIFETIME).map(|(_, value)| *value));
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
		MOUSE_X | MOUSE_Y | MOUSE_DOWN => Err(format!("{name}: the pointer over the playground's canvas; a native run has no canvas")),
		other => Err(format!("{other} is no system value; known: {}", SYSTEM_VALUES.map(|(name, _)| name).join(", "))),
	}
}

fn online() -> bool {
	std::net::UdpSocket::bind("0.0.0.0:0").and_then(|socket| socket.connect(INTERNET_ADDRESS)).is_ok()
}

fn output(program: &str, arguments: &[&str]) -> Result<String, String> {
	let output = std::process::Command::new(program).args(arguments).output().map_err(|problem| format!("{program}: {problem}"))?;
	Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// The battery's percent and whether it charges
#[cfg(target_os = "macos")]
fn battery() -> Result<(i64, bool), String> {
	// "-InternalBattery-0 (id=…)	87%; charging; 1:02 remaining present: true"
	let report = output("pmset", &["-g", "batt"])?;
	let line = report.lines().find(|line| line.contains('%')).ok_or("battery: this Mac has no battery")?;
	let percent = line.split('%').next().and_then(|before| before.rsplit(|letter: char| !letter.is_ascii_digit()).next()).and_then(|digits| digits.parse().ok());
	let charging = line.contains("; charging") || line.contains("; charged") || report.contains("'AC Power'");
	Ok((percent.ok_or_else(|| format!("battery: cannot read {line:?}"))?, charging))
}

#[cfg(target_os = "linux")]
fn battery() -> Result<(i64, bool), String> {
	let supply = std::fs::read_dir("/sys/class/power_supply").map_err(|problem| format!("battery: {problem}"))?
		.flatten().map(|entry| entry.path()).find(|path| path.join("capacity").exists()).ok_or("battery: this machine has no battery")?;
	let read = |file: &str| std::fs::read_to_string(supply.join(file)).map(|text| text.trim().to_string()).map_err(|problem| format!("battery: {problem}"));
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
		if found.is_null() { Err(format!("clipboard: no {name:?}")) } else { Ok(found) }
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

/// The clipboard's text: macOS `pbpaste`, Linux `wl-paste` or `xclip`
pub fn clipboard_text() -> Result<String, String> {
	#[cfg(target_os = "macos")]
	return output("pbpaste", &[]);
	#[cfg(not(target_os = "macos"))]
	output("wl-paste", &["--no-newline"]).or_else(|_| output("xclip", &["-selection", "clipboard", "-o"]))
		.map_err(|problem| format!("clipboard: needs wl-paste or xclip ({problem})"))
}
