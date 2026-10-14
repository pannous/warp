//! The C calls of the program described for a host that cannot see an import's types (the warp.c_calls section)
use super::*;

/// The custom section describing the program's C calls for a host that cannot see an import's types
/// (web/playground/host.js, which calls libc.wasm for `c` and imported modules): a line `module\tname\tparameters\tresult`
/// per import. Parameter letters: `t` a text (a NUL-terminated copy in the program's memory), `l` the length of the text
/// before it (strcmp's pairs, then not NUL-terminated), `s` that length passed on to the module too, `n` a number, `o`
/// an out-pointer, `b` a buffer of the capacity given, `k` its length slot (`o` and `k` take no value of the program);
/// result: `t` a text Node built by the host, `u` an unsigned number, `ot`/`on` what the first out-pointer received,
/// `b` the bytes the buffer received, `n` a number or none. libc.wasm has the program's number types (web/playground/lib/libc.c)
pub const C_CALLS_SECTION: &str = "warp.c_calls";
/// The C library whose calls the browser host makes through libc.wasm
pub(crate) const LIBC: &str = "c";

/// The C_CALLS_SECTION of these imports: libc's, and the exports of modules that take or give C texts, out-pointers or
/// unsigned results
pub fn c_calls<'a>(imports: impl Iterator<Item = &'a FfiSignature>) -> String {
	let line = |import: &FfiSignature, parameters: String, result: &str| {
		format!("{}\t{}\t{parameters}\t{result}\n", import.library, import.name)
	};
	let libc_line = |import: &FfiSignature| {
		let texts = crate::ffi::header_text_parameters(LIBC).get(import.name).cloned().unwrap_or_default();
		let pairs = crate::ffi::string_pair_count(import.name);
		let parameters = (0..import.params.len())
			.map(|index| match index {
				index if index < 2 * pairs => {
					if index % 2 == 0 {
						't'
					} else {
						'l'
					}
				}
				index if texts.get(index - pairs).copied().unwrap_or(false) => 't',
				_ => 'n',
			})
			.collect();
		line(
			import,
			parameters,
			if matches!(import.results.first(), Some(wasm_encoder::ValType::Ref(_))) { "t" } else { "n" },
		)
	};
	let module_line = |import: &FfiSignature, export: &Export| {
		let parameters = export
			.c_parameters
			.iter()
			.map(|parameter| match parameter {
				CParameter::Number => 'n',
				CParameter::Text => 't',
				CParameter::TextLength => 's',
				CParameter::Out => 'o',
				CParameter::Buffer => 'b',
				CParameter::BufferLength => 'k',
			})
			.collect();
		line(
			import,
			parameters,
			match export.c_result {
				CResult::Number => "n",
				CResult::Unsigned => "u",
				CResult::Text => "t",
				CResult::Out { text: true } => "ot",
				CResult::Out { text: false } => "on",
				CResult::Buffer => "b",
			},
		)
	};
	let lines: std::collections::BTreeSet<String> = imports
		.filter_map(|import| match import.library {
			LIBC => Some(libc_line(import)),
			library if is_module_path(library) => exports(library)
				.get(import.name)
				.filter(|export| export.crosses_c_values())
				.map(|export| module_line(import, export)),
			_ => None,
		})
		.collect();
	lines.into_iter().collect()
}

/// The parameters of a module function's import (the program's view) whose text crosses with its length in the next one
pub fn texts_with_lengths(signature: &FfiSignature) -> Vec<usize> {
	if !is_module_path(signature.library) {
		return (0..crate::ffi::string_pair_count(signature.name)).map(|pair| 2 * pair).collect();
	}
	let Some(export) = exports(signature.library).get(signature.name) else {
		return vec![];
	};
	let passed: Vec<CParameter> =
		export.c_parameters.iter().copied().filter(|parameter| parameter.is_passed()).collect();
	(0..passed.len())
		.filter(|index| passed[*index] == CParameter::Text && passed.get(index + 1) == Some(&CParameter::TextLength))
		.collect()
}
