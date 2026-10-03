// binaryen (wasm-opt) expects the subsections of the "name" custom section in ascending id order:
// module 0, function 1, local 2, label 3, type 4, table 5, memory 6, global 7, elem 8, data 9, field 10, tag 11
use warp::wasm_emitter::compile;
use warp::{FieldDef, RawFieldValue, TypeDef, WasmGcEmitter};
use wasmparser::{BinaryReader, KnownCustom, Parser, Payload};

const PROGRAMS: [&str; 4] = ["try 1 + [1 2]#5 else 7", "def square(x):=x*x; square(3)", "x=[1,2,3]; x#2", "\"hi\" + 3"];

/// Ids of the name subsections of `bytes`, in the order they are written.
fn name_subsection_ids(bytes: &[u8]) -> Vec<u8> {
	for payload in Parser::new(0).parse_all(bytes) {
		if let Payload::CustomSection(section) = payload.expect("valid module") {
			if let KnownCustom::Name(_) = section.as_known() {
				let mut reader = BinaryReader::new(section.data(), section.data_offset());
				let mut ids = vec![];
				while !reader.eof() {
					ids.push(reader.read_u8().unwrap());
					let size = reader.read_var_u32().unwrap() as usize;
					reader.read_bytes(size).unwrap();
				}
				return ids;
			}
		}
	}
	panic!("module has no name section");
}

fn assert_ascending(bytes: &[u8], what: &str) {
	let ids = name_subsection_ids(bytes);
	assert!(ids.windows(2).all(|pair| pair[0] < pair[1]), "{what}: name subsection ids {ids:?} not strictly ascending");
}

#[test]
fn name_subsections_ascend_in_compiled_programs() {
	for program in PROGRAMS {
		let module = compile(program).unwrap_or_else(|answer| panic!("{program} compiled to constant {answer:?}"));
		assert_ascending(&module.bytes, program);
	}
}

#[test]
fn name_subsections_ascend_in_raw_struct_modules() {
	let person = TypeDef {
		name: "Person".to_string(),
		tag: 100,
		fields: vec![
			FieldDef { name: "name".to_string(), type_name: "String".to_string() },
			FieldDef { name: "age".to_string(), type_name: "i64".to_string() },
		],
		wasm_type_idx: None,
	};
	let bytes = WasmGcEmitter::emit_raw_struct(&person, &[RawFieldValue::from("Alice"), RawFieldValue::from(30i64)]);
	assert_ascending(&bytes, "raw Person struct");
}
