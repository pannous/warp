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
/// libc's RAND_MAX on glibc and macOS: rand() is 0…RAND_MAX
const RAND_MAX = 2147483647;
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

// the bytes of a file of the served repository or of a URL, failing in the words of the native read
function readBytes(path) {
	try {
		return getSync(/^https?:/.test(path) ? path : FILE_ROOT + path, 0, true);
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
	const cString = pointer => {
		const bytes = new Uint8Array(program().memory.buffer);
		let end = pointer;
		while (bytes[end] !== 0 && end < bytes.length) end++;
		return bytes.slice(pointer, end);
	};
	const compareBytes = (a, b) => {
		for (let index = 0; index < Math.min(a.length, b.length); index++) {
			if (a[index] !== b[index]) return a[index] - b[index];
		}
		return a.length - b.length;
	};
	const fetchUrl = (pointer, length, timeout) => {
		const url = text(pointer, length);
		return hostResult(program(), () => getSync(url, timeout), `fetch ${url}`);
	};
	const known = {
		host: {
			fetch: (pointer, length) => fetchUrl(pointer, length),
			fetch_within: (pointer, length, milliseconds) => fetchUrl(pointer, length, Number(milliseconds)),
			// the file's bytes as they are, like src/host.rs read; a URL (a package's own files) is fetched
			read: (pointer, length) => {
				const path = text(pointer, length);
				try {
					return writeBytes(program(), readBytes(path));
				} catch (reason) {
					const [failed, failedLength] = writeBytes(program(), utf8.encode(`read ${path} failed: ${reason.message ?? reason}`));
					return [failed, -failedLength];
				}
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
		// the pure part of libc (ffi "c"): numbers, and C strings read up to their zero byte
		c: {
			rand: () => Math.floor(Math.random() * RAND_MAX),
			srand: () => {},
			abs: Math.abs,
			labs: value => value < 0n ? -value : value,
			strlen: pointer => cString(pointer).length,
			strcmp: (a, b) => Math.sign(compareBytes(cString(a), cString(b))),
			atoi: pointer => Math.trunc(parseFloat(new TextDecoder().decode(cString(pointer)))) | 0,
			atof: pointer => parseFloat(new TextDecoder().decode(cString(pointer))) || 0,
		},
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
	let pendingFetched; // the text a fetch left for warp_host.take_fetched
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
		// a module file or package source for the compiler: a path of the served repository, or a URL
		fetch: (pointer, length) => {
			const address = utf8Decoder.decode(new Uint8Array(memory().buffer, pointer, length));
			try {
				pendingFetched = utf8.encode(/^https?:/.test(address) ? getSync(address) : readFile(address.replace(/^\.\//, "")));
				return pendingFetched.length;
			} catch {
				return -1;
			}
		},
		take_fetched: into => new Uint8Array(memory().buffer, into, pendingFetched.length).set(pendingFetched),
	};
}
