//! The words of the program's environment, imported from the "host" module and called like C functions:
//! `sleep(ms)` pauses, `random()` is a float in [0, 1), `random_below(n)` an int in 0..n, `clock()` the milliseconds
//! since the Unix epoch. A program that defines a word of the same name keeps its own. warp adds the words that need
//! its compiler, the network or threads (host.rs); these need nothing.
//! `signal_poll()` and `signal_listen(kind, id, argument)` serve the handlers of system signals (system_signals.rs).
pub const HOST_LIBRARY: &str = "host";
/// foreign_call(runtime, module, member, call, arguments): a module of another runtime (foreign.rs), Nodes in and out
pub const FOREIGN_CALL: &str = "foreign_call";
pub const SLEEP: &str = "sleep";
pub const RANDOM: &str = "random";
pub const RANDOM_BELOW: &str = "random_below";
/// `random_seed(n)`: random and random_below give the same numbers after the same seed (natively and in the browser)
pub const RANDOM_SEED: &str = "random_seed";
pub const CLOCK: &str = "clock";
/// `local_offset(ms)`: seconds east of UTC in the environment's time zone (TZ) at that instant: an instant's wall clock
pub const LOCAL_OFFSET: &str = "local_offset";
pub const SIGNAL_POLL: &str = "signal_poll";
/// `signal_listen(kind, id, argument)`: starts the timer on·every·id, `on every 5 seconds {…}` (LISTEN_EVERY, ms),
/// `on frame {…}` (LISTEN_FRAME, ms; every frame of the paint window while it shows, without a window once),
/// `on every monday at 9:00 {…}` and `at 9:00 {…}` (LISTEN_CLOCK, weekdays · CLOCK_WEEKDAYS + minute of day, the
/// weekdays a mask with bit 0 Sunday … bit 6 Saturday, 0 once at the next such local time)
pub const SIGNAL_LISTEN: &str = "signal_listen";
pub const LISTEN_EVERY: i64 = 0;
pub const LISTEN_FRAME: i64 = 1;
pub const LISTEN_CLOCK: i64 = 2;
/// The clock argument's weekdays count in ten-thousands: `(signal_listen 2 0 1271290)` is 21:30 (1290) every day (127)
pub const CLOCK_WEEKDAYS: i64 = 10_000;
pub const SIGNAL_WATCH: &str = "signal_watch";
/// `system_value(name)`: the machine's `battery` percent, `charging`, `online`, `dark mode` (system_values.rs)
pub const SYSTEM_VALUE: &str = "system_value";
/// `set_system_value(name, value)`: writes one, `window.width = 800` resizes the paint window (system_values.rs write)
pub const SET_SYSTEM_VALUE: &str = "set_system_value";
pub const BATTERY: &str = "battery";
pub const CHARGING: &str = "charging";
pub const ONLINE: &str = "online";
pub const DARK_MODE: &str = "dark mode";
/// How often the clipboard changed since the machine started (macOS NSPasteboard changeCount): `on clipboard change`
pub const CLIPBOARD_COUNT: &str = "clipboard count";
/// The pointer over the page's canvas, in canvas pixels, and whether a button is down (card drawing-frames)
pub const MOUSE_X: &str = "mouse_x";
pub const MOUSE_Y: &str = "mouse_y";
pub const MOUSE_DOWN: &str = "mouse_down";
/// Whether a key is held now, over the paint window or the page (card keyboard-state): `on key` hears each press
pub const KEY_DOWN: &str = "key_down";
/// The name of the key held now ("a", "ArrowUp", "" for none), as `on key` names presses (card held-key): the word
/// a program reads, made from KEY_CODE_HELD, the held key's code (0: none), which it does not name
pub const HELD_KEY: &str = "held_key";
pub const KEY_CODE_HELD: &str = "key code held";
/// `key_held(code)`: whether that key is held now, among others (card allow-diagonals: `held_key == LEFT` while UP is
/// held too); code 0 whether none is
pub const KEY_HELD: &str = "key_held";
/// Whether a window shows the pictures: natively the paint window until it closes (no before the first paint and
/// headless), on a page its canvas, always; `while window_open { … }` is a drawing loop that ends with its window
pub const WINDOW_OPEN: &str = "window_open";
/// The local time of day in milliseconds since midnight, what the word `time` is compared with a duration (src/numbers/units.rs)
pub const TIME_OF_DAY: &str = "time of day";
/// the size a picture gets: the playground's output pane, natively the paint window's first size (system_values.rs)
pub const VIEW_WIDTH: &str = "view_width";
pub const VIEW_HEIGHT: &str = "view_height";
/// Whether `use draw` draws on the GPU (src/graphics/gpu_canvas.rs): natively in a window, or as WARP_DRAW says; never on a page
pub const GPU_DRAWING: &str = "gpu_drawing";
/// window.width/height (card make-width): the paint window's inner size in points (the playground: the paintings' frame
/// in CSS pixels), writable; display.width/height the screen's, read only (lowering/words/module_properties.rs)
pub const WINDOW_WIDTH: &str = "window_width";
pub const WINDOW_HEIGHT: &str = "window_height";
pub const DISPLAY_WIDTH: &str = "display_width";
pub const DISPLAY_HEIGHT: &str = "display_height";
/// display.dpi: the screen's device pixels per inch, which turn `px` into lengths (decision dimensions-units)
pub const DISPLAY_DPI: &str = "display_dpi";
/// The next key pressed over the paint window, its code (0: none left), each press read once: natively `on key {…}`
/// (lowering/signals/system_signals.rs); not a value a program names
pub const KEY_PRESSED: &str = "key_pressed";
/// Whether a shader can paint here (1) or not (0): `paint(f, w, h)` of a warp function reads it before painting f's
/// fragment shader, else paints f on the CPU (card paint-function, src/lowering/foreign/gpu_maps/kernels.rs); not a value
/// a program names
pub const GPU_PAINTS: &str = "gpu paints";
/// Read once as a program naming the Escape key starts (`key == ESC`): Escape over the paint window is then its key,
/// else it ends the run (card escape-window, src/graphics/paint_window.rs); not a value a program names
pub const ESCAPE_KEPT: &str = "escape kept";
/// Whether a click over the paint window is left to read (1, then it is the click read) or none (0), each click read
/// once, and that click's place: natively `on click {…}` (lowering/signals/system_signals.rs); not values a program names
pub const CLICK_PRESSED: &str = "click_pressed";
pub const CLICK_X: &str = "click_x";
pub const CLICK_Y: &str = "click_y";
/// The system values, and whether each is a yes/no value
pub const SYSTEM_VALUES: [(&str, bool); 18] = [
	(BATTERY, false),
	(CHARGING, true),
	(ONLINE, true),
	(DARK_MODE, true),
	(CLIPBOARD_COUNT, false),
	(MOUSE_X, false),
	(MOUSE_Y, false),
	(MOUSE_DOWN, true),
	(KEY_DOWN, true),
	(WINDOW_OPEN, true),
	(VIEW_WIDTH, false),
	(VIEW_HEIGHT, false),
	(GPU_DRAWING, true),
	(WINDOW_WIDTH, false),
	(WINDOW_HEIGHT, false),
	(DISPLAY_WIDTH, false),
	(DISPLAY_HEIGHT, false),
	(DISPLAY_DPI, false),
];
/// `clipboard`: the clipboard's text, read only when the program reads it (host.rs clipboard_text)
pub const CLIPBOARD: &str = "clipboard";
pub const CLIPBOARD_TEXT: &str = "clipboard_text";
/// The path of the page shown ("/users/7"), which picks its route (lowering/web/routes.rs); "/" natively
pub const PAGE_PATH: &str = "page_path";
/// `notify "text"`: a desktop notification (system_values.rs notify; the playground: the browser's Notification)
pub const NOTIFY: &str = "notify";
/// `gpu_compute(shader, numbers, workgroups)`: a WGSL compute shader run over the numbers on the GPU (WebGPU in the
/// browser, web/playground/host-gpu.js), the numbers it left (card web-apis, notes/web_framework.md "web-apis: WebGPU")
pub const GPU_COMPUTE: &str = "gpu_compute";
/// `gpu_render(shader, width, height)`: a WGSL fragment shader `main` run over every pixel, the pixels paint shows
/// (0xFFRRGGBB row by row; card g_YqWY, notes/web_framework.md "web-apis: WebGPU")
pub const GPU_RENDER: &str = "gpu_render";
/// `text_coverage(words, size)`: the words set in a sans-serif font `size` pixels high (card paint-text, lib/draw.warp
/// label): [width, height, then how much each pixel is covered, 0 to 255, row by row]; the browser's canvas 2D fillText
/// in the playground, a system font through ab_glyph natively (src/graphics/text_raster.rs)
pub const TEXT_COVERAGE: &str = "text_coverage";
/// `gpu_compute(shader, xs, workgroups)` of a `linear xs = float[n]`: the shader runs over the cells of the block in linear
/// memory, read and written in place without building a list (card gpu-vectors, notes/gpu.md); gives the block back
pub const GPU_COMPUTE_LINEAR: &str = "gpu_compute_linear";
/// `ys = xs.map(x => …) @gpu` of a linear float array (src/lowering/foreign/gpu_maps.rs): the WGSL kernel run over xs's cells,
/// written into ys's; gives 1, or 0 without an adapter (said once), when the CPU maps them instead
pub const GPU_MAP_LINEAR: &str = "gpu_map_linear";
/// `s = sum(xs.map(x => …) @gpu)`, min, max (src/lowering/foreign/gpu_maps.rs): the kernel leaves one partial result per
/// workgroup, written into a block the program combines; 1, or 0 without an adapter, as gpu_map_linear
pub const GPU_REDUCE_LINEAR: &str = "gpu_reduce_linear";
/// What every exported handler name starts with
pub const HANDLER_PREFIX: &str = "on·";
/// `on·file·0`: the handler of the first `on file "x" change {…}`
pub const FILE_HANDLER_PREFIX: &str = "on·file·";
/// `exit(code)` ends the run, not the process (P121, system_signals.rs ExitRequest)
pub const EXIT: &str = "exit";
/// The exported handler of `on interrupt {…}` (src/lowering/signals/event_signals.rs)
pub const INTERRUPT_HANDLER: &str = "on·interrupt";
/// The exported handler of `on exit {…}`, run once as the run ends (system_signals.rs run_exit_handler)
pub const EXIT_HANDLER: &str = "on·exit";
/// `on·every·0`: the handler of the first `on every … {…}`
pub const TIMER_HANDLER_PREFIX: &str = "on·every·";
/// `on·fetch·0`: the handler run when the reply of the first `users := fetch url` arrived (src/web/fetches.rs)
pub const FETCH_HANDLER_PREFIX: &str = "on·fetch·";
/// The checks of the listeners on shared values (P106), polled at every check point (lowering/signals/signal_values.rs)
pub const SHARED_HANDLER: &str = "on·shared";
/// The words link_host_words provides
pub const BASIC_HOST_WORDS: [&str; 13] = [
	SLEEP,
	RANDOM,
	RANDOM_BELOW,
	RANDOM_SEED,
	CLOCK,
	LOCAL_OFFSET,
	SIGNAL_POLL,
	SIGNAL_LISTEN,
	SIGNAL_WATCH,
	SYSTEM_VALUE,
	SET_SYSTEM_VALUE,
	KEY_HELD,
	EXIT,
];

#[cfg(feature = "engine")]
pub use linking::*;

#[cfg(feature = "engine")]
mod linking {
	use super::*;
	use crate::system_signals;
	use std::time::Duration;
	use wasmtime::{Caller, Func, Linker, Result};

	/// Link sleep, random, random_below and clock, for a store of any state
	pub fn link_host_words<T: 'static>(linker: &mut Linker<T>) -> Result<()> {
		system_signals::forget_timers(); // each run links anew, on its own thread
		RANDOM_STATE.set(0); // and starts unseeded
		linker.func_wrap(HOST_LIBRARY, SLEEP, |mut caller: Caller<'_, T>, milliseconds: i64| {
			system_signals::end_at_first_frame()?;
			system_signals::sleep_with_handlers(
				&mut caller,
				Duration::from_millis(milliseconds.max(0) as u64),
				exported,
			)
		})?;
		// the top 53 bits make a uniform f64 in [0, 1)
		linker.func_wrap(HOST_LIBRARY, RANDOM, || (next_random() >> 11) as f64 / (1u64 << 53) as f64)?;
		linker.func_wrap(HOST_LIBRARY, RANDOM_BELOW, |bound: i64| {
			if bound <= 0 {
				0
			} else {
				(next_random() % bound as u64) as i64
			}
		})?;
		linker.func_wrap(HOST_LIBRARY, RANDOM_SEED, |seed: i64| RANDOM_STATE.set(seeded_state(seed as u64)))?;
		linker.func_wrap(HOST_LIBRARY, CLOCK, milliseconds_since_epoch)?;
		linker.func_wrap(HOST_LIBRARY, LOCAL_OFFSET, system_signals::local_offset_at)?;
		linker.func_wrap(HOST_LIBRARY, SIGNAL_POLL, |mut caller: Caller<'_, T>| {
			system_signals::run_due_handlers(&mut caller, exported)
		})?;
		linker.func_wrap(HOST_LIBRARY, SIGNAL_LISTEN, system_signals::listen)?;
		linker.func_wrap(
			HOST_LIBRARY,
			SIGNAL_WATCH,
			|mut caller: Caller<'_, T>, id: i64, path: i32| -> Result<()> {
				let path = c_string(&mut caller, path)?;
				system_signals::watch_file(id, path);
				Ok(())
			},
		)?;
		linker.func_wrap(HOST_LIBRARY, SYSTEM_VALUE, |mut caller: Caller<'_, T>, name: i32| -> Result<i64> {
			let name = c_string(&mut caller, name)?;
			crate::system_values::read(&name).map_err(wasmtime::Error::msg)
		})?;
		linker.func_wrap(
			HOST_LIBRARY,
			SET_SYSTEM_VALUE,
			|mut caller: Caller<'_, T>, name: i32, value: i64| -> Result<()> {
				let name = c_string(&mut caller, name)?;
				crate::system_values::write(&name, value).map_err(wasmtime::Error::msg)
			},
		)?;
		linker.func_wrap(HOST_LIBRARY, KEY_HELD, |code: i64| crate::system_values::key_held(code) as i64)?;
		linker.func_wrap(HOST_LIBRARY, EXIT, |code: i64| -> Result<()> {
			Err(wasmtime::Error::new(system_signals::ExitRequest(code as i32)))
		})?;
		Ok(())
	}

	/// The NUL-terminated text a text literal passes as an i32 (the emitter's C strings)
	fn c_string<T>(caller: &mut Caller<'_, T>, pointer: i32) -> Result<String> {
		let memory = caller
			.get_export("memory")
			.and_then(|export| export.into_memory())
			.ok_or_else(|| wasmtime::Error::msg("no memory export"))?;
		let bytes = memory.data(&*caller);
		let start = (pointer as usize).min(bytes.len());
		let end = bytes[start..].iter().position(|byte| *byte == 0).map_or(bytes.len(), |length| start + length);
		Ok(String::from_utf8_lossy(&bytes[start..end]).into_owned())
	}

	fn exported<T>(caller: &mut Caller<'_, T>, name: &str) -> Option<Func> {
		caller.get_export(name).and_then(|export| export.into_func())
	}

	thread_local! {
		/// per run: a seeded run is not disturbed by the runs of other threads
		static RANDOM_STATE: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
	}

	/// Runs that started unseeded, so two in the same millisecond differ
	static UNSEEDED_RUNS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

	/// The generator's state after `random_seed(seed)`, never 0 (web/playground/host.js seedRandom does the same)
	fn seeded_state(seed: u64) -> u64 {
		seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1
	}

	/// xorshift64*, seeded from the clock once per run unless random_seed set it: random enough for games and
	/// samples, not for secrets
	fn next_random() -> u64 {
		let mut x = RANDOM_STATE.get();
		if x == 0 {
			let run = UNSEEDED_RUNS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
			x = seeded_state(milliseconds_since_epoch() as u64 ^ run.wrapping_mul(0xD1B5_4A32_D192_ED03));
		}
		x ^= x >> 12;
		x ^= x << 25;
		x ^= x >> 27;
		RANDOM_STATE.set(x);
		x.wrapping_mul(0x2545_F491_4F6C_DD1D)
	}

	fn milliseconds_since_epoch() -> i64 {
		std::time::SystemTime::now()
			.duration_since(std::time::UNIX_EPOCH)
			.map_or(0, |elapsed| elapsed.as_millis() as i64)
	}
}
