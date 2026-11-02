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
// the compiler reads the standard modules of this lib/ (core/parsing/parser.warp warp_lib), as the native warp tells it
const COMPILER_ENVIRONMENT = [...ENVIRONMENT, `WARP_LIB=${COMPILER_LIBRARY}`];
// a run of `warp test` (worker.js valueAsData): the compiler makes _start print main's value as data after a mark
// (core/emitter/emitter.warp VALUE_MARK), which the native verdicts read (core/commands/tests.warp run_result), and
// says no hints, as core/native/tests.warp has its runs
const TEST_RUN_ENVIRONMENT = ["WARP_VALUE_AS_DATA=1", "WARP_HINTS=0"];
const compilerEnvironment = () => self.valueAsData ? [...COMPILER_ENVIRONMENT, ...TEST_RUN_ENVIRONMENT] : COMPILER_ENVIRONMENT;
// a program runs as `warp <file>` runs it (core/native/warp.warp), which names the program (lib/storage.warp keeps its
// tables beside it) and serves its routes: here the page's requests (lib/web.warp)
const programEnvironment = file => [...ENVIRONMENT, "WARP_SERVE=page", `WARP_PROGRAM=${file}`];
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

// a module's entry run: its exit status, 0 when it returned (exitCode: the code its host word exit ended it with); a
// trap's sentence to stderr. A program exporting show runs main, and its value goes to `shown` as the text _start would
// print (the page's value line, apart from what it printed); else _start, else a primitive module's main
function ranToEnd(exports, output, exitCode = () => undefined, shown) {
	try {
		if (shown && exports.show && exports.main) {
			const value = exports.main();
			shown(shownValue(exports, value), value);
		}
		else if (exports._start) exports._start();
		else printedResult(exports.main(), output);
		return 0;
	} catch (failure) {
		if (failure instanceof WasiExit) return failure.code;
		if (exitCode() !== undefined) return exitCode();
		output(`${raisedMessage(exports, failure)}\n`, STDERR);
		return FAILURE_STATUS;
	}
}

// main's value as _start prints it (core/emitter/emitter.warp: show), "" for ø, which _start leaves unprinted
const shownValue = (exports, value) => value === null ? "" : textOfArray(exports.show(value));

// a $text (type 0 of every module the compiler writes, `(array (mut i8))` in no rec group) read from JavaScript by a
// module of that one type, which wasm GC takes for the same type as the program's:
//   (module (type $text (array (mut i8)))
//     (func (export "length") (param (ref null $text)) (result i32) local.get 0 array.len)
//     (func (export "byte") (param (ref null $text) i32) (result i32) local.get 0 local.get 1 array.get_u $text))
const TEXT_READER = "AGFzbQEAAAABEQNeeAFgAWMAAX9gAmMAfwF/AwMCAQIHEQIGbGVuZ3RoAAAEYnl0ZQABChICBgAgAPsPCwkAIAAgAfsNAAsADgRuYW1lBAcBAAR0ZXh0";
let textReader;
function textOfArray(text) {
	textReader ??= new WebAssembly.Instance(new WebAssembly.Module(Uint8Array.from(atob(TEXT_READER), letter => letter.charCodeAt(0)))).exports;
	const bytes = new Uint8Array(textReader.length(text));
	for (let index = 0; index < bytes.length; index++) bytes[index] = textReader.byte(text, index);
	return utf8Decoder.decode(bytes);
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

// parse_code(t) (driver.warp code_parsed): warp.wasm parses t (its parse-code command, the compiler's own parser) and
// the tree it encodes is built again in the program, node by node through its builders
const PARSED_CODE_PATH = "/tmp/warp-parsed.code";
const FIELD_END = 1; // driver.warp FIELD_END
const ENCODED = { empty: 101, yes: 84, no: 70, int: 105, float: 102, codepoint: 99, symbol: 115, error: 120, key: 107, list: 108 };
function codeParsed(program, code) {
	let said = "";
	const { status } = compilerRan(["parse-code", guestText(program, code), PARSED_CODE_PATH], text => said += text);
	const encoding = programFiles.written.get(PARSED_CODE_PATH);
	programFiles.written.delete(PARSED_CODE_PATH);
	if (status !== 0 || !encoding) return newGuestError(program, said.trim() || `parse-code wrote nothing (status ${status})`);
	return codeBuilt(program, typeof encoding === "string" ? utf8.encode(encoding) : encoding, { at: 0 });
}
// the field at `cursor.at`, up to its FIELD_END, the cursor past it
function encodedField(encoding, cursor) {
	const end = encoding.indexOf(FIELD_END, cursor.at);
	const field = utf8Decoder.decode(encoding.subarray(cursor.at, end));
	cursor.at = end + 1;
	return field;
}
// the node encoded at the cursor, built in the program (driver.warp code_built)
function codeBuilt(program, encoding, cursor) {
	const tag = encoding[cursor.at++];
	if (tag === ENCODED.empty) return program.new_empty();
	if (tag === ENCODED.yes || tag === ENCODED.no) return program.new_bool(tag === ENCODED.yes ? 1n : 0n);
	const field = encodedField(encoding, cursor);
	if (tag === ENCODED.int) return program.new_int(BigInt(field));
	if (tag === ENCODED.float) return program.new_float(Number(field));
	if (tag === ENCODED.codepoint) return program.new_codepoint(Number(field));
	if (tag === ENCODED.symbol) return newGuestBytes(program, field, "new_symbol");
	if (tag === ENCODED.error) return newGuestError(program, field);
	if (tag === ENCODED.key) {
		const key = codeBuilt(program, encoding, cursor);
		return program.new_spelled_key(key, newGuestText(program, field), codeBuilt(program, encoding, cursor));
	}
	if (tag === ENCODED.list) {
		const [bracket, separator] = [encodedField(encoding, cursor), encodedField(encoding, cursor)];
		const items = Array.from({ length: Number(field) }, () => codeBuilt(program, encoding, cursor));
		const cells = items.reduceRight((rest, item) => program.new_list(item, rest, SQUARE_BRACKET), null);
		return program.new_spelled_list(cells, newGuestText(program, bracket), newGuestText(program, separator));
	}
	return newGuestText(program, field);
}

// a timer's firing (lib/timers.warp): the program's timer_fired(id), what its block prints going out as it goes;
// { value } the program's value anew by its VALUE_ANEW_EXPORT (main's last expression again, as _start shows it),
// { failure } the sentence its trap or raise says, as a run's (raisedMessage)
const VALUE_ANEW_EXPORT = "page_value";
function timerFired(holder, id) {
	const { exports } = holder;
	try {
		exports.timer_fired(BigInt(id));
		return exports[VALUE_ANEW_EXPORT] ? { value: shownValue(exports, exports[VALUE_ANEW_EXPORT]()) } : {};
	} catch (failure) {
		return { failure: failure instanceof WasiExit ? `the timer's block exited (status ${failure.code})` : raisedMessage(exports, failure) };
	}
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

// what a run said on stderr, printed when it ended well, its failure when not, and its value when `valued`:
// { status, said, value }
function ranSaying(exports, print, exitCode, valued = false) {
	let said = "", value = "", result;
	const status = ranToEnd(exports, (text, stream) => stream === STDERR ? said += text : print(text, stream), exitCode, valued ? (shown, node) => { value = shown; result = node; } : undefined);
	if (status === 0 && said) print(said, STDERR);
	return { status, said: said.trimEnd(), value, result };
}

// warp.wasm run with the words after its own path (driver.warp file_ran), each run a fresh instance: { status, said }
function compilerRan(words, print) {
	let exports;
	const wasi = wasiImports(() => exports.memory, [COMPILER_LIBRARY + "/warp.wasm", ...words], print, programFiles, compilerEnvironment());
	exports = new WebAssembly.Instance(compilerModule, withWasi(compilerHostWords(() => exports), wasi)).exports;
	return ranSaying(exports, print);
}

// warp.wasm compiles the file: { module } the bytes it wrote, else { failure } what it said
function compiledProgram(file, print) {
	const { status, said } = compilerRan([file, COMPILED_PATH, folderOf(file), COMPILER_LIBRARY, `${COMPILER_LIBRARY}/runtime.wasm`], print);
	const module = programFiles.written.get(COMPILED_PATH);
	programFiles.written.delete(COMPILED_PATH);
	if (status !== 0 || !module?.length) return { failure: said || `warp.wasm wrote no module of ${file} (status ${status})` };
	return { module };
}

// the compiled program run with the page's host words: { status, said }
function programRan(bytes, file, hooks) {
	const holder = { warnings: [], pagePath: hooks.pagePath?.(), hooks };
	const wasi = wasiImports(() => holder.exports.memory, [file, ...(self.programArguments ?? [])], hooks.print, programFiles, programEnvironment(file));
	try {
		const module = new WebAssembly.Module(bytes);
		holder.run = { module, bytes };
		self.joinVoice?.();
		eachHostPart("started", holder.run);
		holder.exports = new WebAssembly.Instance(module, withWasi(withRuntime(module, programImports(holder, hooks)), wasi)).exports;
	} catch (failure) {
		return { status: FAILURE_STATUS, said: String(failure.message ?? failure) };
	}
	const ran = ranSaying(holder.exports, hooks.print, () => holder.exitCode, !self.valueAsData);
	eachHostPart("finished", holder, hooks);
	holder.value = ran.result;
	return { ...ran, holder };
}

// the code compiled and run as `warp <file>` runs it: what the page shows of it (playground.js showReport), a failure's
// message as its value; what it printed went to hooks.print meanwhile
function evaluatedNatively(code, file, hooks) {
	const source = file || TYPED_FILE;
	return withSource(code, source, () => {
		const compiled = compiledProgram(source, hooks.print);
		if (compiled.failure) return report(compiled.failure, FAILURE_STATUS);
		hooks.module?.(compiled.module);
		const { status, said, value, holder } = programRan(compiled.module, source, hooks);
		const html = status === 0 ? pageHtml(holder) : undefined;
		const events = html !== undefined && holder.exports.page_event ? pageEventsOf(html) : [];
		// a page listens after main for its markup's events and its timers (lib/timers.warp, host-timers.js)
		if (status === 0 && (events.length || holder.timers?.length)) hooks.listen?.(holder, events);
		return { ...report(status === 0 ? value : said, status), html };
	});
}

// ---- a page of markup (lib/markup.warp, card markup-keeps): page_render(value) is the HTML of main's value, each
// handler numbered anew from 1 as data-warp-<event>="<n>"; page_event(n, detail) runs handler n of the last render

// the HTML of the run's value by its page_render, undefined for a program without one
function pageHtml(holder) {
	const render = holder?.exports.page_render;
	return render && holder.value != null ? guestText(holder.exports, render(holder.value)) : undefined;
}

// the markup's attributes of data-warp-… that name no handler (markup.js)
const MARKUP_DATA_ATTRIBUTES = ["key", "instance", "leaving", "scope"];
const HANDLER_ATTRIBUTE = /\bdata-warp-(\w+)="(\d+)"/g;
// the element events the HTML has handlers for, as the page names them: click·1, input·2
const pageEventsOf = html => [...html.matchAll(HANDLER_ATTRIBUTE)].filter(([, event]) => !MARKUP_DATA_ATTRIBUTES.includes(event)).map(([, event, n]) => `${event}·${n}`);

// an element event (click·1) of the page: its handler run with the event's detail, then the page rendered anew:
// { value, html, events } or { value: the failure, error }
function pageEventHandled(holder, event, detail) {
	const { exports } = holder;
	const n = BigInt(event.slice(event.lastIndexOf("·") + 1));
	try {
		const given = newGuestValue(exports, detail ?? null);
		try {
			exports.page_event(n, given);
		} catch (failure) {
			if (!(failure instanceof TypeError)) throw failure; // n is a Node parameter: the call never started
			exports.page_event(exports.new_int(n), given);
		}
		const html = pageHtml(holder);
		return { value: shownValue(exports, holder.value), html, events: pageEventsOf(html) };
	} catch (failure) {
		return { value: raisedMessage(exports, failure), error: true };
	}
}

const report = (value, status) => ({ value, error: status !== 0 && value !== "", status, warnings: [], runtime_warnings: [], hints: [], notes: [] });
