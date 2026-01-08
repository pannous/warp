mod wasm_parser {}

use std::any::{Any, TypeId};
use wasmparser::*;

use parity_wasm::*;
use std::collections::HashMap;
use std::fs::read;
use std::fs::File;
use std::io::Cursor;
use std::io::Read;
use std::io::Write;
use std::path::Path;
use std::str::from_utf8;
use wasmparser::{BinaryReaderError, Chunk, Parser, Payload::*};

pub fn parse_wasm(file: &str) -> Result<()> {
	let data = read(Path::new(file)).unwrap();
	// let mut parser = Parser::new(0);
	// let mut module = ModuleReader::new(Cursor::new(&data)).unwrap();
	// let mut reader = std::io::Cursor::new(data);
	// reader.read_to_end(&mut &data)?;
	let parser = Parser::new(0);
	for payload in parser.parse_all(&data) {
		match payload? {
			// Sections for WebAssembly modules
			Version { num, .. } => {
				println!("num {num}");
			}
			TypeSection(a) => {
				println!("TypeSection {:#?}", a);
				for func in a {
					println!("func {:#?}", func);
				}
			}
			ImportSection(_) => { /* ... */ }

			FunctionSection(_a) => {
				// for (index, func_type) in a.enumerate() {
				//     match func_type {
				//         Some(ft) => println!("func {}: {:?}", index, ft),
				//         None => println!("func {}: Error, missing function type", index),
				//     }
				// }
			}
			TableSection(_) => { /* ... */ }
			MemorySection(_) => { /* ... */ }
			TagSection(_) => { /* ... */ }
			GlobalSection(_) => { /* ... */ }
			ExportSection(_) => { /* ... */ }
			StartSection { .. } => { /* ... */ }
			ElementSection(_) => { /* ... */ }
			DataCountSection { .. } => { /* ... */ }
			DataSection(_) => { /* ... */ }

			// Here we know how many functions we'll be receiving as
			// `CodeSectionEntry`, so we can prepare for that, and
			// afterwards we can parse and handle each function
			// individually.
			CodeSectionStart { .. } => { /* ... */ }
			CodeSectionEntry(body) => {
				print!("CodeSectionEntry {:#?}", body);
				// here we can iterate over `body` to parse the function
				// and its locals
			}

			// Sections for WebAssembly components
			ModuleSection { .. } => { /* ... */ }
			InstanceSection(_) => { /* ... */ }
			CoreTypeSection(_) => { /* ... */ }
			ComponentSection { .. } => { /* ... */ }
			ComponentInstanceSection(_) => { /* ... */ }
			ComponentAliasSection(_) => { /* ... */ }
			ComponentTypeSection(_) => { /* ... */ }
			ComponentCanonicalSection(_) => { /* ... */ }
			ComponentStartSection { .. } => { /* ... */ }
			ComponentImportSection(_) => { /* ... */ }
			ComponentExportSection(_) => { /* ... */ }

			CustomSection(_) => { /* ... */ }

			// most likely you'd return an error here
			// UnknownSection { id, .. } => { return Err(BinaryReaderError::UnknownSection { id }); }
			_ => {} // Once we've reached the end of a parser we either resume
			        // at the parent parser or the payload iterator is at its
			        // end and we're done.
			        // End(_) => {}
		}
	}

	Ok(())

	// println!("{:?}", module);
	// module.
	// let mut parser = Parser::new(0);
}

pub fn parse_wasm_parity(_file: &str) {
	// parity_wasm::deserialize_file()
}

pub fn main() {
	let _ = parse_wasm("test.wasm");
}
