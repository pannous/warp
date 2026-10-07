//! `warp build --component app.wasp` (card wasm-interop-rest, component worlds): the program compiled to a WebAssembly
//! component of the world its `component` declaration names (component_worlds.rs). Each function a world export lists
//! is the program's `export def` of that name, reached through an adapter of the canonical ABI the emitter adds
//! (wasm_emitter/component_adapters.rs); the module, its WIT embedded, becomes the component through wit-component.
const WIT_PATH: &str = "world.wit";

/// The component of the program `code`
pub fn build(code: &str) -> Result<Vec<u8>, String> {
	let program = crate::wasp_parser::parse(code);
	let world = crate::component_worlds::world(&program)?;
	let wit = crate::component_worlds::world_wit(&program)?;
	let module = crate::pipeline::for_a_component(world.functions(), || crate::pipeline::compile(code))
		.map_err(|value| format!("nothing to compile: {}", value.serialize()))?;
	let mut bytes = module.bytes;
	let failure = |error: anyhow::Error| format!("{error:#}");
	let mut resolve = wit_parser::Resolve::default();
	let package = resolve.push_str(WIT_PATH, &wit).map_err(failure)?;
	let world_id = resolve.select_world(&[package], None).map_err(failure)?;
	wit_component::embed_component_metadata(&mut bytes, &resolve, world_id, wit_component::StringEncoding::UTF8).map_err(failure)?;
	wit_component::ComponentEncoder::default().validate(true).module(&bytes).map_err(failure)?.encode().map_err(failure)
}
