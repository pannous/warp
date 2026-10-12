// std_pure / std_io(module, member, arguments): a word of lib/<module>.warp (src/std_adapters.rs), by the adapters the
// parts add (host-hashes.js hash, json and regex; host-files.js file, net, store, os and table; host-sound.js sound and midi). It starts both parts, so it
// holds function declarations only: a site loading both gets it twice (card web-bundle-growth)

// an adapter reports a runtime warning with this.warn(message) (host-files.js openTable) and reaches its run's state as
// this.holder (host-sound.js sound.render)
function stdCall(program_, module, member, argumentList, holder) {
	const [moduleName, memberName, given] = [module, member, argumentList].map(node => plainOfTree(readNode(program_, node)));
	const adapter = STD_ADAPTERS[moduleName]?.[memberName];
	if (!adapter) throw new Error(`${moduleName}.${memberName}: no such word in the browser`);
	try {
		const context = { warn: message => holder.warnings.push(message), holder };
		return buildValue(program_, treeOfPlain(adapter.apply(context, Array.isArray(given) ? given : [given])));
	} catch (error) {
		// the program raises it (src/wasm_emitter/ffi_emitter.rs emit_ffi_result): `try` catches it
		return errorInProgram(program_, `${moduleName}.${memberName}: ${error.message}`);
	}
}
