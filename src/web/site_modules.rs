//! The WebAssembly modules a site's program imports by path (`import shout`, `use uniscript`'s uniscript_c.wasm): the
//! site ships each beside the page as modules/<file>, and the program's imports and its warp.c_calls lines name it
//! `page:modules/<file>`, which host-foreign.js resolves beside the page. Natively they name the module's own path.
use crate::modules::wasm_modules::{is_module_path, module_bytes, C_CALLS_SECTION};
use std::collections::HashMap;
use std::path::Path;
use wasm_encoder::reencode::{utils, Error, Reencode};
use warp_runtime::site_files::SiteFile;

const MODULES_DIRECTORY: &str = "modules";
const WASM_EXTENSION: &str = "wasm";

/// The program with its module imports renamed to the files the site ships, and those files
pub fn with_shipped_modules(program: &[u8]) -> Result<(Vec<u8>, Vec<SiteFile>), String> {
	let mut shipped = Renaming::default();
	let mut files = vec![];
	for (module, _) in crate::host::host_parts::imports_of(program)? {
		if !is_module_path(&module) || shipped.0.contains_key(&module) {
			continue;
		}
		let stem = Path::new(&module).file_stem().unwrap_or_default().to_string_lossy();
		let file = format!("{MODULES_DIRECTORY}/{stem}.{WASM_EXTENSION}");
		if files.iter().any(|(taken, _): &SiteFile| *taken == file) {
			return Err(format!("{module}: the site ships another module as {file} already; rename one of them"));
		}
		files.push((file.clone(), module_bytes(&module)?));
		shipped.0.insert(module, format!("{}{file}", crate::web::PAGE_PREFIX));
	}
	if files.is_empty() {
		return Ok((program.to_vec(), files));
	}
	let mut renamed = wasm_encoder::Module::new();
	shipped
		.parse_core_module(&mut renamed, wasmparser::Parser::new(0), program)
		.map_err(|failure| format!("{failure:?}"))?;
	Ok((renamed.finish(), files))
}

/// module path → the name the page knows it by
#[derive(Default)]
struct Renaming(HashMap<String, String>);

impl Renaming {
	fn name_of<'a>(&'a self, module: &'a str) -> &'a str {
		self.0.get(module).map_or(module, String::as_str)
	}
}

impl Reencode for Renaming {
	type Error = String;

	fn parse_imports(
		&mut self,
		section: &mut wasm_encoder::ImportSection,
		imports: wasmparser::Imports<'_>,
	) -> Result<(), Error<String>> {
		let import = match imports {
			wasmparser::Imports::Single(_, import) => import,
			wasmparser::Imports::Compact1 { module, .. } | wasmparser::Imports::Compact2 { module, .. }
				if self.0.contains_key(module) =>
			{
				return Err(Error::UserError(format!("{module}: compact imports of a shipped module are not renamed yet")))
			}
			compact => return utils::parse_imports(self, section, compact),
		};
		let entity = self.entity_type(import.ty)?;
		section.import(self.name_of(import.module), import.name, entity);
		Ok(())
	}

	fn parse_custom_section(
		&mut self,
		module: &mut wasm_encoder::Module,
		section: wasmparser::CustomSectionReader<'_>,
	) -> Result<(), Error<String>> {
		if section.name() != C_CALLS_SECTION {
			return utils::parse_custom_section(self, module, section);
		}
		let lines = String::from_utf8_lossy(section.data());
		let renamed: String = lines
			.split_inclusive('\n')
			.map(|line| match line.split_once('\t') {
				Some((library, rest)) => format!("{}\t{rest}", self.name_of(library)),
				None => line.to_string(),
			})
			.collect();
		module.section(&wasm_encoder::CustomSection {
			name: C_CALLS_SECTION.into(),
			data: renamed.as_bytes().into(),
		});
		Ok(())
	}
}
