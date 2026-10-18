//! Cranelift's machine code of each function kept on disk between runs (wasmtime's incremental compilation cache, card
//! prelude-word): a fresh program's module is new to the module cache, but most of its functions (the runtime's, the
//! prelude's) are the same in every module, so only the program's own few compile. Keys are Cranelift's (a function's
//! code and the compiler's settings), hashed into file names of the module cache.
use super::module_cache::{keep_bytes, kept_bytes};
use sha2::{Digest, Sha256};
use std::borrow::Cow;

/// The extension of a function's kept machine code in the module cache
const FUNCTION_EXTENSION: &str = "function";

#[derive(Debug)]
pub struct FunctionCache;

impl wasmtime::CacheStore for FunctionCache {
	fn get(&self, key: &[u8]) -> Option<Cow<'_, [u8]>> {
		kept_bytes(&file_key(key), FUNCTION_EXTENSION).map(Cow::Owned)
	}

	fn insert(&self, key: &[u8], value: Vec<u8>) -> bool {
		keep_bytes(&file_key(key), FUNCTION_EXTENSION, &value);
		true
	}
}

fn file_key(key: &[u8]) -> String {
	crate::extensions::hex_text(&Sha256::digest(key))
}
