//! The signatures of a library's C headers kept on disk between runs (card prelude-cache-runs, notes/
//! incremental_builds.md): parsing libc's headers took ~2.5 ms of every program whose analysis looks up `abs` or `/!`.
//! The key: the warp binary (whose parser reads them), the library and each header file's path, size and time. A
//! signature of another value type than i32, i64, f32, f64 or anyref is not kept: the library is parsed at each run then.
use super::FfiSignature;
use std::collections::HashMap;
use wasm_encoder::ValType;

/// The extension of a library's kept header signatures in the module cache
#[cfg(feature = "native")]
const HEADERS_EXTENSION: &str = "headers";
/// The value types header signatures use (header_result: a text result is an anyref)
const KEPT_TYPES: [ValType; 5] =
	[ValType::I32, ValType::I64, ValType::F32, ValType::F64, ValType::Ref(wasm_encoder::RefType::ANYREF)];

/// A signature as kept: name, library, the parameters' and the results' value types by their place in KEPT_TYPES
type KeptSignature = (String, String, Vec<u8>, Vec<u8>);

/// The signatures `parse` reads from the library's `headers`, or as a run before this one kept them
pub(super) fn header_signatures(
	library: &str,
	headers: &[String],
	parse: impl FnOnce() -> HashMap<String, FfiSignature>,
) -> HashMap<String, FfiSignature> {
	let Some(key) = key(library, headers) else { return parse() };
	if let Some(signatures) = kept_signatures(&key) {
		return signatures;
	}
	let signatures = parse();
	let kept: Option<Vec<KeptSignature>> = signatures.values().map(kept_signature).collect();
	match kept.and_then(|kept| postcard::to_allocvec(&kept).ok()) {
		Some(bytes) => keep(&key, &bytes),
		None => log::debug!("the headers of {library} declare a value type the header cache does not keep"),
	}
	signatures
}

fn kept_signature(signature: &FfiSignature) -> Option<KeptSignature> {
	let codes = |types: &[ValType]| -> Option<Vec<u8>> {
		types
			.iter()
			.map(|value_type| KEPT_TYPES.iter().position(|kept| kept == value_type).map(|at| at as u8))
			.collect()
	};
	let (name, library) = (signature.name.to_string(), signature.library.to_string());
	Some((name, library, codes(&signature.params)?, codes(&signature.results)?))
}

fn kept_signatures(key: &str) -> Option<HashMap<String, FfiSignature>> {
	let kept: Vec<KeptSignature> = postcard::from_bytes(&kept_bytes(key)?).ok()?;
	let types = |codes: &[u8]| -> Option<Vec<ValType>> {
		codes.iter().map(|code| KEPT_TYPES.get(*code as usize).copied()).collect()
	};
	kept.into_iter()
		.map(|(name, library, params, results)| {
			let (params, results) = (types(&params)?, types(&results)?);
			// 'static as the parsed signatures' names are (header_sig_to_ffi_sig)
			let (name, library): (&'static str, &'static str) = (name.leak(), library.leak());
			Some((name.to_string(), FfiSignature { name, library, params, results }))
		})
		.collect()
}

#[cfg(feature = "native")]
fn kept_bytes(key: &str) -> Option<Vec<u8>> {
	crate::run::module_cache::kept_bytes(key, HEADERS_EXTENSION)
}

#[cfg(not(feature = "native"))]
fn kept_bytes(_: &str) -> Option<Vec<u8>> {
	None
}

#[cfg(feature = "native")]
fn keep(key: &str, bytes: &[u8]) {
	crate::run::module_cache::keep_bytes(key, HEADERS_EXTENSION, bytes);
}

#[cfg(not(feature = "native"))]
fn keep(_: &str, _: &[u8]) {}

#[cfg(feature = "native")]
fn key(library: &str, headers: &[String]) -> Option<String> {
	use sha2::{Digest, Sha256};
	let mut hasher = Sha256::new();
	hasher.update(crate::modules::binary_stamp()?);
	hasher.update(format!("{library}\n"));
	for header in headers {
		let metadata = std::fs::metadata(header).ok()?;
		hasher.update(format!("{header}\n{}\n{:?}\n", metadata.len(), metadata.modified().ok()?));
	}
	Some(hasher.finalize().iter().map(|byte| format!("{byte:02x}")).collect())
}

#[cfg(not(feature = "native"))]
fn key(_: &str, _: &[String]) -> Option<String> {
	None
}
