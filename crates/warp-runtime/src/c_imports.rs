//! C calls: an import of a module other than host and WASI is the C function `name` of the library `module`
//! (`m`.`sqrt`, `sqlite3`.`sqlite3_open`, `c` the process itself), loaded with dlopen and found with dlsym. How its
//! parameters and result cross comes from the module's `warp.c_functions` section, one c_signatures.rs line per function
//! (`sqlite3 sqlite3 sqlite3_open ptr,out i32 handle sqlite3` with tabs between, nine fields), else from the import's own type: numbers as
//! they are, a reference parameter a warp text (copied NUL-terminated), a reference result a `char *` read back as text.
//! c_calls.rs marshals the call. A library or symbol that is missing fails the run before it starts, naming it.
//! warp-host and warp's own runs (src/ffi/link.rs) link a module's C imports so, alike.
use crate::c_calls::{function_pointer, get_or_load_library, library_paths, link_c_function_as, CallsC};
use crate::c_signatures::{CFunction, CPointer, Failure, ParamType, RetType};
use wasmtime::{ExternType, FuncType, Linker, Module, Result, ValType};

pub const C_FUNCTIONS_SECTION: &str = "warp.c_functions";
/// warp-rust's description of its C calls (src/modules/wasm_modules/c_calls.rs): `module name parameters result`
/// tab-separated, a letter per parameter, `t` a text it copied NUL-terminated into its memory, the parameter its address;
/// the result `t` a `char *` read back as a text, `ft` one the caller owns, read back and then freed (strdup's)
pub const C_CALLS_SECTION: &str = "warp.c_calls";
/// The result letters of an owned `char *` in C_CALLS_SECTION: "free the text" after reading it
pub const OWNED_TEXT_LETTERS: &str = "ft";

/// The C functions a module's `warp.c_functions` section describes (machine code keeps no custom section: aot.rs
/// carries them in front of it)
pub fn described(bytes: &[u8]) -> Result<Vec<CFunction>> {
	custom_sections(bytes, C_FUNCTIONS_SECTION)
		.flat_map(|section| String::from_utf8_lossy(section).lines().map(str::to_string).collect::<Vec<_>>())
		.filter(|line| !line.trim().is_empty())
		.map(|line| {
			CFunction::of_line(&line)
				.ok_or_else(|| wasmtime::Error::msg(format!("{C_FUNCTIONS_SECTION}: not a C function: {line:?}")))
		})
		.collect()
}

/// A C call warp-rust's `warp.c_calls` section letters: module, name, a letter per parameter, the result's letters
pub type Lettered = (String, String, String, String);

/// module.name → its parameters' letters, from warp-rust's modules (none for machine code, which keeps no section)
pub fn lettered(bytes: &[u8]) -> Vec<Lettered> {
	custom_sections(bytes, C_CALLS_SECTION)
		.flat_map(|section| String::from_utf8_lossy(section).lines().map(str::to_string).collect::<Vec<_>>())
		.filter_map(|line| {
			let fields: Vec<&str> = line.split('\t').collect();
			let field = |index: usize| fields.get(index).copied().unwrap_or_default().to_string();
			(fields.len() >= 3).then(|| (field(0), field(1), field(2), field(3)))
		})
		.collect()
}

/// What a module says of its C imports: its `warp.c_functions` (carried in front of machine code, aot.rs) and the
/// letters of warp-rust's `warp.c_calls`
#[derive(Default)]
pub struct CImports {
	pub described: Vec<CFunction>,
	pub lettered: Vec<Lettered>,
}

impl CImports {
	/// Both sections of a module's bytes
	pub fn of(bytes: &[u8]) -> Result<Self> {
		Ok(CImports { described: described(bytes)?, lettered: lettered(bytes) })
	}
}

/// Link every C import of `module` that `c.described` names, or that `defined` says the linker has no fitting
/// definition for
pub fn link_c_imports<T: CallsC>(
	linker: &mut Linker<T>,
	module: &Module,
	c: &CImports,
	mut defined: impl FnMut(&Linker<T>, &str, &str, &FuncType) -> bool,
) -> Result<()> {
	let (described, lettered) = (&c.described, &c.lettered);
	linker.allow_shadowing(true);
	for import in module.imports() {
		let (library, name) = (import.module(), import.name());
		let ExternType::Func(func_type) = import.ty() else { continue };
		if library == crate::host_words::HOST_LIBRARY || library.starts_with("wasi") {
			continue;
		}
		let description = described.iter().find(|function| function.module == library && function.name == name);
		let fits = defined(linker, library, name, &func_type);
		let function = match description {
			Some(function) => function.clone(),
			None if fits => continue,
			None => {
				let letters = lettered.iter().find(|(module, function, _, _)| module == library && function == name);
				let letters = letters.map(|(_, _, parameters, result)| (parameters.as_str(), result.as_str()));
				of_import(library, name, &func_type, letters)
			}
		};
		let loaded = get_or_load_library(&function.library).ok_or_else(|| {
			let tried = library_paths(&function.library).join(", ");
			wasmtime::Error::msg(format!(
				"the C library {} of the import {library}.{name} is not found (tried {tried})",
				function.library
			))
		})?;
		let pointer = function_pointer(&loaded, &function.name).map_err(|_| {
			wasmtime::Error::msg(format!("the C library {} has no function {}", function.library, function.name))
		})?;
		let passed = function.param_types.iter().filter(|param| param.is_passed()).count();
		if passed != func_type.params().len() {
			wasmtime::bail!(
				"the import {library}.{name} takes {} parameters, its C function {passed}",
				func_type.params().len()
			);
		}
		link_c_function_as(linker, function, pointer, func_type)?;
	}
	Ok(())
}

/// A C function as the import's own type says: numbers as they are, a reference parameter a text, a reference result
/// a text read back (freed after when the result's letters are `ft`); a parameter warp-rust's letters mark `t` the
/// address of a text in linear memory
fn of_import(library: &str, name: &str, func_type: &FuncType, letters: Option<(&str, &str)>) -> CFunction {
	let param = |value: ValType| match value {
		ValType::I64 => ParamType::I64,
		ValType::F32 => ParamType::F32,
		ValType::F64 => ParamType::F64,
		ValType::Ref(_) => ParamType::Ptr,
		_ => ParamType::I32,
	};
	let (ret_type, result) = match func_type.results().next() {
		None => (RetType::Void, None),
		Some(ValType::I64) => (RetType::I64, None),
		Some(ValType::F32) => (RetType::F32, None),
		Some(ValType::F64) => (RetType::F64, None),
		Some(ValType::Ref(_)) if letters.is_some_and(|(_, result)| result == OWNED_TEXT_LETTERS) => {
			(RetType::I64, Some(CPointer::OwnedText))
		}
		Some(ValType::Ref(_)) => (RetType::I64, Some(CPointer::Text)),
		Some(_) => (RetType::I32, None),
	};
	CFunction {
		module: library.to_string(),
		library: library.to_string(),
		name: name.to_string(),
		param_types: func_type
			.params()
			.enumerate()
			.map(|(index, value)| match letters.and_then(|(letters, _)| letters.chars().nth(index)) {
				Some('t') => ParamType::Ptr,
				_ => param(value),
			})
			.collect(),
		result,
		ret_type,
		out_pointee: None,
		layouts: vec![],
		failure: Failure::None,
	}
}

/// The payloads of the module's custom sections of that name
pub fn custom_sections<'a>(bytes: &'a [u8], wanted: &'a str) -> impl Iterator<Item = &'a [u8]> + 'a {
	fn uleb(bytes: &[u8], at: &mut usize) -> Option<usize> {
		let (mut value, mut shift) = (0usize, 0);
		loop {
			let byte = *bytes.get(*at)?;
			*at += 1;
			value |= usize::from(byte & 0x7f) << shift;
			if byte & 0x80 == 0 {
				return Some(value);
			}
			shift += 7;
		}
	}
	let mut at = 8; // after the magic number and version
	std::iter::from_fn(move || loop {
		let id = *bytes.get(at)?;
		at += 1;
		let size = uleb(bytes, &mut at)?;
		let (start, end) = (at, at.checked_add(size)?);
		at = end;
		if id == 0 {
			let mut inner = start;
			let length = uleb(bytes, &mut inner)?;
			let name = bytes.get(inner..inner + length)?;
			if name == wanted.as_bytes() {
				return bytes.get(inner + length..end);
			}
		}
	})
}
