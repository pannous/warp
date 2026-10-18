//! The standard modules' sources lowered once and kept on disk between runs (card prelude-cache-runs, notes/
//! incremental_builds.md): `use sound` spent ~150 ms of its ~430 ms parsing and lowering lib/sound.warp at every run.
//! Kept is pipeline::lower_module_source's result, not wasm, so whole-program typing and tree shaking stay. The key: the
//! warp binary (whose sources the module and its passes are), the module's source and what its passes read of the
//! compile so far (compile_context::lowering_inputs, LoweredEffects); the module files its `use`s found are checked at
//! each hit. A lowering that says something, numbers fresh names, changes an input it does not keep the change of, or
//! holds a Data payload serde does not keep (meta.rs PayloadCodec) is not kept.
use super::*;
use crate::compiler::compile_context::{lowering_inputs, LoweredEffects};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;

/// The extension of a kept lowered module in the module cache
#[cfg(feature = "native")]
const LOWERED_EXTENSION: &str = "lowered";

/// A module name looked up and the file found then (its path, size and time), none for a standard module
type Lookup = (String, Option<String>);

/// A lowered module and what its lowering left in the compile context
#[derive(Serialize, Deserialize)]
struct Kept {
	/// each module name the lowering looked up
	lookups: Vec<Lookup>,
	module: Node,
	effects: LoweredEffects,
}

thread_local! {
	/// The module lookups of the lowering being kept, none when no lowering is
	static LOOKUPS: RefCell<Option<Vec<Lookup>>> = const { RefCell::new(None) };
}

/// insert_module_classes looked `name` up: a kept lowering is valid only while it finds the same file
pub(super) fn note_lookup(name: &str, found: impl FnOnce() -> Option<PathBuf>) {
	LOOKUPS.with_borrow_mut(|lookups| {
		if let Some(lookups) = lookups {
			lookups.push((name.to_string(), file_stamp(found())));
		}
	});
}

/// The standard module's source lowered by `lower`, or as a run before this one kept it
pub(super) fn lowered_std_module(
	source: &str,
	classes_early: bool,
	lower: impl FnOnce() -> Result<Node, Node>,
) -> Result<Node, Node> {
	lowered_kept(source.as_bytes(), classes_early, lower)
}

/// Parsed definitions of a standard module (the prelude's the program reaches) lowered by `lower`, or as kept
pub(super) fn lowered_std_definitions(
	definitions: &Node,
	classes_early: bool,
	lower: impl FnOnce() -> Result<Node, Node>,
) -> Result<Node, Node> {
	match postcard::to_allocvec(definitions) {
		Ok(identity) => lowered_kept(&identity, classes_early, lower),
		Err(_) => lower(),
	}
}

/// The prelude's definition as its module source passes leave it (prelude reach look-ahead), or as a run before this
/// one kept it
pub(super) fn lowered_prelude_definition(definition: &Node) -> Vec<Node> {
	let module = Node::List(vec![definition.clone()], Bracket::None, Separator::Semicolon);
	let lowered = lowered_std_definitions(&module, false, || Ok(crate::pipeline::lower_module_source(module.clone())));
	statements(lowered.unwrap_or_else(|failure| failure))
}

/// What `lower` makes of the code `identity` stands for (its source, or its parsed form serialized), or as kept
fn lowered_kept(
	identity: &[u8],
	classes_early: bool,
	lower: impl FnOnce() -> Result<Node, Node>,
) -> Result<Node, Node> {
	let inputs = lowering_inputs();
	let Some(key) = key(identity, classes_early, &inputs) else { return lower() };
	if let Some(kept) = kept(&key) {
		kept.effects.apply();
		return Ok(kept.module);
	}
	let (said, fresh_before) = (crate::errors::diagnostic::said(), fresh_number());
	let outer_lookups = LOOKUPS.replace(Some(vec![]));
	let lowered = lower();
	let lookups = LOOKUPS.replace(outer_lookups).unwrap_or_default();
	let quiet = crate::errors::diagnostic::said() == said && fresh_number() == fresh_before;
	let same_inputs = lowering_inputs() == inputs;
	if !same_inputs {
		log::warn!("a module's lowering changed what lowering_inputs names: keep that field in LoweredEffects");
	}
	if let Ok(module) = &lowered {
		let unkept = unkept_payload(module);
		if let Some(payload) = &unkept {
			log::debug!("a lowered module holds a {payload}, which has no PayloadCodec: not kept");
		}
		if quiet && same_inputs && unkept.is_none() {
			keep(&key, &Kept { lookups, module: module.clone(), effects: LoweredEffects::now() });
		}
	}
	lowered
}

/// Fresh names are numbered by the program around the module, so a lowering that numbers some is not kept
fn fresh_number() -> usize {
	crate::compiler::compile_context::read(|context| context.fresh_number)
}

/// The type of a Data payload in the module (also in its metadata) that serde does not keep (meta.rs PayloadCodec)
fn unkept_payload(node: &Node) -> Option<String> {
	match node {
		Node::Data(value) => (!value.is_kept_payload()).then(|| value.type_name.clone()),
		Node::Meta { node, data } => unkept_payload(node).or_else(|| unkept_payload(data)),
		Node::Key(left, _, right) | Node::Type { name: left, body: right } => {
			unkept_payload(left).or_else(|| unkept_payload(right))
		}
		Node::List(items, ..) => items.iter().find_map(unkept_payload),
		Node::Error(inner) => unkept_payload(inner),
		_ => None,
	}
}

fn file_stamp(file: Option<PathBuf>) -> Option<String> {
	let file = file?;
	let metadata = std::fs::metadata(&file).ok();
	let stamp = metadata.map(|metadata| (metadata.len(), metadata.modified().ok()));
	Some(format!("{}\n{stamp:?}", file.display()))
}

/// The kept lowering for `key`, when the module files its lookups found are still those
fn kept(key: &str) -> Option<Kept> {
	let kept: Kept = postcard::from_bytes(&kept_bytes(key)?).ok()?;
	let loader = program_loader();
	let same_files = kept.lookups.iter().all(|(name, found)| file_stamp(module_file(&loader, name)) == *found);
	same_files.then_some(kept)
}

#[cfg(feature = "native")]
fn kept_bytes(key: &str) -> Option<Vec<u8>> {
	crate::run::module_cache::kept_bytes(key, LOWERED_EXTENSION)
}

#[cfg(not(feature = "native"))]
fn kept_bytes(_: &str) -> Option<Vec<u8>> {
	None
}

#[cfg(feature = "native")]
fn keep(key: &str, kept: &Kept) {
	if let Ok(bytes) = postcard::to_allocvec(kept) {
		crate::run::module_cache::keep_bytes(key, LOWERED_EXTENSION, &bytes);
	}
}

#[cfg(not(feature = "native"))]
fn keep(_: &str, _: &Kept) {}

/// The binary, the module's code, whether its classes went in front of the program and what its passes read
#[cfg(feature = "native")]
fn key(identity: &[u8], classes_early: bool, inputs: &str) -> Option<String> {
	use sha2::{Digest, Sha256};
	let mut hasher = Sha256::new();
	hasher.update(binary_stamp()?);
	hasher.update(format!("{classes_early}\n{inputs}\n{:?}\n", LoweredEffects::now()));
	hasher.update(identity);
	Some(hasher.finalize().iter().map(|byte| format!("{byte:02x}")).collect())
}

#[cfg(not(feature = "native"))]
fn key(_: &[u8], _: bool, _: &str) -> Option<String> {
	None
}
