// The warp compiler (warp.wasm, built by build.sh) and the programs it compiles, run off the page's thread: a worker may
// compile any module synchronously and block on a synchronous fetch, which the compiler's host calls need.
// The imports of a compiled program mirror what the CLI's wasmtime runner links (src/host.rs, WASI fd_write, libm).

importScripts("reader.js");

const COMPILER_URL = "warp.wasm";
const TEXT_HEAP_EXPORT = "text_heap";
const TRAP_DETAIL_EXPORT = "trap_detail";
const PAGE_BITS = 16;
const STDERR = 2;
/// where host.read finds files: relative to the repository root that the static server serves (README of web/playground)
const FILE_ROOT = "../../";
/// C names of libm functions (ffi "m") that Math spells differently
const LIBM = { fabs: Math.abs, fmin: Math.min, fmax: Math.max, fmod: (a, b) => a % b, ceil: Math.ceil, floor: Math.floor };

const utf8 = new TextEncoder();
let compiler; // the compiler instance's exports
let pendingOutcome; // the JSON a run left for the compiler's warp_host.take
let panicMessage; // the compiler's last panic message
let output; // what the running evaluation reports to the page: printed text, runtime warnings

const post = message => self.postMessage(message);
const print = (text, stream) => post({ type: "print", text, stream });

// ---- the program's imports ----------------------------------------------------------------------------------

// copy bytes into the program's memory from its text heap, the rule of src/host.rs write_bytes_to_caller
function writeBytes(program, bytes) {
	const heap = program[TEXT_HEAP_EXPORT];
	const memoryEnd = program.memory.buffer.byteLength;
	let pointer = heap ? heap.value : 0;
	if (pointer === 0 || pointer + bytes.length > memoryEnd) {
		program.memory.grow((bytes.length >> PAGE_BITS) + 1);
		pointer = memoryEnd;
	}
	new Uint8Array(program.memory.buffer, pointer, bytes.length).set(bytes);
	if (heap) heap.value = pointer + bytes.length;
	return [pointer, bytes.length];
}

// a synchronous GET (host calls are synchronous); `timeout` in ms
function getSync(url, timeout) {
	const request = new XMLHttpRequest();
	request.open("GET", url, false);
	if (timeout) request.timeout = timeout;
	request.send();
	if (request.status >= 400 || request.status === 0) throw new Error(request.status ? `HTTP status ${request.status}` : "network error (blocked by CORS?)");
	return request.responseText;
}

// the body of a host call as (pointer, length), or (pointer, -length) of the failure reason, like src/host.rs
function hostResult(program, action, what) {
	try {
		let text = action();
		if (!text.endsWith("\n")) text += "\n"; // wasp convention (src/host.rs fetch)
		return writeBytes(program, utf8.encode(text));
	} catch (reason) {
		const [pointer, length] = writeBytes(program, utf8.encode(`${what} failed: ${reason.message ?? reason}`));
		return [pointer, -length];
	}
}

function programImports(instance) {
	const program = () => instance.exports;
	const text = (pointer, length) => readText(program(), pointer, length);
	const fetchUrl = (pointer, length, timeout) => {
		const url = text(pointer, length);
		return hostResult(program(), () => getSync(url, timeout), `fetch ${url}`);
	};
	const known = {
		host: {
			fetch: (pointer, length) => fetchUrl(pointer, length),
			fetch_within: (pointer, length, milliseconds) => fetchUrl(pointer, length, Number(milliseconds)),
			read: (pointer, length) => {
				const path = text(pointer, length);
				return hostResult(program(), () => getSync(FILE_ROOT + path), `read ${path}`);
			},
			warn: (pointer, length) => {
				const message = text(pointer, length);
				output.runtimeWarnings.push(message);
				print(`warning: ${message}\n`, STDERR);
			},
			run: () => -1n,
		},
		wasi_snapshot_preview1: {
			fd_write: (fd, vectors, count, written) => {
				const memory = new DataView(program().memory.buffer);
				let total = 0;
				for (let index = 0; index < count; index++) {
					const pointer = memory.getUint32(vectors + 8 * index, true);
					const length = memory.getUint32(vectors + 8 * index + 4, true);
					print(text(pointer, length), fd);
					total += length;
				}
				memory.setUint32(written, total, true);
				return 0;
			},
		},
		m: new Proxy(LIBM, { get: (libm, name) => libm[name] ?? Math[name] }),
	};
	// anything else (native FFI libraries) is missing in the browser: say which, when the program calls it
	const missing = (module, name) => () => { throw new Error(`${module}.${name} is not available in the browser`); };
	return new Proxy(known, {
		get: (modules, module) => new Proxy(modules[module] ?? {}, {
			get: (functions, name) => functions[name] ?? missing(module, String(name)),
		}),
	});
}

// run a compiled program: the outcome src/web.rs run_outcome reads
function runProgram(bytes) {
	let instance;
	try {
		const module = new WebAssembly.Module(bytes);
		const holder = {};
		instance = new WebAssembly.Instance(module, programImports(holder));
		holder.exports = instance.exports;
	} catch (failure) {
		return { failure: String(failure.message ?? failure) };
	}
	try {
		return { result: readResult(instance.exports, instance.exports.main()) };
	} catch (trap) {
		if (!(trap instanceof WebAssembly.RuntimeError || trap instanceof RangeError)) return { failure: String(trap.message ?? trap) };
		const detail = instance.exports[TRAP_DETAIL_EXPORT]?.value;
		return { trap: trap.message, trace: trap.stack ?? "", detail: detail ? readNode(instance.exports, detail) : null };
	}
}

// ---- the compiler -------------------------------------------------------------------------------------------

function compilerText(pointer, length) {
	return readText(compiler, pointer, length);
}

function compilerImports() {
	return {
		warp_host: {
			run: (pointer, length) => {
				const bytes = new Uint8Array(compiler.memory.buffer, pointer, length).slice();
				post({ type: "module", bytes });
				pendingOutcome = utf8.encode(JSON.stringify(runProgram(bytes)));
				return pendingOutcome.length;
			},
			take: into => new Uint8Array(compiler.memory.buffer, into, pendingOutcome.length).set(pendingOutcome),
			now_ms: () => Date.now(),
			panicked: (pointer, length) => { panicMessage = compilerText(pointer, length); },
		},
	};
}

async function loadCompiler() {
	const response = await fetch(COMPILER_URL);
	if (!response.ok) throw new Error(`${COMPILER_URL}: HTTP ${response.status}; build it with web/playground/build.sh`);
	const { instance } = await WebAssembly.instantiate(await response.arrayBuffer(), compilerImports());
	compiler = instance.exports;
}

function passText(text) {
	const bytes = utf8.encode(text);
	const pointer = compiler.web_alloc(bytes.length);
	new Uint8Array(compiler.memory.buffer, pointer, bytes.length).set(bytes);
	return [pointer, bytes.length];
}

function evaluate(code, answers) {
	output = { runtimeWarnings: [] };
	panicMessage = undefined;
	const codeText = passText(code);
	const answersText = passText(JSON.stringify(answers));
	try {
		const length = compiler.web_evaluate(...codeText, ...answersText);
		const report = JSON.parse(compilerText(compiler.web_report(), length));
		report.runtime_warnings.push(...output.runtimeWarnings);
		compiler.web_free(...codeText);
		compiler.web_free(...answersText);
		return report;
	} catch (crash) {
		compiler = undefined; // a panic leaves the compiler's memory in an unknown state: load it again
		return { value: `compiler crashed: ${panicMessage ?? crash.message}`, error: true, crashed: true, warnings: [], runtime_warnings: [], hints: [], asks: [], notes: [] };
	}
}

const ready = loadCompiler().then(() => post({ type: "ready" }), failure => post({ type: "failed", message: failure.message }));

self.onmessage = async ({ data }) => {
	await ready;
	if (!compiler) await loadCompiler();
	const started = performance.now();
	const report = evaluate(data.code, data.answers ?? {});
	post({ type: "report", id: data.id, report, milliseconds: performance.now() - started });
};
