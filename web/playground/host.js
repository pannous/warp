// What a host gives the warp compiler built without the native feature (src/web.rs `page`): `warp_host` runs the modules
// it compiles. A compiled program's imports mirror what the CLI's wasmtime runner links (src/host.rs, WASI fd_write,
// libm). Shared by the playground (worker.js) and the browser test runner (test-worker.js); needs reader.js.

const TEXT_HEAP_EXPORT = "text_heap";
const TRAP_DETAIL_EXPORT = "trap_detail";
const PAGE_BITS = 16;
const STDERR = 2;
/// where host.read and the test runner's file system find files: the repository root the static server serves
const FILE_ROOT = new URL("../../", self.location.href).href;
/// C names of libm functions (ffi "m") that Math spells differently
const LIBM = { fabs: Math.abs, fmin: Math.min, fmax: Math.max, fmod: (a, b) => a % b, ceil: Math.ceil, floor: Math.floor };

const HTTP_NOT_FOUND = "HTTP status 404";
const FILE_NOT_FOUND = "No such file or directory (os error 2)";

const utf8 = new TextEncoder();

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

// a synchronous GET (host calls are synchronous, so this runs in a worker); `timeout` in ms
function getSync(url, timeout, binary = false) {
	const request = new XMLHttpRequest();
	request.open("GET", url, false);
	if (timeout) request.timeout = timeout;
	if (binary) request.overrideMimeType("text/plain; charset=x-user-defined"); // bytes as they are
	request.send();
	if (request.status >= 400 || request.status === 0) throw new Error(request.status ? `HTTP status ${request.status}` : "network error (blocked by CORS?)");
	return binary ? Uint8Array.from(request.responseText, character => character.charCodeAt(0) & 0xff) : request.responseText;
}

// a file of the served repository, failing in the words of the native read (src/host.rs)
function readFile(path) {
	try {
		return getSync(FILE_ROOT + path);
	} catch (failure) {
		throw failure.message === HTTP_NOT_FOUND ? new Error(FILE_NOT_FOUND) : failure;
	}
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

// hooks: print(text, fd), module(bytes) (each compiled module), panicked(message) (the compiler's)
function programImports(holder, hooks) {
	const program = () => holder.exports;
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
				return hostResult(program(), () => readFile(path), `read ${path}`);
			},
			warn: (pointer, length) => holder.warnings.push(text(pointer, length)),
			run: () => -1n,
			// the host words (src/host.rs): a page cannot block, so sleep busy-waits
			sleep: milliseconds => {
				const until = Date.now() + Number(milliseconds);
				while (Date.now() < until);
			},
			random: () => Math.random(),
			random_below: bound => bound > 0n ? BigInt(Math.floor(Math.random() * Number(bound))) : 0n,
			clock: () => BigInt(Date.now()),
		},
		wasi_snapshot_preview1: {
			fd_write: (fd, vectors, count, written) => {
				const memory = new DataView(program().memory.buffer);
				let total = 0;
				for (let index = 0; index < count; index++) {
					const pointer = memory.getUint32(vectors + 8 * index, true);
					const length = memory.getUint32(vectors + 8 * index + 4, true);
					hooks.print(text(pointer, length), fd);
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
function runProgram(bytes, hooks) {
	let instance;
	const holder = { warnings: [] }; // the runtime warnings go back to the compiler, which reports them (src/web.rs)
	try {
		instance = new WebAssembly.Instance(new WebAssembly.Module(bytes), programImports(holder, hooks));
		holder.exports = instance.exports;
	} catch (failure) {
		return { failure: String(failure.message ?? failure) };
	}
	const { warnings } = holder;
	try {
		return { result: readResult(instance.exports, instance.exports.main()), warnings };
	} catch (trap) {
		if (!(trap instanceof WebAssembly.RuntimeError || trap instanceof RangeError)) return { failure: String(trap.message ?? trap), warnings };
		const detail = instance.exports[TRAP_DETAIL_EXPORT]?.value;
		return { trap: trap.message, trace: trap.stack ?? "", detail: detail ? readNode(instance.exports, detail) : null, warnings };
	}
}

// the `warp_host` imports of a compiler instance; `memory()` is its memory (known only after instantiation)
function warpHost(memory, hooks) {
	let pendingOutcome; // the JSON a run left for warp_host.take
	return {
		run: (pointer, length) => {
			const bytes = new Uint8Array(memory().buffer, pointer, length).slice();
			hooks.module?.(bytes);
			pendingOutcome = utf8.encode(JSON.stringify(runProgram(bytes, hooks)));
			return pendingOutcome.length;
		},
		take: into => new Uint8Array(memory().buffer, into, pendingOutcome.length).set(pendingOutcome),
		now_ms: () => Date.now(),
		panicked: (pointer, length) => hooks.panicked(utf8Decoder.decode(new Uint8Array(memory().buffer, pointer, length))),
	};
}
