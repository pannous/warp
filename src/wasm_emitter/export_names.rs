use std::collections::HashSet;
use wasm_encoder::{ExportKind, ExportSection};

/// The export section with the names it holds. A program's function is exported last and only under a name still
/// free: one named like a runtime function or the entry (`node_count`, `main`) leaves that name to them, which the
/// host calls by name (card export-name-clash)
#[derive(Default)]
pub(super) struct Exports {
	section: ExportSection,
	taken: HashSet<String>,
	program_functions: Vec<(String, u32)>,
}

impl Exports {
	pub(super) fn export(&mut self, name: &str, kind: ExportKind, index: u32) {
		self.taken.insert(name.to_string());
		self.section.export(name, kind, index);
	}

	pub(super) fn export_program_function(&mut self, name: &str, index: u32) {
		self.program_functions.push((name.to_string(), index));
	}

	/// The finished section, the program's functions added under the names left free
	pub(super) fn section(&mut self) -> &ExportSection {
		for (name, index) in std::mem::take(&mut self.program_functions) {
			if !self.taken.contains(&name) {
				self.export(&name, ExportKind::Func, index);
			}
		}
		&self.section
	}
}
