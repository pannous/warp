// std_pure / std_io(module, member, arguments): a word of lib/<module>.warp (src/modules/std_adapters.rs), by the adapters the
// parts add (host-hashes.js hash, json and regex; host-files.js file, net, store, os and table; host-sound.js sound and midi). It starts both parts, so it
// holds function declarations only: a site loading both gets it twice (card web-bundle-growth)

// an adapter reports a runtime warning with this.warn(message) (host-files.js openTable), reaches its run's state as
// this.holder (host-sound.js sound.render) and its arguments as read, exact numbers included, as this.tree (host-hashes.js json)
function stdCall(program_, module, member, argumentList, holder) {
	const [moduleName, memberName] = [module, member].map(node => plainOfTree(readNode(program_, node)));
	const givenTree = readNode(program_, argumentList);
	const given = plainOfTree(givenTree);
	const adapter = STD_ADAPTERS[moduleName]?.[memberName];
	if (!adapter) throw new Error(`${moduleName}.${memberName}: no such word in the browser`);
	try {
		const context = { warn: message => holder.warnings.push(message), holder, tree: givenTree };
		return buildValue(program_, treeOfPlain(adapter.apply(context, Array.isArray(given) ? given : [given])));
	} catch (error) {
		// the program raises it (src/wasm_emitter/host/ffi_emitter.rs emit_ffi_result): `try` catches it
		return errorInProgram(program_, `${moduleName}.${memberName}: ${error.message}`);
	}
}
