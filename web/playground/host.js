// What a host gives the warp compiler built without the native feature (src/web.rs `page`): `warp_host` runs the modules
// it compiles. A compiled program's imports mirror what the CLI's wasmtime runner links (src/host.rs, WASI fd_write,
// libm). Shared by the playground (worker.js) and the browser test runner (test-worker.js); needs reader.js.

const TEXT_HEAP_EXPORT = "text_heap";
const TRAP_DETAIL_EXPORT = "trap_detail";
// the checks of the listeners on shared values (src/lowering/signal_values.rs), run at every check point
const SHARED_HANDLER = "on·shared";
const SHARED_CHECK_MILLISECONDS = 10;
const PAGE_BITS = 16;
const STDERR = 2;
const DARK_MODE_QUERY = "(prefers-color-scheme: dark)";
/// where host.read and the test runner's file system find files: the repository root the static server serves
const FILE_ROOT = new URL("../../", self.location.href).href;
const PAGE_PREFIX = "page:"; // src/web.rs PAGE_PREFIX: a file of the page itself (lib/libc.h), not of the served repository
/// C names of libm functions (ffi "m") that Math spells differently
const LIBC_URL = new URL("lib/libc.wasm", self.location.href).href; // libc for `use c` (lib/build_libc.sh, P147)
const LIBM = { fabs: Math.abs, fmin: Math.min, fmax: Math.max, fmod: (a, b) => a % b, ceil: Math.ceil, floor: Math.floor };

const HTTP_NOT_FOUND = "HTTP status 404";
const MODULE_PATH = /\.(wasm|wat)$/; // src/wasm_modules.rs MODULE_EXTENSIONS
const SETTER_PREFIX = "set "; // src/wasm_modules.rs SETTER_PREFIX: the import that sets a mutable global
const C_CALLS_SECTION = "warp.c_calls"; // src/wasm_modules.rs C_CALLS_SECTION
const FILE_NOT_FOUND = "No such file or directory (os error 2)";
// SHA-256 (FIPS 180-4) and CRC-32 (IEEE) of bytes, synchronous: host calls cannot await crypto.subtle
const SHA256_K = Uint32Array.from([0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
	0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786,
	0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
	0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb,
	0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
	0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3, 0x748f82ee, 0x78a5636f,
	0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2]);
function sha256Hex(bytes) {
	const length = Math.ceil((bytes.length + 9) / 64) * 64;
	const padded = new Uint8Array(length);
	padded.set(bytes);
	padded[bytes.length] = 0x80;
	new DataView(padded.buffer).setUint32(length - 4, bytes.length * 8);
	new DataView(padded.buffer).setUint32(length - 8, Math.floor(bytes.length / 0x20000000));
	const hash = Uint32Array.from([0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19]);
	const words = new Uint32Array(64);
	const rotate = (x, n) => (x >>> n) | (x << (32 - n));
	for (let block = 0; block < length; block += 64) {
		for (let i = 0; i < 16; i++) words[i] = new DataView(padded.buffer).getUint32(block + i * 4);
		for (let i = 16; i < 64; i++) {
			const s0 = rotate(words[i - 15], 7) ^ rotate(words[i - 15], 18) ^ (words[i - 15] >>> 3);
			const s1 = rotate(words[i - 2], 17) ^ rotate(words[i - 2], 19) ^ (words[i - 2] >>> 10);
			words[i] = words[i - 16] + s0 + words[i - 7] + s1;
		}
		let [a, b, c, d, e, f, g, h] = hash;
		for (let i = 0; i < 64; i++) {
			const t1 = h + (rotate(e, 6) ^ rotate(e, 11) ^ rotate(e, 25)) + ((e & f) ^ (~e & g)) + SHA256_K[i] + words[i];
			const t2 = (rotate(a, 2) ^ rotate(a, 13) ^ rotate(a, 22)) + ((a & b) ^ (a & c) ^ (b & c));
			[h, g, f, e, d, c, b, a] = [g, f, e, (d + t1) >>> 0, c, b, a, (t1 + t2) >>> 0];
		}
		[a, b, c, d, e, f, g, h].forEach((value, i) => { hash[i] += value; });
	}
	return [...hash].map(word => word.toString(16).padStart(8, "0")).join("");
}
const CRC32_TABLE = Uint32Array.from({ length: 256 }, (_, n) => {
	for (let bit = 0; bit < 8; bit++) n = n & 1 ? 0xedb88320 ^ (n >>> 1) : n >>> 1;
	return n;
});
const crc32Of = bytes => (bytes.reduce((crc, byte) => CRC32_TABLE[(crc ^ byte) & 0xff] ^ (crc >>> 8), 0xffffffff) ^ 0xffffffff) >>> 0;

// the standard library's adapters (src/std_adapters.rs, notes/stdlib.md section 7): module → member → function of
// plain values (plainOfTree / treeOfPlain, as for foreign_call)
const STD_ADAPTERS = {
	json: { parse: text => JSON.parse(text), to_json: (value, classes) => JSON.stringify(classes ? withoutClassTags(value, new Set(classes)) : value) },
	file: {
		write: (path, content) => { writtenFiles.set(filePath(path), contentText(content)); return null; },
		append: (path, content) => { writtenFiles.set(filePath(path), textOfFile(path) + contentText(content)); return null; },
		exists: path => writtenFiles.has(filePath(path)) || servedFileExists(path),
		list: folder => {
			const prefix = filePath(folder).replace(/\/?$/, "/");
			return [...writtenFiles.keys()].filter(path => path.startsWith(prefix) && !path.slice(prefix.length).includes("/")).map(path => path.slice(prefix.length)).sort();
		},
	},
	os: { env: () => null }, // a page has no environment
	// `stored theme = "dark"` (src/lowering/stored_values.rs): the page's values (worker.js), each save sent back to it
	store: {
		load: (name, fallback) => name in storedValues ? storedValues[name] : fallback,
		save: (name, value) => { storedValues[name] = value; self.keepStored?.(name, value); return null; },
	},
	net: { post: (url, body) => postSync(url, contentText(body)) },
	hash: { sha256: subject => sha256Hex(utf8.encode(contentText(subject))), crc32: subject => crc32Of(utf8.encode(contentText(subject))) },
	regex: {
		matches: (subject, pattern) => regexOf(pattern).test(subject),
		first: (subject, pattern) => subject.match(regexOf(pattern))?.[0] ?? null,
		all: (subject, pattern) => [...subject.matchAll(regexOf(pattern, "g"))].map(found => found[0]),
		replace: (subject, pattern, replacement) => subject.replace(regexOf(pattern, "g"), replacement),
	},
};
// an instance of one of the program's classes is its fields: {Point: {x: 1}} is {x: 1} (src/std_adapters.rs)
function withoutClassTags(value, classes) {
	if (Array.isArray(value)) return value.map(item => withoutClassTags(item, classes));
	if (value === null || typeof value !== "object") return value;
	const keys = Object.keys(value);
	if (keys.length === 1 && classes.has(keys[0]) && value[keys[0]] !== null && typeof value[keys[0]] === "object" && !Array.isArray(value[keys[0]])) return withoutClassTags(value[keys[0]], classes);
	return Object.fromEntries(keys.map(key => [key, withoutClassTags(value[key], classes)]));
}
// what Rust's regex lacks is refused here too, so a pattern means the same in both hosts (src/std_adapters.rs regex_of)
function regexOf(pattern, flags = "") {
	const feature = /\(\?<?[=!]/.test(pattern) ? "look-around" : /\\[1-9]/.test(pattern) ? "a backreference" : null;
	if (feature) throw new Error(`${feature} is not in wasp's regex (one engine lacks it): ${pattern}`);
	return new RegExp(pattern, flags + "u");
}
// the stored values of `stored x = v`, by name: the page's localStorage as the worker started (playground.js)
const storedValues = {};
// the files std's file module wrote, kept while the page is open: read and the file words see them before the served ones
const writtenFiles = new Map();
const filePath = path => path.replace(/^\.\//, "");
const contentText = content => typeof content === "string" ? content : JSON.stringify(content);
const textOfFile = path => writtenFiles.get(filePath(path)) ?? (servedFileExists(path) ? readFile(filePath(path)) : "");
function servedFileExists(path) {
	try {
		readBytes(path);
		return true;
	} catch {
		return false;
	}
}

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

// a synchronous POST of a text (stdlib net's post, src/extensions/utils.rs post_within), its answer's text
function postSync(url, body) {
	const request = new XMLHttpRequest();
	request.open("POST", url, false);
	request.setRequestHeader("Content-Type", "text/plain; charset=utf-8");
	request.send(body);
	if (request.status >= 400 || request.status === 0) throw new Error(request.status ? `HTTP status ${request.status}` : "network error (blocked by CORS?)");
	return request.responseText;
}

// a file of the served repository, failing in the words of the native read (src/host.rs)
function readFile(path) {
	try {
		return getSync(FILE_ROOT + path);
	} catch (failure) {
		throw failure.message === HTTP_NOT_FOUND ? new Error(FILE_NOT_FOUND) : failure;
	}
}

// the URL of a file of the served repository, of the page itself (PAGE_PREFIX) or a URL
function fileUrl(path) {
	if (/^https?:/.test(path)) return path;
	if (path.startsWith(PAGE_PREFIX)) return new URL(path.slice(PAGE_PREFIX.length), self.location.href).href;
	return FILE_ROOT + path.replace(/^\.\//, "");
}

// the bytes of a file of the served repository, of the page or of a URL, failing in the words of the native read
function readBytes(path) {
	const written = writtenFiles.get(filePath(path));
	if (written !== undefined) return utf8.encode(written);
	try {
		return getSync(fileUrl(path), 0, true);
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
			// `try f(args) else Y` (src/host.rs guarded_call): f's node wrapper called in this instance; the engine's stack
			// overflow (a RangeError) is the Error "call stack exhausted", any other failure (a raised error) goes on up
			guarded_call: (name, values) => {
				const module = program();
				try {
					const result = module[decode(cString(name))](values);
					if (typeof result === "bigint") return module.new_int(result);
					return typeof result === "number" ? module.new_float(result) : result;
				} catch (failure) {
					if (!(failure instanceof RangeError)) throw failure;
					const [pointer, length] = writeBytes(module, utf8.encode("call stack exhausted"));
					return module.error_of(module.new_text(pointer, length));
				}
			},
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
			// std_pure / std_io(module, member, arguments): a word of std/<module>.wasp (src/std_adapters.rs)
			std_pure: (module, member, argumentList) => stdCall(program(), module, member, argumentList),
			std_io: (module, member, argumentList) => stdCall(program(), module, member, argumentList),
			// `serve 8080 {…}` (src/web_server.rs): a page cannot listen on a port
			serve_routes: port => { throw new Error(`serve ${port}: a server runs only in the native host (the warp CLI)`); },
			// a module of another runtime (src/foreign.rs), run by the runtime registered under its name
			foreign_call: (runtime, module, member, call, argumentList) => {
				const program_ = program();
				const [runtimeName, moduleName, memberName] = [runtime, module, member].map(node => plainOfTree(readNode(program_, node)));
				const foreign = foreignRuntimes.get(runtimeName);
				if (!foreign) throw new Error(`${runtimeName} ${moduleName}.${memberName}: ${runtimeName} runs only in the native host (the warp CLI)`);
				const given = plainOfTree(readNode(program_, call)) === 1 ? plainOfTree(readNode(program_, argumentList)) : undefined;
				const argumentValues = given === undefined ? null : given === null ? [] : Array.isArray(given) ? given : [given];
				return buildValue(program_, treeOfPlain(foreign.call(moduleName, memberName, argumentValues, hooks)));
			},
			// the host words (src/host.rs): a page cannot block, so sleep busy-waits
			sleep: milliseconds => {
				const until = Date.now() + Number(milliseconds);
				let check = Date.now() + SHARED_CHECK_MILLISECONDS;
				while (Date.now() < until) {
					if (Date.now() >= check) {
						checkShared(holder);
						deliverFetches(holder);
						check = Date.now() + SHARED_CHECK_MILLISECONDS;
					}
				}
				checkShared(holder);
				deliverFetches(holder);
			},
			random: () => Math.random(),
			random_below: bound => bound > 0n ? BigInt(Math.floor(Math.random() * Number(bound))) : 0n,
			clock: () => BigInt(Date.now()),
			// a page has no ctrl-c: `on interrupt {…}` never runs here (notes/system_signals.md); shared listeners do
			signal_poll: () => { checkShared(holder); deliverFetches(holder); },
			// channels between programs (src/channels.rs): a BroadcastChannel of the name, which reaches the other tabs and
			// workers of this page's origin; the listener's timer (lowering/system_signals.rs) takes what arrived
			channel_listen: (id, channel) => listenOnChannel(holder, hooks, Number(id), plainOfTree(readNode(program(), channel))),
			channel_pending: id => BigInt(holder.channels?.get(Number(id))?.messages.length ?? 0),
			channel_next: id => buildValue(program(), treeOfPlain(holder.channels?.get(Number(id))?.messages.shift() ?? null)),
			channel_send: (channel, message) => {
				const program_ = program();
				new BroadcastChannel(plainOfTree(readNode(program_, channel))).postMessage(plainOfTree(readNode(program_, message)));
			},
			// channels inside one run (P155, src/tasks.rs Channels): Go's unbuffered channel between the task Workers
			channel_new: () => openChannel(holder.run),
			channel_put: (id, value) => putOnChannel(holder, id, utf8.encode(JSON.stringify(readTaskValue(program(), value)))),
			channel_take: id => {
				const bytes = takeFromChannel(holder, id);
				return bytes ? buildValue(program(), JSON.parse(decode(bytes))) : null;
			},
			channel_more: id => moreOnChannel(holder, id),
			channel_close: id => closeChannel(holder.run, id),
			signal_every: (id, milliseconds) => addTimer(holder, hooks, id, { every: Number(milliseconds) }, "on every …"),
			signal_daily: (id, minute, weekdays) => addTimer(holder, hooks, id, { minute: Number(minute), weekdays: Number(weekdays) }, "on every day at …"),
			signal_at: (id, minute) => addTimer(holder, hooks, id, { minute: Number(minute), once: true }, "at 9:00 {…}"),
			signal_watch: () => { holder.warnings.push("on file … change: a page has no files to watch"); },
			fetch_start: (id, url) => startFetch(holder, hooks, Number(id), plainOfTree(readNode(program(), url))),
			fetch_reply: id => buildValue(program(), treeOfPlain(fetchReply(holder, Number(id)))),
			// system values (crates/warp-runtime/src/system_values.rs): what the browser tells, a loud error for the rest
			system_value: name => {
				const value = decode(cString(name));
				if (value === "online") return BigInt(navigator.onLine);
				// a Worker has no matchMedia: the page tells it (playground.js tellSystemValues)
				const known = value === "dark mode" && globalThis.matchMedia ? matchMedia(DARK_MODE_QUERY).matches : self.pageSystemValues?.[value];
				if (known !== undefined) return BigInt(known);
				throw new Error(`${value}: the playground cannot read it yet`);
			},
			// `notify "text"` (src/host.rs notify): the page shows it (playground.js notification)
			notify: text => {
				const shown = plainOfTree(readNode(program(), text));
				if (!hooks.notify) throw new Error(`notify ${JSON.stringify(shown)}: this page shows no notifications`);
				hooks.notify(typeof shown === "string" ? shown : JSON.stringify(shown));
			},
			clipboard_text: () => { throw new Error("clipboard: the playground cannot read it (the browser's clipboard is asynchronous)"); },
			// `exit(code)` ends the run, its value ø (P121): runProgram tells it from a failure by holder.exitCode
			exit: code => {
				holder.exitCode = Number(code);
				throw new Error(`exit(${code})`);
			},
			// paint(pixels, width, height) (src/host.rs): the page draws them on a canvas (playground.js showPaintings)
			paint: (pixels, width, height) => {
				if (!hooks.paint) throw new Error("paint: no canvas here; it draws in the playground page");
				hooks.paint(plainOfTree(readNode(program(), pixels)), Number(width), Number(height));
			},
			// tasks (src/tasks.rs): `go f(x)` runs f in a fresh instance of the program, values copied in and out; a page
			// without cross-origin isolation has no shared memory to wait on, so the task runs at once where it starts
			task_spawn: (name, a0, a1, a2, a3) => startTask(holder, hooks, decode(cString(name)), [a0, a1, a2, a3]),
			task_spawn_values: (name, values) => startTask(holder, hooks, decode(cString(name)), null, readTaskValue(program(), values)),
			task_await: id => {
				const task = takenTask(holder.run, hooks, id);
				if (task.failure) throw new Error(task.failure);
				// a task whose function returns a Node (`go { n + 1 }` of a Node parameter n) still gives its number here
				if (typeof task.value === "object" && task.value !== null) {
					const number = task.value.data?.int;
					if (number === undefined) throw new Error(`task ${taskName(task.name ?? "")}: its result is no whole number`);
					return BigInt(number);
				}
				return task.value;
			},
			task_await_value: id => buildValue(program(), taskTree(takenTask(holder.run, hooks, id).value)),
			// every await joins first: the raises the task forwarded run here, before its value is read
			task_join: id => {
				const failed = takenTask(holder.run, hooks, id).failure ? 1n : 0n;
				deliverSignals(holder);
				return failed;
			},
			task_failure: id => buildValue(program(), textTree(takenTask(holder.run, hooks, id).failure ?? "")),
			task_status: id => {
				const status = taskStatus(holder.run, id);
				deliverSignals(holder);
				return status;
			},
			// a raise inside a task goes to the starting program's handlers (src/tasks.rs signal_send, deliver)
			task_inside: () => holder.inTask ? 1n : 0n,
			signal_send: (handler, values) => {
				(holder.run.signals ??= []).push({ handler: decode(cString(handler)), values: readTaskValue(program(), values) });
				return 0n;
			},
			task_control: (id, operation) => controlTask(holder.run, id, operation),
			// where a loop starts: a task paused from its starting program waits (its control word, set by controlTask)
			task_poll: () => {
				deliverSignals(holder);
				const control = holder.control;
				if (!control) return;
				while (Atomics.load(control, 0) === CONTROL_PAUSED) Atomics.wait(control, 0, CONTROL_PAUSED);
			},
			// shared arrays (src/shared.rs): Ints every task of the run reaches, in shared memory when the page is isolated;
			// one cell more, the last, counts the writes (shared_writes)
			shared_new: length => {
				const Buffer = self.crossOriginIsolated ? SharedArrayBuffer : ArrayBuffer;
				holder.run.shared.push(new BigInt64Array(new Buffer(8 * (Math.max(0, Number(length)) + 1))));
				return BigInt(holder.run.shared.length);
			},
			shared_get: (id, index) => Atomics.load(...sharedCell(holder.run, id, index)),
			shared_set: (id, index, value) => (Atomics.store(...writtenCell(holder.run, id, index), value), value),
			shared_add: (id, index, value) => Atomics.add(...writtenCell(holder.run, id, index), value) + value,
			shared_count: id => BigInt(sharedArray(holder.run, id).length - 1),
			shared_writes: id => {
				const array = sharedArray(holder.run, id);
				return Atomics.load(array, array.length - 1);
			},
			// an array of floats: the cells hold the bits; an add swaps until no other task came between
			shared_getf: (id, index) => floatOfBits(Atomics.load(...sharedCell(holder.run, id, index))),
			shared_setf: (id, index, value) => (Atomics.store(...writtenCell(holder.run, id, index), bitsOfFloat(value)), value),
			shared_addf: (id, index, value) => {
				const [array, cell] = writtenCell(holder.run, id, index);
				for (;;) {
					const old = Atomics.load(array, cell);
					const sum = floatOfBits(old) + value;
					if (Atomics.compareExchange(array, cell, old, bitsOfFloat(sum)) === old) return sum;
				}
			},
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
		c: libcImports(holder),
	};
	// anything else (native FFI libraries) is missing in the browser: say which, when the program calls it
	const missing = (module, name) => () => { throw new Error(`${module}.${name} is not available in the browser`); };
	return new Proxy(known, {
		get: (modules, module) => new Proxy(modules[module] ?? (MODULE_PATH.test(module) ? moduleImports(holder, hooks, module) : {}), {
			get: (functions, name) => functions[name] ?? missing(module, String(name)),
		}),
	});
}

const TASK_FINISHED = 1n; // src/host.rs TASK_FINISHED, TASK_FAILED, TASK_STOPPED, TASK_STOP
const TASK_FAILED = 2n;
const TASK_STOPPED_CODE = 3n;
const TASK_STOP = 1n;
const TASK_PAUSE = 2n; // src/host.rs TASK_PAUSE, TASK_RESUME, TASK_PAUSED
const TASK_RESUME = 3n;
const TASK_PAUSED = 4n;
const CONTROL_RUNNING = 0; // a task Worker's control word
const CONTROL_PAUSED = 1;
const KIND_MASK = 0xFFn;
const decode = bytes => utf8Decoder.decode(bytes);

const TASK_WORKER = "task-worker.js";
const TASK_WORKER_READY = "ready"; // a task Worker's first message: loaded
const FETCH_DONE = "fetched"; // a task Worker's message after a fetch it made (startFetch)
const TASK_RESULT_BYTES = 1 << 16; // the shared buffer a Worker writes its result into, grown as needed
const TASK_RESULT_LIMIT = 1 << 28;
const TASK_HEADER = 8; // [state, length] as Int32, then the result's JSON
const TASK_STOPPED = "task stopped";
// Workers a task runs on: made while the program's worker is idle (prepareTaskPool), since a Worker only starts once
// its creator returns to its event loop and a running program never does (emscripten keeps a thread pool for this)
const taskPool = [];
const TASK_POOL_SIZE = Math.min(4, self.navigator?.hardwareConcurrency ?? 2);
// how long a run waits for the pool's Workers to load before it starts anyway (a task then runs inline)
const TASK_POOL_WAIT_MS = 10000;
const TASK_POOL_POLL_MS = 10;
const hasTaskWorkers = () => self.crossOriginIsolated && self.Worker;

function addTaskWorker() {
	const worker = new Worker(TASK_WORKER);
	// loaded: it can take tasks; later a fetch it made is done (startFetch)
	worker.onmessage = ({ data }) => data === FETCH_DONE ? worker.fetched?.() : taskPool.push(worker);
}

// the pool of task Workers, made by the workers that run programs (worker.js, test-worker.js) when they start
function prepareTaskPool(size = TASK_POOL_SIZE) {
	if (!hasTaskWorkers()) return;
	for (let index = 0; index < size; index++) addTaskWorker();
}

// resolves once every task Worker of the pool has loaded: a run that starts before would run its tasks inline, where
// `stop` cannot end one (samples/threads.wasp's endless spin hung the browser suite, card flaky-browser)
function taskPoolReady() {
	if (!hasTaskWorkers()) return Promise.resolve();
	const deadline = performance.now() + TASK_POOL_WAIT_MS;
	return new Promise(resolve => {
		const check = () => (taskPool.length >= TASK_POOL_SIZE || performance.now() > deadline) ? resolve() : setTimeout(check, TASK_POOL_POLL_MS);
		check();
	});
}

const floatBits = new DataView(new ArrayBuffer(8));
const floatOfBits = bits => (floatBits.setBigInt64(0, bits), floatBits.getFloat64(0));
const bitsOfFloat = number => (floatBits.setFloat64(0, number), floatBits.getBigInt64(0));

function sharedArray(run, id) {
	const array = run.shared[Number(id) - 1];
	if (!array) throw new WebAssembly.RuntimeError(`no shared array ${id}`);
	return array;
}

// the array and the cell of a 1-based index; out of range is the runtime error natively too
function sharedCell(run, id, index) {
	const array = sharedArray(run, id);
	const cell = Number(index) - 1;
	if (!(cell >= 0 && cell < array.length - 1)) throw new WebAssembly.RuntimeError("index out of range");
	return [array, cell];
}

// the cell a set or add writes, counted in the array's last cell
function writtenCell(run, id, index) {
	const [array, cell] = sharedCell(run, id, index);
	Atomics.add(array, array.length - 1, 1n);
	return [array, cell];
}

// The local channels of a run (P155, src/tasks.rs Channels) in one SharedArrayBuffer every task Worker gets: a lock,
// a change counter the waiting sides sleep on (a condition variable), the next channel id and whether the program
// ended, then per channel [full, closed, sent, taken, length] and the bytes of the value offered (JSON of a task tree).
// Go's unbuffered channel: a send waits until a receiver took its value
const CHANNEL_SLOTS = 64;
const CHANNEL_VALUE_BYTES = 1 << 16;
const [CHANNEL_LOCK, CHANNEL_CHANGED, CHANNEL_NEXT_ID, CHANNEL_ENDED, CHANNEL_HEADER] = [0, 1, 2, 3, 4];
const [SLOT_FULL, SLOT_CLOSED, SLOT_SENT, SLOT_TAKEN, SLOT_LENGTH, SLOT_FIELDS] = [0, 1, 2, 3, 4, 5];
const CHANNEL_CHECK_MS = 20; // how often a waiting side looks whether it waits forever, as natively
const CHANNEL_INTS = CHANNEL_HEADER + CHANNEL_SLOTS * SLOT_FIELDS;

// a run's channel table, when its program makes channels and the page has shared memory
function channelTable(module) {
	if (!WebAssembly.Module.imports(module).some(entry => entry.name === "channel_new")) return null;
	if (!hasTaskWorkers()) return null;
	return new Int32Array(new SharedArrayBuffer(4 * CHANNEL_INTS + CHANNEL_SLOTS * CHANNEL_VALUE_BYTES));
}

function channelsOf(run) {
	if (!run.channels) throw new Error("ch = channel(): channels need shared memory between Workers, which this page lacks (it is not cross-origin isolated); `warp run` has them");
	return run.channels;
}

function openChannel(run) {
	const table = channelsOf(run);
	const id = Atomics.add(table, CHANNEL_NEXT_ID, 1) + 1;
	if (id > CHANNEL_SLOTS) throw new Error(`channel(): the playground holds ${CHANNEL_SLOTS} channels per run`);
	return BigInt(id);
}

// channel id's fields, and the bytes of its value
function channelSlot(table, id) {
	const index = Number(id);
	if (!(index >= 1 && index <= Atomics.load(table, CHANNEL_NEXT_ID))) throw new Error(`no channel ${id}`);
	const base = CHANNEL_HEADER + (index - 1) * SLOT_FIELDS;
	const field = name => base + name;
	const bytes = new Uint8Array(table.buffer, 4 * CHANNEL_INTS + (index - 1) * CHANNEL_VALUE_BYTES, CHANNEL_VALUE_BYTES);
	return { get: name => table[field(name)], set: (name, value) => { table[field(name)] = value; }, bytes };
}

function lockChannels(table) {
	while (Atomics.compareExchange(table, CHANNEL_LOCK, 0, 1) !== 0) Atomics.wait(table, CHANNEL_LOCK, 1);
}

function unlockChannels(table, changed) {
	if (changed) Atomics.add(table, CHANNEL_CHANGED, 1);
	Atomics.store(table, CHANNEL_LOCK, 0);
	Atomics.notify(table, CHANNEL_LOCK, 1);
	if (changed) Atomics.notify(table, CHANNEL_CHANGED);
}

// the program alone: no task of it runs any more, so nothing could answer. A task run inline (no free Worker) blocks
// its starting program, which could never answer either
function waitsAlone(holder) {
	if (holder.inTask) return !holder.control;
	return ![...holder.run.tasks.values()].some(task => task.worker && Atomics.load(new Int32Array(task.shared, 0, 1), 0) === 0);
}

// wait on channel id until `ready` answers ({value} or {error}), under the lock; it may change the channel
function channelWait(holder, id, waiting, ready) {
	const table = channelsOf(holder.run);
	for (;;) {
		lockChannels(table);
		const seen = Atomics.load(table, CHANNEL_CHANGED);
		let answer;
		try {
			answer = ready(channelSlot(table, id));
			if (!answer && Atomics.load(table, CHANNEL_ENDED)) answer = { error: TASK_STOPPED };
			if (!answer && waitsAlone(holder)) answer = { error: `${waiting} waits forever: no task is left to answer it` };
		} catch (failure) {
			unlockChannels(table, false);
			throw failure;
		}
		unlockChannels(table, Boolean(answer));
		if (answer?.error) throw new Error(answer.error);
		if (answer) return answer.value;
		Atomics.wait(table, CHANNEL_CHANGED, seen, CHANNEL_CHECK_MS);
	}
}

// `ch.send(v)`: offers v once the channel is free, then waits until a receiver took it
function putOnChannel(holder, id, bytes) {
	if (bytes.length > CHANNEL_VALUE_BYTES) throw new Error(`ch.send: a value of ${bytes.length} bytes is more than the playground's ${CHANNEL_VALUE_BYTES} per channel`);
	let mine;
	channelWait(holder, id, "ch.send", slot => {
		if (mine === undefined) {
			if (slot.get(SLOT_CLOSED)) return { error: "send on a closed channel" };
			if (slot.get(SLOT_FULL)) return null;
			slot.bytes.set(bytes);
			slot.set(SLOT_LENGTH, bytes.length);
			slot.set(SLOT_FULL, 1);
			mine = slot.get(SLOT_SENT);
			slot.set(SLOT_SENT, mine + 1);
			Atomics.add(holder.run.channels, CHANNEL_CHANGED, 1); // the offer wakes a receiver
			Atomics.notify(holder.run.channels, CHANNEL_CHANGED);
		}
		return slot.get(SLOT_TAKEN) > mine ? { value: undefined } : null;
	});
}

// `ch.receive()`: the value offered, waiting for one; null (ø) once the channel is closed and empty
function takeFromChannel(holder, id) {
	return channelWait(holder, id, "ch.receive()", slot => {
		if (slot.get(SLOT_FULL)) {
			const bytes = slot.bytes.slice(0, slot.get(SLOT_LENGTH));
			slot.set(SLOT_FULL, 0);
			slot.set(SLOT_TAKEN, slot.get(SLOT_TAKEN) + 1);
			return { value: bytes };
		}
		return slot.get(SLOT_CLOSED) ? { value: null } : null;
	});
}

// `for v in ch`: 1 when a value is offered, 0 once the channel is closed and empty
function moreOnChannel(holder, id) {
	return channelWait(holder, id, "for … in ch", slot => slot.get(SLOT_FULL) ? { value: 1n } : slot.get(SLOT_CLOSED) ? { value: 0n } : null);
}

function closeChannel(run, id) {
	const table = channelsOf(run);
	lockChannels(table);
	const slot = channelSlot(table, id);
	const closed = slot.get(SLOT_CLOSED);
	slot.set(SLOT_CLOSED, 1);
	unlockChannels(table, true);
	if (closed) throw new Error("close of a closed channel");
}

// the program ended: tasks still waiting on a channel stop
function endChannels(run) {
	const table = run.channels;
	if (!table || run.channelsEnded) return;
	run.channelsEnded = true;
	lockChannels(table);
	Atomics.store(table, CHANNEL_ENDED, 1);
	unlockChannels(table, true);
}

// a task of the run: f(arguments) in a fresh instance of the program (src/tasks.rs TaskTable::run), the Int arguments as
// they are or the argument list (`values`, a tree of reader.js) rebuilt for a wrapper f·node. On a Worker of the pool
// (shared memory needs cross-origin isolation), which writes the result into a SharedArrayBuffer; else at once, here
function startTask(holder, hooks, name, ints, values) {
	const run = holder.run;
	const id = BigInt(run.tasks.size + 1);
	const captured = capturedValues(holder.exports, run.module);
	if (taskPool.length > 0) {
		const shared = new SharedArrayBuffer(TASK_HEADER + TASK_RESULT_BYTES, { maxByteLength: TASK_RESULT_LIMIT });
		const worker = taskPool.pop();
		const control = new Int32Array(new SharedArrayBuffer(4));
		worker.postMessage({ module: run.module, name, ints, values, shared, arrays: run.shared, captured, control, channels: run.channels });
		run.tasks.set(id, { name, worker, shared, control, inline: () => runTask(run.module, hooks, holder.warnings, name, ints, values, run.shared, captured, null, run.channels) });
	} else {
		run.tasks.set(id, runTask(run.module, hooks, holder.warnings, name, ints, values, run.shared, captured, null, run.channels));
	}
	return id;
}

// the task's function as the program wrote it (src/tasks.rs task_name): `f`, not `f·node`; `go block 1`, not `go·block·1`
function taskName(name) {
	return name.replace(/·node$/, "").replace(/^go·block·/, "go block ");
}

// the task here: {value} (a number or a tree) or {failure}
function runTask(module, hooks, warnings, name, ints, values, arrays, captured = [], control = null, channels = null) {
	const taskHolder = { warnings, run: { module, tasks: new Map(), shared: arrays, channels }, control, inTask: true };
	let instance;
	try {
		instance = new WebAssembly.Instance(module, programImports(taskHolder, hooks));
		taskHolder.exports = instance.exports;
		// what the spawning instance's closures captured, as it had it
		for (const [global, value] of captured) instance.exports[global].value = value.tree ? buildValue(instance.exports, value.tree) : value.raw;
		const callee = instance.exports[name];
		const result = values ? callee(buildValue(instance.exports, values)) : callee(...ints.slice(0, callee.length));
		const unread = joinTasks(taskHolder.run, hooks);
		if (unread) return { failure: `task ${taskName(name)}: ${unread}`, signals: taskHolder.run.signals };
		return { value: typeof result === "object" && result !== null ? readNode(instance.exports, result) : result, signals: taskHolder.run.signals };
	} catch (trap) {
		// no instance: the task never ran (`Out of memory: Cannot allocate Wasm memory`, see finishedTask)
		return { failure: `task ${taskName(name)}: ${raisedText(instance?.exports) ?? trapMessage(trap, name)}`, unstarted: !instance, signals: taskHolder.run.signals };
	}
}

// the task's record once it is done: a Worker's is waited for (the program runs in a worker, which may block) and
// read from its shared buffer, its output printed then
function finishedTask(run, hooks, id) {
	const task = run.tasks.get(id);
	if (!task.worker) return queuedSignals(run, task);
	const header = new Int32Array(task.shared, 0, 2);
	Atomics.wait(header, 0, 0);
	let record = JSON.parse(decode(new Uint8Array(task.shared, TASK_HEADER, header[1]).slice()));
	if (record.output) hooks.print(record.output, 1);
	taskPool.push(task.worker); // free for the next task
	if (record.value?.kind === KIND_INT && record.ints) record.value = BigInt(record.value.data.int);
	// a Worker that could not even instantiate the task runs it here instead: a page holds ~124 Wasm memories in all its
	// Workers, and only the isolate that dropped an instance collects it, so the dead instances of this one (the
	// starting program's) block the Worker until this isolate collects them, which it does when its own allocation
	// fails (card browser-test, probes/wasm_memory_limit.html)
	if (record.unstarted) record = task.inline();
	run.tasks.set(id, record);
	return queuedSignals(run, record);
}

// the raises a finished task forwarded, queued once for the program (deliverSignals)
function queuedSignals(run, record) {
	if (record.signals?.length && !record.signalsQueued) {
		(run.signals ??= []).push(...record.signals);
		record.signalsQueued = true;
	}
	return record;
}

// the raises tasks forwarded, run in the program's instance by their handlers' node wrappers; a task leaves them to
// the program (its record carries them)
function deliverSignals(holder) {
	if (holder.inTask) return;
	const queue = holder.run.signals ?? [];
	while (queue.length > 0) {
		const { handler, values } = queue.shift();
		holder.exports[handler](buildValue(holder.exports, values));
	}
}

// the listeners on shared values look whether a task changed them; a task leaves that to the program
function checkShared(holder) {
	if (!holder.inTask) holder.exports?.[SHARED_HANDLER]?.();
}

// the task's record, read by the program (await, join): a failure nobody read ends the run (joinTasks)
function takenTask(run, hooks, id) {
	(run.taken ??= new Set()).add(id);
	return finishedTask(run, hooks, id);
}

// every task done; the first failure nobody read, but a stopped task's (src/tasks.rs join_all)
function joinTasks(run, hooks) {
	let unread;
	for (const id of run.tasks.keys()) {
		const { failure } = finishedTask(run, hooks, id);
		if (failure && unread === undefined && !failure.endsWith(TASK_STOPPED) && !run.taken?.has(id)) unread = failure;
	}
	return unread;
}

// running, finished or failed, without waiting (src/host.rs task status codes)
function taskStatus(run, id) {
	const task = run.tasks.get(id);
	if (task.worker && Atomics.load(new Int32Array(task.shared, 0, 1), 0) === 0) return Atomics.load(task.control, 0) === CONTROL_PAUSED ? TASK_PAUSED : 0n;
	const record = task.worker ? JSON.parse(decode(new Uint8Array(task.shared, TASK_HEADER, new Int32Array(task.shared, 0, 2)[1]).slice())) : task;
	if (!record.failure) return TASK_FINISHED;
	return record.failure.endsWith(TASK_STOPPED) ? TASK_STOPPED_CODE : TASK_FAILED;
}

// `stop job` ends a Worker; `pause job` / `resume job` set its control word, which the task reads where a loop starts
function controlTask(run, id, operation) {
	const task = run.tasks.get(id);
	if (!task.worker || Atomics.load(new Int32Array(task.shared, 0, 1), 0) !== 0) return 0n;
	if (operation === TASK_PAUSE || operation === TASK_RESUME) {
		Atomics.store(task.control, 0, operation === TASK_PAUSE ? CONTROL_PAUSED : CONTROL_RUNNING);
		Atomics.notify(task.control, 0);
		return 1n;
	}
	task.worker.terminate();
	addTaskWorker(); // the pool's replacement, loaded once this run returns to its event loop
	run.tasks.set(id, { failure: `task ${task.name}: ${TASK_STOPPED}` });
	return 1n;
}

// the message a raised error left in trap_detail before trapping (src/wasm_reader.rs with_trap_detail), if any
function raisedText(exports) {
	const detail = exports?.[TRAP_DETAIL_EXPORT]?.value;
	if (!detail) return null;
	const tree = readNode(exports, detail);
	return tree.data?.text ?? tree.data?.node?.data?.text ?? null;
}

// the runtime error a trap means: the first runtime error function on its stack (`index_out_of_range` reads "index out
// of range", as src/wasm_emitter list_ops runtime_error_message has it), else the engine's words
function trapMessage(trap, callee) {
	// V8 names a wasm frame `module.function (wasm://…)`
	const names = [...String(trap.stack ?? "").matchAll(/at (?:\w+\.)?([a-z]+(?:_[a-z]+)+) \(wasm/g)].map(match => match[1]);
	const runtime = names.find(name => name !== callee);
	return runtime ? runtime.replaceAll("_", " ") : String(trap.message ?? trap);
}

const textTree = text => ({ kind: "3", data: { text }, chain: [] });
const KIND_FUNCTION = 16n; // src/type_kinds.rs Kind::Function: a closure
const KIND_LIST = 8n;
const CAPTURE_PREFIX = "capture·"; // src/wasm_emitter CAPTURE_EXPORT_PREFIX

// a value of the program for a task: a closure as its target's name and captured values ({closure, captured}, rebuilt
// by the module's closure_rebuild), a list item by item, anything else as reader.js reads it
function readTaskValue(module, node) {
	if (node === null) return { kind: "0", data: null, chain: [] };
	const kind = BigInt(module.get_kind(node));
	if ((kind & KIND_MASK) === KIND_FUNCTION) {
		const captured = module.closure_captured(node);
		return { closure: readNode(module, module.reflect_value(node)).data.text, captured: captured === null ? null : readTaskValue(module, captured) };
	}
	// a key (`value: 5`, a field of a raised event): left and right, as buildValue rebuilds it
	if ((kind & KIND_MASK) === KIND_KEY) return { kind: String(kind), key: [readTaskValue(module, module.reflect_data(node)), readTaskValue(module, module.reflect_value(node))] };
	if ((kind & KIND_MASK) !== KIND_LIST) return readNode(module, node);
	const items = [];
	for (let cell = node; cell !== null; cell = module.reflect_value(cell)) {
		const item = module.reflect_data(cell);
		if (item !== null) items.push(readTaskValue(module, item));
	}
	return { kind: String(kind), data: null, chain: [], items };
}

// the capture globals of the program's closures (src/tasks.rs captured): their values now, for the task's instance
function capturedValues(exports, module) {
	const names = WebAssembly.Module.exports(module).map(entry => entry.name).filter(name => name.startsWith(CAPTURE_PREFIX));
	return names.map(name => {
		const value = exports[name].value;
		return [name, typeof value === "object" && value !== null ? { tree: readTaskValue(exports, value) } : { raw: value }];
	});
}

// a task's result as a tree: a number of a function of numbers, else the tree already read
function taskTree(value) {
	if (typeof value === "bigint") return { kind: KIND_INT, data: { int: String(value) }, chain: [] };
	if (typeof value === "number") return { kind: KIND_FLOAT, data: { float: value }, chain: [] };
	return value ?? { kind: "0", data: null, chain: [] };
}

// the inverse of reader.js readNode for the values a task carries (src/tasks.rs TaskValue): built through the module's
// constructors; a list from its cons cells, the first item in data, the next cells in the chain
function buildValue(module, tree) {
	if (tree.closure !== undefined) {
		const name = buildValue(module, { kind: "5", data: { text: tree.closure }, chain: [] });
		return module.closure_rebuild(name, tree.captured === null ? null : buildValue(module, tree.captured));
	}
	if (tree.key) return module.new_key(buildValue(module, tree.key[0]), buildValue(module, tree.key[1]), BigInt(tree.kind) >> 8n);
	if (tree.items) return tree.items.reduceRight((rest, item) => module.new_list(buildValue(module, item), rest, BigInt(tree.kind) >> 8n), null) ?? module.new_empty();
	const kind = BigInt(tree.kind);
	const payload = tree.data ?? {};
	switch (Number(kind & KIND_MASK)) {
		case 0: return module.new_empty();
		case 1:
			if (payload.exact) return module.new_int(exactHandle(module, ...payload.exact.map(BigInt)));
			if (payload.int === undefined) throw new Error("an exact number beyond the fixnum range cannot cross to another task yet");
			return module.new_int(BigInt(payload.int));
		case 2: return module.new_float(Number(payload.float));
		case 3: case 5: {
			const [pointer, length] = writeBytes(module, utf8.encode(payload.text));
			return Number(kind & KIND_MASK) === 3 ? module.new_text(pointer, length) : module.new_symbol(pointer, length);
		}
		case 4: return module.new_codepoint(payload.i31);
		case 7: case 8: {
			const items = [payload.node, ...tree.chain.map(cell => cell.data?.node)].filter(Boolean);
			return items.reduceRight((rest, item) => module.new_list(buildValue(module, item), rest, kind >> 8n), null) ?? module.new_empty();
		}
		default: throw new Error(`a value of kind ${kind & KIND_MASK} cannot cross to another task yet`);
	}
}

function stdCall(program_, module, member, argumentList) {
	const [moduleName, memberName, given] = [module, member, argumentList].map(node => plainOfTree(readNode(program_, node)));
	const adapter = STD_ADAPTERS[moduleName]?.[memberName];
	if (!adapter) throw new Error(`${moduleName}.${memberName}: no such word in the browser`);
	try {
		return buildValue(program_, treeOfPlain(adapter(...(Array.isArray(given) ? given : [given]))));
	} catch (error) {
		throw new Error(`${moduleName}.${memberName}: ${error.message}`);
	}
}

// a value of the program (a reader.js tree) as a plain JavaScript value, for foreign_call: lists arrays, `{a:1}` objects
const KIND_KEY = 6n;
function plainOfTree(tree) {
	const kind = BigInt(tree.kind);
	const payload = tree.data ?? {};
	// a ø item reads as a null node: it stays null ([1, ø] is [1, null]); only an empty list has no first node
	const items = () => payload.node === undefined ? [] : [payload.node, ...(tree.chain ?? []).map(cell => cell.data?.node ?? null)];
	const plainOfItem = item => item === null ? null : plainOfTree(item);
	switch (Number(kind & KIND_MASK)) {
		case 0: return null;
		case 1:
			if (payload.ratio) return Number(payload.ratio[0].int) / Number(payload.ratio[1].int);
			return payload.int === undefined ? null : Number(payload.int);
		case 2: return Number(payload.float);
		case 3: case 5: return payload.text;
		case 4: return String.fromCodePoint(payload.i31);
		// the value's own cells follow it in the key's flat chain (reader.js): `{a:[1, 2]}` keeps its 2
		case 6: {
			const [value, ...rest] = tree.chain ?? [];
			return { [plainOfTree(payload.node)]: value ? plainOfTree({ ...value, chain: rest }) : null };
		}
		case 7: case 8: {
			const values = items();
			const isObject = (kind >> 8n) === 0n && values.length > 0 && values.every(item => item !== null && (BigInt(item.kind) & KIND_MASK) === KIND_KEY);
			return isObject ? Object.assign({}, ...values.map(plainOfTree)) : values.map(plainOfItem);
		}
		default: return null;
	}
}

// the runtimes foreign_call reaches in the page, by their name in `use <runtime> …`: call(module, member, arguments,
// hooks) gives the member's plain value (arguments null: a read, no call); prepare(code), when given, readies the runtime
// for a program before it runs (an asynchronous load), since a call itself is synchronous; a later registration of a
// name adds to the earlier one (worker.js gives python its prepare)
const foreignRuntimes = new Map();
function registerForeignRuntime(name, runtime) {
	foreignRuntimes.set(name, { ...foreignRuntimes.get(name), ...runtime });
}
const prepareForeignRuntimes = code => Promise.all([...foreignRuntimes.values()].map(runtime => runtime.prepare?.(code)));

// what wasp's operators on a value of the page forward to (src/lowering/foreign_modules.rs), as in src/foreign.rs's loop
const FOREIGN_OPERATORS = { add: (a, b) => a + b, sub: (a, b) => a - b, mul: (a, b) => a * b, truediv: (a, b) => a / b, mod: (a, b) => a % b, pow: (a, b) => a ** b,
	lt: (a, b) => a < b, gt: (a, b) => a > b, le: (a, b) => a <= b, ge: (a, b) => a >= b, eq: (a, b) => a === b, ne: (a, b) => a !== b, neg: a => -a,
	getitem: (a, i) => typeof a.get === "function" ? a.get(i) : a[i], len: a => a.length ?? a.size, list: a => Array.from(a) };

// `use python` in the page: the bridge of src/foreign.rs (foreign_python.py) in Pyodide, which worker.js loads before
// a program that says `use python` runs; requests and answers are the same JSON, handles stay in Pyodide
// `use python`: Pyodide's bridge (web/playground/foreign_python.py), which the playground's worker.js loads before a run
registerForeignRuntime("python", {
	call(moduleName, memberName, argumentValues) {
		if (!self.pythonAnswer) throw new Error(`python ${moduleName}.${memberName}: Python (Pyodide) is not loaded here`);
		const reply = JSON.parse(self.pythonAnswer(JSON.stringify({ module: moduleName, member: memberName, arguments: argumentValues })));
		if (reply.error !== undefined) throw new Error(`python ${typeof moduleName === "string" ? moduleName : "value"}.${memberName}: ${reply.error}`);
		return reply.value;
	},
});

// objects of the page without a plain form (a Date, a Map, an instance, a function), kept for foreign_call behind ids:
// they cross as `{$handle: id, type, text}` and are the object again when they come back
const foreignHandles = [];
const handleOf = value => ({ $handle: foreignHandles.push(value), type: value?.constructor?.name ?? typeof value, text: String(value).slice(0, 200) });
const unhandled = value => value !== null && typeof value === "object" && "$handle" in value ? foreignHandles[value.$handle - 1] : value;
const isPlainObject = value => [Object.prototype, null].includes(Object.getPrototypeOf(value));

// `use js Math`: the page's own globalThis.Math; npm modules need the native host
registerForeignRuntime("js", {
	call(moduleName, memberName, argumentValues) {
		// a module's name, or a handle: an object of the page kept behind an id (src/foreign.rs)
		let owner = null, value = typeof moduleName === "object" ? unhandled(moduleName) : moduleName === "operator" ? FOREIGN_OPERATORS : globalThis[moduleName];
		if (value === undefined) throw new Error(`js ${moduleName}.${memberName}: the page has no global ${moduleName} (modules need the native host)`);
		for (const part of memberName.split(".")) {
			if (value?.[part] === undefined) throw new Error(`js ${moduleName}.${memberName}: ReferenceError: ${moduleName} has no ${memberName}`);
			[owner, value] = [value, value[part]];
		}
		return argumentValues === null ? value : value.apply(owner, argumentValues.map(unhandled));
	},
});

// a plain JavaScript value as a tree buildValue builds: arrays square lists, objects `{key:value …}`, booleans 1/0
const SQUARE_LIST = String((1n << 8n) | KIND_LIST);
const CURLY_LIST = String(KIND_LIST);
const COLON_KEY = String((1n << 8n) | KIND_KEY); // src/operators.rs OP_CODES: Colon is 1
// a codepoint (a WIT char, P94) as src/foreign.rs sends it, since JSON would make it a text: `{$char: "b"}`
const KIND_CODEPOINT = "4";
const isCodepoint = value => value !== null && typeof value === "object" && Object.keys(value).length === 1 && typeof value.$char === "string" && [...value.$char].length === 1;
function treeOfPlain(value) {
	if (value === null || value === undefined) return { kind: "0", data: null, chain: [] };
	if (typeof value === "boolean") return { kind: KIND_INT, data: { int: value ? "1" : "0" }, chain: [] };
	if (typeof value === "bigint") return { kind: KIND_INT, data: { int: String(value) }, chain: [] };
	if (typeof value === "number") return Number.isSafeInteger(value) ? { kind: KIND_INT, data: { int: String(value) }, chain: [] } : { kind: KIND_FLOAT, data: { float: value }, chain: [] };
	if (typeof value === "string") return textTree(value);
	if (isCodepoint(value)) return { kind: KIND_CODEPOINT, data: { i31: value.$char.codePointAt(0) }, chain: [] };
	if (Array.isArray(value)) return { kind: SQUARE_LIST, items: value.map(treeOfPlain) };
	if (typeof value === "function" || (typeof value === "object" && !isPlainObject(value))) return treeOfPlain(handleOf(value));
	// an integer beyond 64 bits from Python: its digits (foreign_python.py plain)
	if (typeof value === "object" && Object.keys(value).length === 1 && typeof value.$int === "string") return { kind: KIND_INT, data: { exact: [value.$int, "1"] }, chain: [] };
	if (typeof value === "object") return { kind: CURLY_LIST, items: Object.entries(value).map(([key, item]) => ({ kind: COLON_KEY, key: [{ kind: "5", data: { text: key }, chain: [] }, treeOfPlain(item)] })) };
	return textTree(String(value));
}

// the Int handle of an exact number beyond the fixnums, composed from fixnum pieces with the module's exported Int
// operations (src/tasks.rs EXACT_BUILDERS, integer_handle)
const FIXNUM_MIN = -(2n ** 62n - 1n);
const FIXNUM_MAX = 2n ** 62n;
function exactHandle(module, numerator, denominator) {
	const integer = n => {
		if (n >= FIXNUM_MIN && n <= FIXNUM_MAX) return n;
		const limbs = [];
		for (let rest = n < 0n ? -n : n; rest > 0n; rest >>= 32n) limbs.push(rest & 0xffffffffn);
		const magnitude = limbs.reduceRight((handle, limb) => module.exact_add(module.int_shift_left(handle, 32n), limb), 0n);
		return n < 0n ? module.exact_sub(0n, magnitude) : magnitude;
	};
	return denominator === 1n ? integer(numerator) : module.exact_div(integer(numerator), integer(denominator));
}

// the NUL-terminated bytes at `address` of a linear memory
function cText(memory, address) {
	const bytes = new Uint8Array(memory.buffer, address);
	const end = bytes.indexOf(0);
	return bytes.slice(0, end < 0 ? bytes.length : end);
}

// the program's C calls (src/wasm_modules.rs c_calls, which lists the letters) by "module\tname"
function cCalls(run) {
	if (!run.cCalls) {
		const lines = WebAssembly.Module.customSections(run.module, C_CALLS_SECTION).flatMap(section => decode(new Uint8Array(section)).split("\n"));
		run.cCalls = new Map(lines.filter(Boolean).map(line => line.split("\t")).map(([module, name, parameters, result]) =>
			[`${module}\t${name}`, { parameters: [...parameters], result }]));
	}
	return run.cCalls;
}

// a C function of a module, called with the program's values: each text (a NUL-terminated copy in the program's memory,
// or a pointer and length) is copied into a block of the module's malloc, each out-pointer gets a NULL slot there, each
// buffer a block of its capacity and a length slot, all freed after the call; a char * result (or char ** out-pointer,
// or the bytes a buffer received) is read back as a text of the program (NULL is ø)
function callC(holder, exports, member, call, values, what) {
	const program = holder.exports;
	const blocks = [];
	const outSlots = [];
	const buffers = [];
	const moduleArguments = [];
	const malloc = bytes => {
		if (!exports.malloc) throw new Error(`${what} takes a text, buffer or out-pointer, but its module exports no malloc(size)`);
		const block = exports.malloc(bytes.length);
		new Uint8Array(exports.memory.buffer).set(bytes, block);
		blocks.push(block);
		return block;
	};
	const u32 = count => new Uint8Array(new Uint32Array([count]).buffer);
	const remaining = [...values];
	(call?.parameters ?? values.map(() => "n")).forEach((letter, index) => {
		if (letter === "o") return moduleArguments.push(outSlots[outSlots.push(malloc(u32(0))) - 1]);
		if (letter === "k") return moduleArguments.push((buffers.at(-1).slot = malloc(u32(buffers.at(-1).capacity))));
		const value = remaining.shift();
		if (letter === "l") return; // the length of the text before it, libc.wasm reads up to the NUL
		if (letter === "b") return moduleArguments.push(buffers[buffers.push({ capacity: Number(value) }) - 1].block = malloc(new Uint8Array(Number(value))));
		if (letter !== "t") return moduleArguments.push(value);
		const length = ["l", "s"].includes(call.parameters[index + 1]) ? Number(remaining[0]) : undefined;
		const text = length === undefined ? cText(program.memory, value) : new Uint8Array(program.memory.buffer, value, length).slice();
		moduleArguments.push(malloc([...text, 0]));
	});
	const returned = member(...moduleArguments);
	const textOf = bytes => bytes === null ? program.new_empty() : program.new_text(...writeBytes(program, bytes));
	const memory = () => new DataView(exports.memory.buffer);
	const received = () => {
		const address = memory().getUint32(outSlots[0], true);
		if (address === 0) throw new Error(`${what} gave nothing through its out-pointer (C status ${returned ?? 0})`);
		return address;
	};
	const bufferBytes = () => {
		if (returned !== 0) throw new Error(`${what} failed (C status ${returned})`);
		const { block, slot } = buffers[0];
		return new Uint8Array(exports.memory.buffer, block, memory().getUint32(slot, true)).slice();
	};
	const value = {
		t: () => textOf(returned === 0 ? null : cText(exports.memory, returned)),
		u: () => BigInt(returned >>> 0),
		ot: () => textOf(cText(exports.memory, received())),
		on: () => received() | 0,
		b: () => textOf(bufferBytes()),
	}[call?.result]?.() ?? returned;
	if (exports.free) blocks.forEach(block => exports.free(block));
	return value;
}

// libc.wasm, instantiated once per worker: its WASI imports serve only stdio, which nothing here reaches
let libc;
function libcExports() {
	const refused = name => () => { throw new Error(`libc.wasm called WASI ${name}`); };
	libc ??= new WebAssembly.Instance(new WebAssembly.Module(getSync(LIBC_URL, 0, true)),
		{ wasi_snapshot_preview1: new Proxy({}, { get: (_, name) => refused(String(name)) }) }).exports;
	return libc;
}

// `use c` in the browser: the program's C calls go to libc.wasm (natively warp opens the system libc)
function libcImports(holder) {
	return new Proxy({}, {
		get: (_, name) => (...values) => {
			const exports = libcExports();
			const member = exports[name];
			if (typeof member !== "function") throw new Error(`c.${name} is not available in the browser (web/playground/lib/build_libc.sh lists libc.wasm's functions)`);
			return callC(holder, exports, member, cCalls(holder.run).get(`c\t${name}`), values, `c.${name}`);
		},
	});
}

// the exports of a WebAssembly module the program imports by path (src/wasm_modules.rs link): instantiated once per
// run at the first call, with its own imports linked like the program's; a global reads through a getter and is set
// through `set g`
function moduleImports(holder, hooks, path) {
	const instance = () => {
		const modules = holder.run.wasmModules ??= new Map();
		const file = new URL(path, FILE_ROOT).href; // one instance however the path is spelled
		if (!modules.has(file)) {
			if (file.endsWith(".wat")) throw new Error(`${path}: a module in WAT text needs the native build`);
			const moduleHolder = { warnings: holder.warnings, run: holder.run };
			const module = new WebAssembly.Module(readBytes(file));
			moduleHolder.exports = new WebAssembly.Instance(module, programImports(moduleHolder, hooks)).exports;
			modules.set(file, moduleHolder.exports);
		}
		return modules.get(file);
	};
	return new Proxy({}, {
		get: (_, name) => (...values) => {
			const exports = instance();
			if (name.startsWith(SETTER_PREFIX)) return (exports[name.slice(SETTER_PREFIX.length)].value = values[0]);
			const member = exports[name];
			if (member instanceof WebAssembly.Global) return member.value;
			if (!member) throw new Error(`${path} exports no ${name}`);
			const call = cCalls(holder.run).get(`${path}\t${name}`);
			return call ? callC(holder, exports, member, call, values, `${path} ${name}`) : member(...values);
		},
	});
}

// Timers (crates/warp-runtime/src/system_signals.rs): main records them, the page starts them after it (worker.js
// startTimers runs the handler on·every·<id>); a host without a page that stays (test-worker.js) only warns
const TIMER_HANDLER_PREFIX = "on·every·";
function addTimer(holder, hooks, id, timer, written) {
	if (!hooks.listen) return holder.warnings.push(`${written}: timers do not run here`);
	(holder.timers ??= []).push({ id: Number(id), handler: TIMER_HANDLER_PREFIX + id, ...timer });
}

// milliseconds from now to the next local minute_of_day on a weekday of the mask (bit 0 Sunday), as seconds_until_on
const EVERY_DAY = 0b1111111;
const DAY_MILLISECONDS = 24 * 3600 * 1000;
function millisecondsUntil(minuteOfDay, weekdays = EVERY_DAY, now = new Date()) {
	const midnight = new Date(now.getFullYear(), now.getMonth(), now.getDate());
	for (let daysAhead = 0; daysAhead <= 7; daysAhead++) {
		const due = new Date(midnight.getFullYear(), midnight.getMonth(), midnight.getDate() + daysAhead, 0, minuteOfDay);
		if (due > now && (weekdays || EVERY_DAY) & (1 << due.getDay())) return due - now;
	}
	return 7 * DAY_MILLISECONDS;
}

// what a timer says in the page: "every 500 ms", "at 09:00", "every day at 09:00", or the channel its listener reads
function timerLabel(holder, { id, every, minute, once }) {
	const channel = holder.channels?.get(id);
	if (channel) return `message from "${channel.name}"`;
	if (every !== undefined) return every % 1000 ? `every ${every} ms` : `every ${every / 1000} s`;
	const time = `${String(Math.floor(minute / 60)).padStart(2, "0")}:${String(minute % 60).padStart(2, "0")}`;
	return once ? `at ${time}` : `every day at ${time}`;
}

// `users := fetch url` (src/fetches.rs, src/lowering/fetch_signals.rs): fetch_start fetches without waiting. A Worker
// of the task pool fetches into shared memory, so a running main sees the reply at its check points (sleep, loop
// starts: deliverFetches runs on·fetch·<id> there, as natively); else, and for a reply that comes after main returned,
// the page runs the handler (worker.js hooks.arrived). A fetch started anew drops the reply of the one before; a run
// that ended drops all.
const FETCH_HANDLER_PREFIX = "on·fetch·";
function startFetch(holder, hooks, id, url) {
	if (!hooks.arrived && taskPool.length === 0) return holder.warnings.push(`fetch ${url}: replies arriving later are not handled here`);
	const started = { url };
	(holder.fetches ??= new Map()).set(id, started);
	const current = () => holder.fetches.get(id) === started && !holder.stopped;
	const arrive = reply => {
		if (!current() || started.reply) return;
		started.reply = reply;
		hooks.arrived?.(holder, FETCH_HANDLER_PREFIX + id);
	};
	const failed = reason => ({ error: `fetch ${url} failed: ${reason}` });
	if (taskPool.length === 0) return void fetchReplyOf(url).then(({ body, error }) => arrive(error ? failed(error) : { body }));
	const worker = taskPool.pop();
	started.shared = new SharedArrayBuffer(TASK_HEADER + TASK_RESULT_BYTES, { maxByteLength: TASK_RESULT_LIMIT });
	started.failed = failed;
	// the Worker is free once it wrote the reply: taken at a check point or here, whichever comes first
	started.release = () => {
		if (!taskPool.includes(worker)) taskPool.push(worker);
		started.release = () => {};
	};
	worker.fetched = () => {
		started.release();
		const reply = sharedFetchReply(started);
		if (reply) arrive(reply);
	};
	worker.postMessage({ fetch: new URL(url, self.location.href).href, shared: started.shared });
}

// {body} or {error} of a URL: the HTTP status of a failed request, or why it failed
function fetchReplyOf(url) {
	return fetch(url).then(async response => response.ok ? { body: await response.text() } : { error: `HTTP status ${response.status}` }, failure => ({ error: failure.message }));
}

// a task Worker's answer: the JSON of `record` in the shared buffer, then its state set to done for the waiting side
function writeShared(shared, record) {
	const reply = utf8.encode(JSON.stringify(record));
	if (TASK_HEADER + reply.length > shared.byteLength) shared.grow(TASK_HEADER + reply.length);
	new Uint8Array(shared, TASK_HEADER, reply.length).set(reply);
	const header = new Int32Array(shared, 0, 2);
	header[1] = reply.length;
	Atomics.store(header, 0, 1);
	Atomics.notify(header, 0);
}

// the reply a task Worker wrote into the fetch's shared buffer, once: undefined while none is there or it was taken
function sharedFetchReply(started) {
	const header = new Int32Array(started.shared, 0, 2);
	if (started.reply || Atomics.load(header, 0) === 0) return undefined;
	const { body, error } = JSON.parse(decode(new Uint8Array(started.shared, TASK_HEADER, header[1]).slice()));
	return error ? started.failed(error) : { body };
}

// a check point of a running main (sleep, loop starts): the handlers of the fetches whose reply is in shared memory
function deliverFetches(holder) {
	for (const [id, started] of holder.fetches ?? []) {
		const reply = started.shared && !holder.stopped ? sharedFetchReply(started) : undefined;
		if (!reply) continue;
		started.reply = reply;
		started.release();
		holder.exports[FETCH_HANDLER_PREFIX + id]();
	}
}

// [value, error] as src/fetches.rs reply gives it: a JSON object or array parsed, any other body the text
function fetchReply(holder, id) {
	const { reply } = holder.fetches?.get(id) ?? {};
	if (!reply) return [null, `fetch ${id} has no reply yet`];
	if (reply.error) return [null, reply.error];
	try {
		const value = JSON.parse(reply.body);
		if (value !== null && typeof value === "object") return [value, null];
	} catch {} // not JSON: the text
	return [reply.body.endsWith("\n") ? reply.body : reply.body + "\n", null]; // wasp convention (src/host.rs fetch)
}

// `on message from "chat" {…}`: what arrives waits in the listener's queue until its timer asks
function listenOnChannel(holder, hooks, id, name) {
	if (!hooks.listen) return holder.warnings.push(`on message from "${name}": channels are not received here`);
	const channel = new BroadcastChannel(name);
	const listener = { name, channel, messages: [] };
	channel.onmessage = ({ data }) => listener.messages.push(data);
	(holder.channels ??= new Map()).set(id, listener);
}

// a run's timers and channels end with it (the next run, worker.js)
function stopListening(holder) {
	holder.stopped = true;
	holder.channels?.forEach(({ channel }) => channel.close());
	holder.stopTimers?.();
}

// run a compiled program: the outcome src/web.rs run_outcome reads
function runProgram(bytes, hooks) {
	let instance;
	const holder = { warnings: [] }; // the runtime warnings go back to the compiler, which reports them (src/web.rs)
	try {
		const module = new WebAssembly.Module(bytes);
		holder.run = { module, tasks: new Map(), shared: [], channels: channelTable(module) };
		instance = new WebAssembly.Instance(holder.run.module, programImports(holder, hooks));
		holder.exports = instance.exports;
	} catch (failure) {
		return { failure: String(failure.message ?? failure) };
	}
	const outcome = outcomeOf(holder, hooks, () => withExitHandler(holder, instance.exports, () => instance.exports.main()));
	const events = pageEvents(instance.exports);
	if ((events.length > 0 || holder.timers || holder.fetches) && outcome.result) hooks.listen?.(holder, events);
	return outcome;
}

// `on exit {…}` (src/lowering/event_signals.rs, natively system_signals.rs with_exit_handler): on·exit runs once after
// main returns or `exit(code)` ends it, never after a failure
function withExitHandler(holder, exports, call) {
	const handler = exports["on·exit"];
	if (!handler) return call();
	// `event` is the exit code (P134), 0 when main returned: an i64 parameter takes a BigInt, a Node one a new_int
	const withCode = () => {
		const code = BigInt(holder.exitCode ?? 0);
		try {
			return handler(code);
		} catch (failure) {
			if (!(failure instanceof TypeError)) throw failure; // the parameter is a Node: the call never started
			return handler(exports.new_int(code));
		}
	};
	const runHandler = () => (handler.length ? withCode() : handler());
	let result;
	try {
		result = call();
	} catch (trap) {
		if (holder.exitCode !== undefined) runHandler();
		throw trap;
	}
	runHandler();
	return result;
}

// the outcome of a call into a run's instance (main, or a page event's handler), as src/web.rs run_outcome reads it
function outcomeOf(holder, hooks, call) {
	const { warnings, exports } = holder;
	try {
		const result = call();
		endChannels(holder.run); // a task still waiting on a channel stops, as natively
		// the tasks nobody awaited finish before the result, as natively; a failure nobody read ends the run
		const unread = joinTasks(holder.run, hooks);
		deliverSignals(holder); // the raises of tasks nobody awaited run their handlers before the run ends
		checkShared(holder); // and the listeners on shared values see what the tasks left
		if (unread) return { failure: unread, warnings };
		return { result: readResult(exports, result), warnings };
	} catch (trap) {
		endChannels(holder.run);
		if (holder.exitCode !== undefined) return { result: { kind: "0", data: null, chain: [] }, warnings };
		if (holder.blockError !== undefined) return { error: holder.blockError, warnings };
		if (!(trap instanceof WebAssembly.RuntimeError || trap instanceof RangeError)) return { failure: String(trap.message ?? trap), warnings };
		const detail = exports[TRAP_DETAIL_EXPORT]?.value;
		return { trap: trap.message, trace: trap.stack ?? "", detail: detail ? readNode(exports, detail) : null, warnings };
	}
}

// the page events a program handles (src/lowering/event_signals.rs PAGE_EVENTS): `on click {…}` exports on·click·node
const PAGE_EVENT_HANDLER = /^on·((?:click|key|input)(?:·\d+)?)·node$/;
function pageEvents(exports) {
	return Object.keys(exports).map(name => name.match(PAGE_EVENT_HANDLER)?.[1]).filter(Boolean);
}

// a page event of a program that handles it, after its main ran: the handler with the event's data, its outcome
function runPageEvent(holder, hooks, event, detail) {
	holder.warnings = [];
	const handler = holder.exports[`on·${event}·node`];
	return outcomeOf(holder, hooks, () => handler(buildValue(holder.exports, treeOfPlain([detail]))));
}

// a timer's handler (on·every·<id>), given ø when it reads `event`, as natively (system_signals.rs call_handler)
function runTimer(holder, hooks, name) {
	holder.warnings = [];
	const handler = holder.exports[name];
	return outcomeOf(holder, hooks, () => handler.length ? handler(holder.exports.new_empty()) : handler());
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
