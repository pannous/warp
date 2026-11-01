// The native warp's driver (core/native/driver.warp) in the page's worker: warp.wasm, the self-hosted compiler, compiles
// a program as the native warp has it compile (the file, the module to write, its folder, lib/, runtime.wasm), over
// WASI (wasi.js: the served repository read with synchronous GETs, writes in an overlay); then the module it wrote runs
// with the program's host words (host.js programImports) and the same WASI, its _start printing its own value.
// Needs wasi.js, values.js and host.js.

// the installed compiler's lib/ (beside ~/dev/bin/warp), which build.sh copies into the page
const COMPILER_LIBRARY = "web/playground/warp-lib";
const COMPILER_URL = `../../${COMPILER_LIBRARY}/warp.wasm`;
const COMPILED_PATH = "/tmp/warp-program.wasm";
// the program typed in the page when it is no sample's file: at the repository root, as `warp playground.warp` there
const TYPED_FILE = "playground.warp";
const FAILURE_STATUS = 1;

let compilerModule; // warp.wasm compiled once; each compile gets a fresh instance, as each native run a fresh store

async function loadDriverCompiler(progress = () => {}) {
	const response = await fetch(COMPILER_URL);
	if (!response.ok) throw new Error(`${COMPILER_URL}: HTTP ${response.status}; build it with web/playground/build.sh`);
	const bytes = await progress(response);
	compilerModule = await WebAssembly.compile(bytes);
}

// the folder of a path, "." for a bare name
const folderOf = path => path.includes("/") ? path.slice(0, path.lastIndexOf("/")) : ".";

// a text the module packed as its address times 2^32 plus its length (runtime/show.warp shown_at), -1 for none
function packedText(exports, packed) {
	if (packed < 0n) return "";
	return utf8Decoder.decode(new Uint8Array(exports.memory.buffer, Number(packed / SHOWN_TEXT_SHIFT), Number(packed % SHOWN_TEXT_SHIFT)).slice());
}

// the WASI module of a run, its other imports as given: the files of the run's overlay, stdout and stderr to `output`
function withWasi(imports, wasi) {
	return new Proxy(imports, { get: (modules, module) => module === "wasi_snapshot_preview1" ? wasi : modules[module] });
}

// a module's entry run (_start, else main): its exit status, 0 when it returned (exitCode: the code its host word exit
// ended it with); a trap's sentence to stderr
function ranToEnd(exports, output, exitCode = () => undefined) {
	try {
		(exports._start ?? exports.main)();
		return 0;
	} catch (failure) {
		if (failure instanceof WasiExit) return failure.code;
		if (exitCode() !== undefined) return exitCode();
		output(`${raisedMessage(exports, failure)}\n`, STDERR);
		return FAILURE_STATUS;
	}
}

// what a run failed in, as every host says it (driver.warp raised_message): the module's trap_message, else the
// engine's own message
function raisedMessage(exports, failure) {
	const sentence = exports.trap_message ? packedText(exports, exports.trap_message(-1n)) : "";
	return sentence || String(failure.message ?? failure);
}

// the host words warp.wasm calls itself (driver.warp answered_word): no window, no system values, paths as given
function compilerHostWords(exports) {
	return {
		host: {
			std_io: (module, member) => newGuestError(exports(), `the host word ${guestText(exports(), module)}.${guestText(exports(), member)} is not in the browser yet`),
			paint: () => { throw new Error("the compiler paints nothing"); },
			real_path: () => -1,
			random: Math.random,
			random_below: below => BigInt(Math.floor(Math.random() * Number(below))),
			clock: () => BigInt(Date.now()),
			system_value: () => 0n,
		},
		// C's round, half away from zero
		m: { round: x => Math.sign(x) * Math.round(Math.abs(x)) },
	};
}

// warp.wasm compiles the code as the file: { module } the bytes it wrote, else { failure } what it said on stderr
function compiledProgram(code, file, overlay, print) {
	const source = file || TYPED_FILE;
	overlay.written.set(source, utf8.encode(code));
	overlay.removed.delete(source);
	let exports;
	let said = "";
	const output = (text, stream) => stream === STDERR ? said += text : print(text, stream);
	const args = [COMPILER_LIBRARY + "/warp.wasm", source, COMPILED_PATH, folderOf(source), COMPILER_LIBRARY, `${COMPILER_LIBRARY}/runtime.wasm`];
	const wasi = wasiImports(() => exports.memory, args, output, overlay);
	exports = new WebAssembly.Instance(compilerModule, withWasi(compilerHostWords(() => exports), wasi)).exports;
	const status = ranToEnd(exports, output);
	const module = overlay.written.get(COMPILED_PATH);
	overlay.written.delete(COMPILED_PATH);
	if (status !== 0 || !module?.length) return { failure: said || `warp.wasm wrote no module of ${source} (status ${status})` };
	if (said) print(said, STDERR);
	return { module };
}

// the compiled program run with the page's host words: { status, failure? }
function programRan(bytes, file, overlay, hooks) {
	const holder = { warnings: [], pagePath: hooks.pagePath?.() };
	const name = file || TYPED_FILE;
	const wasi = wasiImports(() => holder.exports.memory, [name, ...(self.programArguments ?? [])], hooks.print, overlay);
	try {
		const module = new WebAssembly.Module(bytes);
		holder.run = { module, bytes };
		self.joinVoice?.();
		eachHostPart("started", holder.run);
		holder.exports = new WebAssembly.Instance(module, withWasi(withRuntime(module, programImports(holder, hooks)), wasi)).exports;
	} catch (failure) {
		return { status: FAILURE_STATUS, failure: String(failure.message ?? failure) };
	}
	let said = "";
	const status = ranToEnd(holder.exports, (text, stream) => stream === STDERR ? said += text : hooks.print(text, stream), () => holder.exitCode);
	eachHostPart("finished", holder, hooks);
	return { status, failure: said || undefined, holder };
}

// the code compiled and run as `warp <file>` runs it: what the page shows of it (playground.js showReport); what it
// printed went to hooks.print meanwhile
function evaluatedNatively(code, file, hooks) {
	const overlay = newOverlay();
	const compiled = compiledProgram(code, file, overlay, hooks.print);
	if (compiled.failure) return report(compiled.failure.trimEnd(), true);
	hooks.module?.(compiled.module);
	const ran = programRan(compiled.module, file, overlay, hooks);
	return report(ran.failure?.trimEnd() ?? "", ran.failure !== undefined, ran.status);
}

const report = (value, error, status = error ? FAILURE_STATUS : 0) =>
	({ value, error, status, warnings: [], runtime_warnings: [], hints: [], notes: [] });
