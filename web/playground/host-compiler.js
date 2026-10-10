// The warp compiler in the page (a part of host.js, which says how parts work; needs host-files.js): the warp_host
// imports of a compiler instance (warpHost, for the playground's workers), and the blocks a program builds at run time
// (run_block), compiled by a compiler instance of their own

// a page event in the last listening run: the page value its handler leaves, else the handler's own outcome (worker.js
// showHandled shows the same)
function pageEventOutcome(hooks, event, detail) {
	if (!listeningRun) return { failure: `no running page handles ${event}` };
	const handled = runPageEvent(listeningRun, hooks, event, detail);
	const binding = listeningRun.exports[PAGE_VALUE_EXPORT];
	return handled.result && binding ? outcomeOf(listeningRun, hooks, binding) : handled;
}

// The compiler that runs the blocks a program builds at run time (run_block): loaded on first use, an instance of its own,
// so it never re-enters the compiler whose program is running. A page sets BLOCK_COMPILER_URL to its compiler's URL, or
// BLOCK_COMPILER to a function giving the exports of a new compiler instance (test-worker.js: the test binary itself).
let blockCompilerExports;
function blockCompiler(hooks) {
	blockCompilerExports ??= self.BLOCK_COMPILER ? self.BLOCK_COMPILER(hooks) : compilerFromUrl(hooks);
	return blockCompilerExports;
}

// the next block gets a new compiler instance: after a trap the old one is unusable
function forgetBlockCompiler() {
	blockCompilerExports = undefined;
}

function compilerFromUrl(hooks) {
	const url = self.BLOCK_COMPILER_URL ?? "warp.wasm";
	let bytes;
	try {
		bytes = getSync(url, undefined, true);
	} catch (failure) {
		throw new Error(`a block known only at run time needs the warp compiler ${url} (${failure.message}): build it with web/playground/build.sh`);
	}
	let exports;
	exports = new WebAssembly.Instance(new WebAssembly.Module(bytes), { warp_host: warpHost(() => exports.memory, hooks) }).exports;
	return exports;
}

// a compiler entry point whose runs may wait for promises (warpHost `awaited`), as an async function where JSPI is
const awaitedEntry = entry => JSPI ? WebAssembly.promising(entry) : entry;

// the report of src/web.rs eval_block_report: {result: tree} or {error: message}
function evalBlock(hooks, request) {
	const compiler = blockCompiler(hooks);
	const bytes = utf8.encode(request);
	const pointer = compiler.web_alloc(bytes.length);
	new Uint8Array(compiler.memory.buffer, pointer, bytes.length).set(bytes);
	let length;
	try {
		length = compiler.web_eval_block(pointer, bytes.length);
	} catch (trap) {
		forgetBlockCompiler();
		throw trap;
	}
	const report = JSON.parse(readText(compiler, compiler.web_report(), length));
	compiler.web_free(pointer, bytes.length);
	return report;
}

// the `warp_host` imports of a compiler instance; `memory()` is its memory (known only after instantiation).
// `awaited`: the instance's entry points are called through WebAssembly.promising (host.js JSPI, awaitedEntry), so a run
// may wait for its program's promises, the compiler suspended meanwhile
function warpHost(memory, hooks, awaited = false) {
	let pendingOutcome; // the JSON a run left for warp_host.take
	let pendingFetched; // the text a fetch left for warp_host.take_fetched
	const run = (pointer, length) => {
		const bytes = new Uint8Array(memory().buffer, pointer, length).slice();
		hooks.module?.(bytes);
		return whenSettled(runProgram(bytes, hooks, awaited), outcome => {
			pendingOutcome = utf8.encode(JSON.stringify(outcome));
			return pendingOutcome.length;
		});
	};
	return {
		run: awaited && JSPI ? new WebAssembly.Suspending(run) : run,
		take: into => new Uint8Array(memory().buffer, into, pendingOutcome.length).set(pendingOutcome),
		page_event: (eventPointer, eventLength, detailPointer, detailLength) => {
			const text = (pointer, length) => utf8Decoder.decode(new Uint8Array(memory().buffer, pointer, length));
			pendingOutcome = utf8.encode(JSON.stringify(pageEventOutcome(hooks, text(eventPointer, eventLength), JSON.parse(text(detailPointer, detailLength)))));
			return pendingOutcome.length;
		},
		now_ms: () => Date.now(),
		panicked: (pointer, length) => hooks.panicked(utf8Decoder.decode(new Uint8Array(memory().buffer, pointer, length))),
		// a file for the compiler (module, package source, C header): a path of the served repository, of the page, or a URL
		fetch: (pointer, length) => {
			const address = utf8Decoder.decode(new Uint8Array(memory().buffer, pointer, length));
			try {
				pendingFetched = readBytes(address);
				return pendingFetched.length;
			} catch {
				return -1;
			}
		},
		take_fetched: into => new Uint8Array(memory().buffer, into, pendingFetched.length).set(pendingFetched),
	};
}

addHostPart({
	words: (holder, hooks, { program }) => ({
		// a block known only at run time (src/host.rs run_block): a compiler instance of its own compiles and runs it
		run_block: (block, names, values, definitions) => {
			const module = program();
			const request = { block: readNode(module, block), names: readNode(module, names), values: readNode(module, values), definitions: readNode(module, definitions) };
			const report = evalBlock(hooks, JSON.stringify(request));
			if (report.error !== undefined) {
				holder.blockError = report.error;
				throw new Error(report.error);
			}
			return buildValue(module, report.result);
		},
	}),
});
