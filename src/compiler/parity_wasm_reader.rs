mod wasm_parser {}

use std::any::{Any, TypeId};
// use wasmparser::*;
use parity_wasm::*;
use std::io::Cursor;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::str::from_utf8;
use std::fs::read;

pub fn parse_wasm(file: &str) {
    let module = parity_wasm::elements::deserialize_file(file).unwrap();
    println!("{:?}", module);
    let _ = module.clone().parse_names(); // returns self :(
    if module.has_names_section() { // export names are ELSEWHERE!
        let names = module.names_section().unwrap();
        println!("{:?}", names);
    }
    let exports = module.export_section().unwrap().entries();
    let mut export_names = Vec::new();
    for export in exports {
        println!("{:?}", export);
        export_names.push(export.field());
    }
    let types = module.type_section().unwrap().types();
    for t in types {
        if t.type_id() == TypeId::of::<parity_wasm::elements::Type>() {}
        if t.type_id() == TypeId::of::<elements::Type>() {
            println!("{:?}", t);
        }
        println!("{:?}", t);
    }

    let function_section = module.function_section().unwrap();
    let funcs = function_section.entries();
    for func in funcs {
        println!("{:?}", func);
        let name = export_names.get(func.type_ref() as usize);
        println!("{:?}", name);
    }

    // let data = read(Path::new(file)).unwrap();
    // let mut parser = Parser::new(0);
    // let mut module = ModuleReader::new(Cursor::new(&data)).unwrap();
    // println!("{:?}", module);
    // module.
}


pub fn parse_wasm_parity(_file: &str) {
    // parity_wasm::deserialize_file()
}

pub fn main() {
    parse_wasm("test.wasm");
}