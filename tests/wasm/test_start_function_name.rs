// The start function, which installs the witness tables in every instance (card traps-task), has its name in the name
// section like main, so backtraces and WAT show it (reviewer)
use wasmparser::{Name, NameSectionReader, Parser, Payload};

/// The start function's index and the function names of the module `code` compiles to
fn start_and_names(code: &str) -> (Option<u32>, Vec<(u32, String)>) {
	let bytes = warp::wasm_emitter::compile(code).unwrap_or_else(|value| panic!("{code}: {value:?}")).bytes;
	let (mut start, mut names) = (None, vec![]);
	for payload in Parser::new(0).parse_all(&bytes) {
		match payload.unwrap() {
			Payload::StartSection { func, .. } => start = Some(func),
			Payload::CustomSection(section) if section.name() == "name" => {
				let reader = wasmparser::BinaryReader::new(section.data(), section.data_offset());
				for subsection in NameSectionReader::new(reader) {
					if let Name::Function(map) = subsection.unwrap() {
						names.extend(map.into_iter().map(|naming| {
							let naming = naming.unwrap();
							(naming.index, naming.name.to_string())
						}));
					}
				}
			}
			_ => {}
		}
	}
	(start, names)
}

#[test]
fn the_start_function_is_named_install() {
	let (start, names) = start_and_names("class N{v}\ntext(n: N) := \"n\" + n.v\n[N(1), N(2)]");
	let start = start.expect("a printable class installs its witness table in a start function");
	assert!(names.contains(&(start, "install".to_string())), "{start}: {names:?}");
}
