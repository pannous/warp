// The fixed runtime of every crate shim warp-crate generates (notes/crate_adapter.md): how a Rust value crosses the C
// ABI that the native warp's FFI speaks. Nothing here knows a crate; the generated wrappers below it call these.
#![allow(dead_code, clippy::missing_safety_doc, non_snake_case)]
use std::cell::RefCell;
use std::ffi::{c_char, CStr, CString};
use std::fmt::Display;

thread_local! {
	/// the last call's failure (an Err's Display, a panic, a NULL handle), ø after a call that worked
	static LAST_ERROR: RefCell<Option<CString>> = const { RefCell::new(None) };
	/// the texts of the last calls: a text result stays valid until TEXT_RING more text results came back
	static TEXTS: RefCell<(usize, Vec<Option<CString>>)> = RefCell::new((0, (0..TEXT_RING).map(|_| None).collect()));
}
const TEXT_RING: usize = 16;

fn c_text(text: &str) -> CString {
	let end = text.find('\0').unwrap_or(text.len());
	CString::new(&text[..end]).unwrap_or_default()
}

pub fn fail(message: impl Display) {
	LAST_ERROR.with(|last| *last.borrow_mut() = Some(c_text(&message.to_string())));
}
pub fn clear_error() {
	LAST_ERROR.with(|last| *last.borrow_mut() = None);
}
pub fn last_error() -> *const c_char {
	LAST_ERROR.with(|last| last.borrow().as_ref().map_or(std::ptr::null(), |text| text.as_ptr()))
}

/// A text crossing back: kept in the ring, its pointer handed out
pub fn keep_text(text: &str) -> *const c_char {
	TEXTS.with(|texts| {
		let (next, ring) = &mut *texts.borrow_mut();
		let kept = c_text(text);
		let pointer = kept.as_ptr();
		ring[*next] = Some(kept);
		*next = (*next + 1) % TEXT_RING;
		pointer
	})
}

/// A text parameter: the native warp passes a NUL-terminated copy; NULL is ""
pub unsafe fn text_in<'a>(pointer: *const c_char) -> &'a str {
	if pointer.is_null() {
		return "";
	}
	match CStr::from_ptr(pointer).to_str() {
		Ok(text) => text,
		Err(_) => "",
	}
}
/// A bytes parameter: the native warp passes the bytes of a text or a list of 0 to 255, NULs and all, and their count
pub unsafe fn bytes_in<'a>(pointer: *const u8, size: usize) -> &'a [u8] {
	if pointer.is_null() || size == 0 {
		return &[];
	}
	std::slice::from_raw_parts(pointer, size)
}
/// A handle parameter: the value a handle result boxed; NULL fails the call
pub unsafe fn handle_in<'a, T>(pointer: *mut T, name: &str) -> Option<&'a mut T> {
	if pointer.is_null() {
		fail(format!("{name}: no value (a NULL handle)"));
	}
	pointer.as_mut()
}

/// The C value of a failed call: -1 for numbers, NaN, false, NULL
pub trait Null {
	fn null() -> Self;
}
macro_rules! nulls { ($($t:ty => $v:expr),*) => { $(impl Null for $t { fn null() -> Self { $v } })* } }
nulls!(i32 => -1, i64 => -1, f32 => f32::NAN, f64 => f64::NAN, bool => false, () => ());
impl Null for *const c_char {
	fn null() -> Self {
		std::ptr::null()
	}
}
impl<T> Null for *mut T {
	fn null() -> Self {
		std::ptr::null_mut()
	}
}

/// A Rust result as the C value C: numbers cast, texts kept, None and Err the null (an Err recorded as last_error)
pub trait Out<C> {
	fn out(self) -> C;
}
macro_rules! numbers_out {
	($($c:ty: $($t:ty),*;)*) => { $($(impl Out<$c> for $t { fn out(self) -> $c { self as $c } })*)* }
}
numbers_out! {
	i64: i8, i16, i32, i64, isize, u8, u16, u32, u64, usize, f32, f64;
	i32: i8, i16, i32, i64, isize, u8, u16, u32, u64, usize, f32, f64;
	f64: i8, i16, i32, i64, isize, u8, u16, u32, u64, usize, f32, f64;
	f32: i8, i16, i32, i64, isize, u8, u16, u32, u64, usize, f32, f64;
}
impl Out<i64> for char {
	fn out(self) -> i64 {
		self as i64
	}
}
impl Out<i32> for char {
	fn out(self) -> i32 {
		self as i32
	}
}
impl Out<bool> for bool {
	fn out(self) -> bool {
		self
	}
}
impl Out<()> for () {
	fn out(self) {}
}
impl Out<*const c_char> for String {
	fn out(self) -> *const c_char {
		keep_text(&self)
	}
}
impl Out<*const c_char> for &String {
	fn out(self) -> *const c_char {
		keep_text(self)
	}
}
impl Out<*const c_char> for &str {
	fn out(self) -> *const c_char {
		keep_text(self)
	}
}
impl Out<*const c_char> for std::borrow::Cow<'_, str> {
	fn out(self) -> *const c_char {
		keep_text(&self)
	}
}
impl Out<*const c_char> for char {
	fn out(self) -> *const c_char {
		keep_text(&self.to_string())
	}
}
impl<C: Null, T: Out<C>> Out<C> for Option<T> {
	fn out(self) -> C {
		self.map_or_else(C::null, Out::out)
	}
}
impl<C: Null, T: Out<C>, E: Display> Out<C> for Result<T, E> {
	fn out(self) -> C {
		match self {
			Ok(value) => value.out(),
			Err(problem) => {
				fail(problem);
				C::null()
			}
		}
	}
}

/// A value kept for warp as an opaque handle of type T: boxed; None and Err are NULL
pub trait HandleOut<T> {
	fn handle_out(self) -> *mut T;
}
impl<T> HandleOut<T> for T {
	fn handle_out(self) -> *mut T {
		Box::into_raw(Box::new(self))
	}
}
impl<T> HandleOut<T> for Option<T> {
	fn handle_out(self) -> *mut T {
		self.map_or(std::ptr::null_mut(), HandleOut::handle_out)
	}
}
impl<T, E: Display> HandleOut<T> for Result<T, E> {
	fn handle_out(self) -> *mut T {
		match self {
			Ok(value) => value.handle_out(),
			Err(problem) => {
				fail(problem);
				std::ptr::null_mut()
			}
		}
	}
}

/// Bytes as lowercase hex text (a digest)
pub fn hex_out(bytes: impl AsRef<[u8]>) -> *const c_char {
	let text: String = bytes.as_ref().iter().map(|byte| format!("{byte:02x}")).collect();
	keep_text(&text)
}

/// Bytes kept as a handle: warp reads them with <library>_bytes_count / _bytes_at into a list of 0 to 255
pub struct ByteList(pub Vec<u8>);
pub fn bytes_out(bytes: impl AsRef<[u8]>) -> *mut ByteList {
	Box::into_raw(Box::new(ByteList(bytes.as_ref().to_vec())))
}

/// A list of texts kept as a handle: warp reads it with <library>_list_count / _list_text and frees it
pub struct TextList(pub Vec<String>);
pub fn list_out<T: Into<ListItem>>(items: impl IntoIterator<Item = T>) -> *mut TextList {
	Box::into_raw(Box::new(TextList(items.into_iter().map(|item| item.into().0).collect())))
}
pub struct ListItem(pub String);
impl From<&str> for ListItem {
	fn from(text: &str) -> Self {
		ListItem(text.to_string())
	}
}
impl From<String> for ListItem {
	fn from(text: String) -> Self {
		ListItem(text)
	}
}
impl From<&String> for ListItem {
	fn from(text: &String) -> Self {
		ListItem(text.clone())
	}
}
impl From<std::borrow::Cow<'_, str>> for ListItem {
	fn from(text: std::borrow::Cow<'_, str>) -> Self {
		ListItem(text.into_owned())
	}
}
macro_rules! number_items { ($($t:ty),*) => { $(impl From<$t> for ListItem { fn from(n: $t) -> Self { ListItem(n.to_string()) } })* } }
number_items!(i8, i16, i32, i64, isize, u8, u16, u32, u64, usize, f32, f64, char, bool);

/// The call `body`, its panic caught and recorded (unwinding into C would abort the native warp)
pub fn guarded<C: Null>(body: impl FnOnce() -> Option<C>) -> C {
	clear_error();
	match std::panic::catch_unwind(std::panic::AssertUnwindSafe(body)) {
		Ok(Some(value)) => value,
		Ok(None) => C::null(),
		Err(panic) => {
			let message = panic
				.downcast_ref::<&str>()
				.map(|text| text.to_string())
				.or_else(|| panic.downcast_ref::<String>().cloned())
				.unwrap_or_else(|| "a panic".to_string());
			fail(format!("panic: {message}"));
			C::null()
		}
	}
}
