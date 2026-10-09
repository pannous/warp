//! get_text_ptr / get_text_len: a host without GC field access (the browser page web/uniscript) reads result texts through them

use warp::host::{create_host_linker, HostState};
use warp::util::{fueled_store, gc_engine};
use warp::wasm_emitter::compile;
use wasmtime::{Instance, Memory, Module, Rooted, Store, StructRef, Val};

const KIND_MASK: i64 = 0xff;
const KIND_TEXT: i64 = 3;
const KIND_ERROR: i64 = 11;
const TEXT_HEAP_EXPORT: &str = "text_heap";
const PAGE_BITS: usize = 16;

struct Compiled {
	store: Store<HostState>,
	instance: Instance,
}

impl Compiled {
	fn of(code: &str) -> Self {
		let module_bytes = compile(code).unwrap_or_else(|value| panic!("nothing to compile: {}", value.serialize())).bytes;
		let engine = gc_engine();
		let module = Module::new(&engine, &module_bytes).unwrap();
		let mut store = fueled_store(&engine, HostState::new());
		let instance = create_host_linker(&engine).unwrap().instantiate(&mut store, &module).unwrap();
		Compiled { store, instance }
	}

	fn call(&mut self, name: &str, arguments: &[Val]) -> Val {
		let function = self.instance.get_func(&mut self.store, name).unwrap_or_else(|| panic!("no export {name}"));
		let mut results = vec![Val::I32(0)];
		function.call(&mut self.store, arguments, &mut results).unwrap();
		results.remove(0)
	}

	fn memory(&mut self) -> Memory {
		self.instance.get_memory(&mut self.store, "memory").unwrap()
	}

	/// the kind and text of a node, read the way JavaScript does: getters and linear memory only
	fn read(&mut self, node: &Rooted<StructRef>) -> (i64, String) {
		let argument = [Val::AnyRef(Some((*node).into()))];
		let kind = self.call("get_kind", &argument).unwrap_i64() & KIND_MASK;
		let pointer = self.call("get_text_ptr", &argument).unwrap_i32() as usize;
		let length = self.call("get_text_len", &argument).unwrap_i32() as usize;
		let memory = self.memory();
		(kind, String::from_utf8(memory.data(&self.store)[pointer..pointer + length].to_vec()).unwrap())
	}

	fn main(&mut self) -> (i64, String) {
		let node = self.call("main", &[]).unwrap_anyref().unwrap().unwrap_struct(&self.store).unwrap();
		self.read(&node)
	}

	/// a text node made by the module from bytes written past its memory, as the browser host does: from its text heap
	/// (web/uniscript/uniscript.js writeBytes), the heap pointer moved past them, so texts the module builds later never
	/// overwrite them
	fn new_text(&mut self, text: &str) -> Val {
		let memory = self.memory();
		let heap = self.instance.get_global(&mut self.store, TEXT_HEAP_EXPORT).expect("the text heap export");
		let memory_end = memory.data_size(&self.store);
		let mut pointer = heap.get(&mut self.store).unwrap_i32() as usize;
		if pointer == 0 || pointer + text.len() > memory_end {
			memory.grow(&mut self.store, (text.len() >> PAGE_BITS) as u64 + 1).unwrap();
			pointer = memory_end;
		}
		memory.write(&mut self.store, pointer, text.as_bytes()).unwrap();
		heap.set(&mut self.store, Val::I32((pointer + text.len()) as i32)).unwrap();
		self.call("new_text", &[Val::I32(pointer as i32), Val::I32(text.len() as i32)])
	}

	fn apply(&mut self, function: &str, text: &str) -> (i64, String) {
		let argument = self.new_text(text);
		let node = self.call(function, &[argument]).unwrap_anyref().unwrap().unwrap_struct(&self.store).unwrap();
		self.read(&node)
	}
}

#[test]
fn text_getters_read_texts_and_errors() {
	assert_eq!(Compiled::of("\"héllo\"").main(), (KIND_TEXT, "héllo".to_string()));
	assert_eq!(Compiled::of("error(\"bad\")").main(), (KIND_ERROR, "bad".to_string()));
	assert_eq!(Compiled::of("42").main().1, "");
}

#[test]
fn compiled_uniscript_converts_like_the_browser_page() {
	let mut uniscript = Compiled::of(&std::fs::read_to_string("web/uniscript/uniscript.warp").unwrap());
	uniscript.main();
	assert_eq!(uniscript.apply("uniscript", "<:alpha> <:fracture A> \\:infinity"), (KIND_TEXT, "α 𝔄 ∞".to_string()));
	assert_eq!(uniscript.apply("uniscript", "<:nosuchthing>"), (KIND_ERROR, "unknown uniscript entity: nosuchthing".to_string()));
	assert_eq!(uniscript.apply("unicode_to_uniscript", "α 𝔄"), (KIND_TEXT, "\\:alpha \\:fracture-A".to_string()));
}

