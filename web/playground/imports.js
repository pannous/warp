// The {module, name, kind} of each import a module's bytes declare, read from its import section. Not
// WebAssembly.Module.imports: Safari 27 throws "unable to produce import descriptors" for any module importing a
// function with a GC reference (anyref, eqref) in its signature, as the task words do (card task-sample).
// Shared by the playground's workers, the host parts that need it (src/site.rs prepends it to them) and the uniscript
// page (web/uniscript/build.sh copies it): one function declaration and nothing else, as a site with two such parts
// declares it twice, which a top-level const could not
function importDescriptors(bytes) {
	const WASM_HEADER_BYTES = 8;
	const IMPORT_SECTION = 2;
	const IMPORT_KINDS = ["function", "table", "memory", "global", "tag"]; // the kinds of WebAssembly.Module.imports, by their byte
	const REFERENCE_WITH_HEAP_TYPE = [0x63, 0x64]; // (ref null ht), (ref ht): a heap type follows
	const LIMITS_HAVE_MAXIMUM = 1;
	const view = new Uint8Array(bytes);
	let at = WASM_HEADER_BYTES;
	const number = () => {
		let value = 0, shift = 0, byte;
		do { byte = view[at++]; value += (byte & 0x7F) * 2 ** shift; shift += 7; } while (byte & 0x80);
		return value;
	};
	const name = () => { const length = number(); return new TextDecoder().decode(view.subarray(at, at += length)); };
	const valueType = () => { if (REFERENCE_WITH_HEAP_TYPE.includes(view[at++])) number(); };
	const limits = () => { const flags = view[at++]; number(); if (flags & LIMITS_HAVE_MAXIMUM) number(); };
	const skipDescription = {
		function: number,
		table: () => { valueType(); limits(); },
		memory: limits,
		global: () => { valueType(); at++; },
		tag: () => { at++; number(); },
	};
	while (at < view.length) {
		const section = view[at++], size = number(), end = at + size;
		if (section !== IMPORT_SECTION) { at = end; continue; }
		return Array.from({ length: number() }, () => {
			const entry = { module: name(), name: name(), kind: IMPORT_KINDS[view[at++]] };
			skipDescription[entry.kind]();
			return entry;
		});
	}
	return [];
}
