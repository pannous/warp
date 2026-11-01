// The native warp's driver (core/native/driver.warp) in the page's worker: warp.wasm, the self-hosted compiler, compiles
// a program as the native warp has it compile (the file, the module to write, its folder, lib/, runtime.wasm), over
// WASI (wasi.js: the served repository read with synchronous GETs, writes in an overlay); then the module it wrote runs
// with the program's host words (host.js programImports) and the same WASI, its _start printing its own value.
// Needs wasi.js, values.js and host.js.

// the installed compiler's lib/ (beside ~/dev/bin/warp), which build.sh copies into the page; the compiler gets its
// absolute path (wasi.js REPOSITORY_DIRECTORY), as natively, so it is found from any program's folder
const COMPILER_FOLDER = "web/playground/warp-lib";
const COMPILER_LIBRARY = `${REPOSITORY_DIRECTORY}/${COMPILER_FOLDER}`;
const COMPILER_URL = `../../${COMPILER_FOLDER}/warp.wasm`;
const COMPILED_PATH = "/tmp/warp-program.wasm";
// the program typed in the page when it is no sample's file: at the repository root, as `warp playground.warp` there
const TYPED_FILE = "playground.warp";
const FAILURE_STATUS = 1;
const STDOUT = 1;
const FLOAT_DIGITS = 17; // driver.warp FLOAT_DIGITS

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
		if (exports._start) exports._start();
		else printedResult(exports.main(), output);
		return 0;
	} catch (failure) {
		if (failure instanceof WasiExit) return failure.code;
		if (exitCode() !== undefined) return exitCode();
		output(`${raisedMessage(exports, failure)}\n`, STDERR);
		return FAILURE_STATUS;
	}
}

// what a primitive module's main gave, printed as its kind is (driver.warp print_result): an int, a float as C's %.17g
function printedResult(result, output) {
	if (typeof result === "bigint" || Number.isInteger(result)) output(`${result}\n`, STDOUT);
	else if (typeof result === "number") output(`${shortestG(result)}\n`, STDOUT);
}

// a float as C's gcvt(x, 17) writes it: 17 significant digits, trailing zeros dropped, an exponent below 1e-4 and from 1e17
function shortestG(x) {
	if (!Number.isFinite(x)) return Number.isNaN(x) ? "nan" : x > 0 ? "inf" : "-inf";
	const exponent = x === 0 ? 0 : Math.floor(Math.log10(Math.abs(x)));
	const trimmed = digits => digits.includes(".") ? digits.replace(/0+$/, "").replace(/\.$/, "") : digits;
	if (exponent >= -4 && exponent < FLOAT_DIGITS) return trimmed(x.toPrecision(FLOAT_DIGITS));
	const [mantissa, power] = x.toExponential(FLOAT_DIGITS - 1).split("e");
	return `${trimmed(mantissa)}e${power[0]}${power.slice(1).padStart(2, "0")}`;
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

// the files programs write, over the served ones, for the page's life and kept in its IndexedDB (host-files.js
// writtenFiles), as natively they stay on disk from run to run
const programFiles = { ...newOverlay(), written: writtenFiles, kept: (path, bytes) => { keepWrittenFile(path, bytes); self.taskWrote?.(path, bytes); }, forgotten: forgetKeptFile };

// the code as the file while `work` runs, the file's own content back after it
function withSource(code, file, work) {
	const before = programFiles.written.get(file);
	programFiles.written.set(file, utf8.encode(code));
	try {
		return work();
	} finally {
		if (before === undefined) programFiles.written.delete(file);
		else programFiles.written.set(file, before);
	}
}

// what a run said on stderr, printed when it ended well, its failure when not: { status, said }
function ranSaying(exports, print, exitCode) {
	let said = "";
	const status = ranToEnd(exports, (text, stream) => stream === STDERR ? said += text : print(text, stream), exitCode);
	if (status === 0 && said) print(said, STDERR);
	return { status, said: said.trimEnd() };
}

// warp.wasm compiles the file: { module } the bytes it wrote, else { failure } what it said
function compiledProgram(file, print) {
	let exports;
	const args = [COMPILER_LIBRARY + "/warp.wasm", file, COMPILED_PATH, folderOf(file), COMPILER_LIBRARY, `${COMPILER_LIBRARY}/runtime.wasm`];
	const wasi = wasiImports(() => exports.memory, args, print, programFiles);
	exports = new WebAssembly.Instance(compilerModule, withWasi(compilerHostWords(() => exports), wasi)).exports;
	const { status, said } = ranSaying(exports, print);
	const module = programFiles.written.get(COMPILED_PATH);
	programFiles.written.delete(COMPILED_PATH);
	if (status !== 0 || !module?.length) return { failure: said || `warp.wasm wrote no module of ${file} (status ${status})` };
	return { module };
}

// the compiled program run with the page's host words: { status, said }
function programRan(bytes, file, hooks) {
	const holder = { warnings: [], pagePath: hooks.pagePath?.() };
	const wasi = wasiImports(() => holder.exports.memory, [file, ...(self.programArguments ?? [])], hooks.print, programFiles);
	try {
		const module = new WebAssembly.Module(bytes);
		holder.run = { module, bytes };
		self.joinVoice?.();
		eachHostPart("started", holder.run);
		holder.exports = new WebAssembly.Instance(module, withWasi(withRuntime(module, programImports(holder, hooks)), wasi)).exports;
	} catch (failure) {
		return { status: FAILURE_STATUS, said: String(failure.message ?? failure) };
	}
	const ran = ranSaying(holder.exports, hooks.print, () => holder.exitCode);
	eachHostPart("finished", holder, hooks);
	return ran;
}

// the code compiled and run as `warp <file>` runs it: what the page shows of it (playground.js showReport), a failure's
// message as its value; what it printed went to hooks.print meanwhile
function evaluatedNatively(code, file, hooks) {
	const source = file || TYPED_FILE;
	return withSource(code, source, () => {
		const compiled = compiledProgram(source, hooks.print);
		if (compiled.failure) return report(compiled.failure, FAILURE_STATUS);
		hooks.module?.(compiled.module);
		const { status, said } = programRan(compiled.module, source, hooks);
		return report(status === 0 ? "" : said, status);
	});
}

const report = (value, status) => ({ value, error: status !== 0 && value !== "", status, warnings: [], runtime_warnings: [], hints: [], notes: [] });
