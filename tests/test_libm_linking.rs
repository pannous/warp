// libm is linked from its headers first; the hand-linked table LIBM_UNARY/LIBM_BINARY serves only when the headers
// declare nothing for "m", as glibc's math.h does behind __MATHCALL macros (user decision 2026-10-03)
use warp::ffi::{libm_header_signatures, link_libm, FfiState, LibmSource};
use warp::is;
use wasmtime::{Linker, Store};

/// Which libm source served, and whether `m.<function>` is linked afterwards for each name
fn linked(signatures: &[warp::ffi::FfiHeaderSignature], names: &[&str]) -> (LibmSource, Vec<bool>) {
	let engine = warp::util::gc_engine();
	let mut linker: Linker<FfiState> = Linker::new(&engine);
	let source = link_libm(&mut linker, &engine, signatures).unwrap();
	let mut store = Store::new(&engine, FfiState::new());
	(source, names.iter().map(|name| linker.get(&mut store, "m", name).is_ok()).collect())
}

#[test]
fn libm_comes_from_the_headers_when_they_declare_it() {
	crate::requires!(crate::common::MACOS_C_HEADERS);
	let (source, found) = linked(&libm_header_signatures(), &["sin", "pow", "hypot"]);
	assert!(matches!(source, LibmSource::Headers(count) if count > 0), "{source:?}");
	assert_eq!(found, [true, true, true], "hypot is in the headers, not in the table");
	is!("import hypot from \"m\"\nhypot(3.0, 4.0)", 5.0);
}

#[test]
fn the_table_serves_libm_when_the_headers_declare_nothing() {
	let (source, found) = linked(&[], &["sin", "pow", "fmin", "hypot"]);
	assert_eq!(source, LibmSource::Table);
	assert_eq!(found, [true, true, true, false]);
}
