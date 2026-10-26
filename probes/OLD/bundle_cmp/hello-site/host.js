const TEXT_HEAP_EXPORT = "text_heap";
const TRAP_DETAIL_EXPORT = "trap_detail";
const SHARED_CHECK_MILLISECONDS = 10; // how often a busy sleep looks at its check points
const PAGE_ROUTES_EXPORT = "page·routes";
const PAGE_SUBMITTED_EXPORT = "page·submitted"; // lowering/serve.rs PAGE_SUBMITTED
const PAGE_BITS = 16;
const STDERR = 2;
const DARK_MODE_QUERY = "(prefers-color-scheme: dark)";
const VIEW_DEFAULTS = { view_width: 640, view_height: 480, window_open: 0 };
const STD_ADAPTERS = {};
const storedValues = {};
const sessionValues = {};
const contentText = content => typeof content === "string" ? content : JSON.stringify(content);
const utf8 = new TextEncoder();
const sharedPointer = ({ buffer, names }) => ({ values: new Int32Array(buffer), names });
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
function scratch(program, byteCount) {
const heap = program[TEXT_HEAP_EXPORT];
if (!heap) return undefined;
if (heap.value === 0) heap.value = program.memory.buffer.byteLength;
const address = Math.ceil(heap.value / BigInt64Array.BYTES_PER_ELEMENT) * BigInt64Array.BYTES_PER_ELEMENT;
const missing = address + byteCount - program.memory.buffer.byteLength;
if (missing > 0) program.memory.grow(Math.ceil(missing / (1 << PAGE_BITS)));
return address;
}
const TEXT_FONT = "sans-serif";
function textCoverage(words, size) {
const font = `${size}px ${TEXT_FONT}`;
const measured = new OffscreenCanvas(1, 1).getContext("2d");
measured.font = font;
const metrics = measured.measureText(words);
const width = Math.max(1, Math.ceil(metrics.width));
const height = Math.max(1, Math.ceil(metrics.fontBoundingBoxAscent + metrics.fontBoundingBoxDescent));
const context = new OffscreenCanvas(width, height).getContext("2d");
context.font = font;
context.fillText(words, 0, metrics.fontBoundingBoxAscent);
const pixels = context.getImageData(0, 0, width, height).data;
const covered = new Uint32Array(2 + width * height);
covered.set([width, height]);
for (let pixel = 0; pixel < width * height; pixel++) covered[2 + pixel] = pixels[pixel * 4 + 3];
return covered;
}
function listOfInts(program, ints) {
const address = program.ints_to_list && scratch(program, ints.byteLength);
if (address === undefined) return undefined;
new Uint32Array(program.memory.buffer, address, ints.length).set(ints);
return program.ints_to_list(address, ints.length);
}
function intsOfList(program, list, room) {
const address = program.list_to_ints && scratch(program, room * BigInt64Array.BYTES_PER_ELEMENT);
if (address === undefined) return undefined;
const count = program.list_to_ints(list, address, room);
return count < 0 ? undefined : Array.from(new BigInt64Array(program.memory.buffer, address, count), Number);
}
function errorInProgram(program, reason) {
return program.error_of(program.new_text(...writeBytes(program, utf8.encode(reason))));
}
const HOST_PART_FILES = ["host-files.js", "host-hashes.js", "host-tasks.js", "host-foreign.js", "host-compiler.js", "host-routes.js", "host-gpu.js", "host-timers.js", "host-random.js"];
const hostParts = [];
function addHostPart(part) {
hostParts.push(part);
Object.assign(STD_ADAPTERS, part.adapters);
}
const eachHostPart = (step, ...values) => hostParts.map(part => part[step]?.(...values));
function programImports(holder, hooks) {
const program = () => holder.exports;
const text = (pointer, length) => readText(program(), pointer, length);
const cString = pointer => {
const bytes = new Uint8Array(program().memory.buffer);
let end = pointer;
while (bytes[end] !== 0 && end < bytes.length) end++;
return bytes.slice(pointer, end);
};
const access = { program, text, cString };
const checkPoint = () => eachHostPart("poll", holder);
const known = {
host: {
warn: (pointer, length) => holder.warnings.push(text(pointer, length)),
run: () => -1n,
guarded_call: (name, values) => {
const module = program();
try {
const result = module[decode(cString(name))](values);
if (typeof result === "bigint") return module.new_int(result);
return typeof result === "number" ? module.new_float(result) : result;
} catch (failure) {
if (!(failure instanceof RangeError)) throw failure;
return errorInProgram(module, "call stack exhausted");
}
},
std_pure: (module, member, argumentList) => stdCall(program(), module, member, argumentList, holder),
std_io: (module, member, argumentList) => stdCall(program(), module, member, argumentList, holder),
serve_routes: port => { throw new Error(`serve ${port}: a server runs only in the native host (the warp CLI)`); },
sleep: milliseconds => {
hooks.sleeping?.(Number(milliseconds)); // the next frame of an animation, and no runaway time (playground.js)
const until = Date.now() + Number(milliseconds);
let check = Date.now() + SHARED_CHECK_MILLISECONDS;
while (Date.now() < until) {
if (Date.now() >= check) {
checkPoint();
check = Date.now() + SHARED_CHECK_MILLISECONDS;
}
}
checkPoint();
},
clock: () => BigInt(Date.now()),
signal_poll: checkPoint,
signal_watch: () => { holder.warnings.push("on file … change: a page has no files to watch"); },
system_value: name => {
const value = decode(cString(name));
if (value === "online") return BigInt(navigator.onLine);
if (value === "time of day") { const now = new Date(); return BigInt(now - new Date(now).setHours(0, 0, 0, 0)); }
const pointerIndex = self.pagePointer?.names.indexOf(value) ?? -1; // canvas.js trackPointer
if (pointerIndex >= 0) return BigInt(Atomics.load(self.pagePointer.values, pointerIndex));
const known = value === "dark mode" && globalThis.matchMedia ? matchMedia(DARK_MODE_QUERY).matches : self.pageSystemValues?.[value] ?? VIEW_DEFAULTS[value];
if (known !== undefined) return BigInt(known);
throw new Error(`${value}: the playground cannot read it yet`);
},
notify: text => {
const shown = plainOfTree(readNode(program(), text));
if (!hooks.notify) throw new Error(`notify ${JSON.stringify(shown)}: this page shows no notifications`);
hooks.notify(typeof shown === "string" ? shown : JSON.stringify(shown));
},
text_coverage: (words, size) => {
const covered = textCoverage(String(plainOfTree(readNode(program(), words))), Number(size));
return listOfInts(program(), covered) ?? buildValue(program(), treeOfPlain(Array.from(covered)));
},
clipboard_text: () => { throw new Error("clipboard: the playground cannot read it (the browser's clipboard is asynchronous)"); },
exit: code => {
holder.exitCode = Number(code);
throw new Error(`exit(${code})`);
},
paint: (pixels, width, height, values) => {
if (!hooks.paint) throw new Error("paint: no canvas here; it draws in the playground page");
const room = Number(width) * Number(height);
let painted = intsOfList(program(), pixels, room) ?? plainOfTree(readNode(program(), pixels));
if (typeof painted === "string") {
if (!holder.gpuRendered) throw new Error("paint: a shader needs WebGPU (host-gpu.js), which this page has not");
painted = Array.from(holder.gpuRendered(painted, width, height, values));
}
hooks.paint(painted, Number(width), Number(height));
},
sound_samples: (samples, count, rate) => {
const values = intsOfList(program(), samples, Number(count)) ?? plainOfTree(readNode(program(), samples));
(holder.unrenderedSounds ??= []).push({ samples: values, rate: Number(rate) });
hooks.sound?.(values, Number(rate));
},
...Object.assign({}, ...eachHostPart("words", holder, hooks, access)),
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
...Object.assign({}, ...eachHostPart("imports", holder, hooks)),
};
const missing = (module, name) => () => { throw new Error(`${module}.${name} is not available in the browser`); };
return new Proxy(known, {
get: (modules, module) => new Proxy(modules[module] ?? eachHostPart("importModule", holder, hooks, module).find(Boolean) ?? {}, {
get: (functions, name) => functions[name] ?? missing(module, String(name)),
}),
});
}
const decode = bytes => utf8Decoder.decode(bytes);
const textTree = text => ({ kind: "3", data: { text }, chain: [] });
const KIND_LIST = 8n;
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
case 6: { // a key (an instance among them): its right side the first cell of the chain, which holds that side's own chain
const [right, ...rest] = tree.chain;
return module.new_key(buildValue(module, payload.node), right ? buildValue(module, { ...right, chain: rest }) : module.new_empty(), kind >> 8n);
}
case 7: case 8: {
const items = [payload.node, ...tree.chain.map(cell => cell.data?.node)].filter(Boolean);
return items.reduceRight((rest, item) => module.new_list(buildValue(module, item), rest, kind >> 8n), null) ?? module.new_empty();
}
default: throw new Error(`a value of kind ${kind & KIND_MASK} cannot cross to another task yet`);
}
}
function stdCall(program_, module, member, argumentList, holder) {
const [moduleName, memberName, given] = [module, member, argumentList].map(node => plainOfTree(readNode(program_, node)));
const adapter = STD_ADAPTERS[moduleName]?.[memberName];
if (!adapter) throw new Error(`${moduleName}.${memberName}: no such word in the browser`);
try {
const context = { warn: message => holder.warnings.push(message), holder };
return buildValue(program_, treeOfPlain(adapter.apply(context, Array.isArray(given) ? given : [given])));
} catch (error) {
return errorInProgram(program_, `${moduleName}.${memberName}: ${error.message}`);
}
}
const KIND_KEY = 6n;
function plainOfTree(tree) {
const kind = BigInt(tree.kind);
const payload = tree.data ?? {};
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
case 6: {
const [value, ...rest] = tree.chain ?? [];
return { [plainOfTree(payload.node)]: value ? plainOfTree({ ...value, chain: rest }) : null };
}
case 7: case 8: {
const values = items();
const isObject = (kind >> 8n) === 0n && values.length > 0 && values.every(item => item !== null && (BigInt(item.kind) & KIND_MASK) === KIND_KEY);
return isObject ? Object.assign({}, ...values.map(plainOfTree)) : values.map(plainOfItem);
}
case Number(KIND_FUNCTION): return tree.warpFunction ? callableOf(tree.warpFunction) : null;
default: return null;
}
}
function callableOf({ module, node }) {
return (...values) => plainOfTree(readResult(module, module.closure_apply(node, buildValue(module, treeOfPlain(values)))));
}
const foreignHandles = [];
const handleOf = value => ({ $handle: foreignHandles.push(value), type: value?.constructor?.name ?? typeof value, text: String(value).slice(0, 200) });
const unhandled = value => value !== null && typeof value === "object" && "$handle" in value ? foreignHandles[value.$handle - 1] : value;
const isPlainObject = value => [Object.prototype, null].includes(Object.getPrototypeOf(value));
const SQUARE_LIST = String((1n << 8n) | KIND_LIST);
const CURLY_LIST = String(KIND_LIST);
const COLON_KEY = String((1n << 8n) | KIND_KEY); // src/operators.rs OP_CODES: Colon is 1
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
if (typeof value === "object" && Object.keys(value).length === 1 && typeof value.$int === "string") return { kind: KIND_INT, data: { exact: [value.$int, "1"] }, chain: [] };
if (typeof value === "object") return { kind: CURLY_LIST, items: Object.entries(value).map(([key, item]) => ({ kind: COLON_KEY, key: [{ kind: "5", data: { text: key }, chain: [] }, treeOfPlain(item)] })) };
return textTree(String(value));
}
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
function stopListening(holder) {
holder.stopped = true;
eachHostPart("stopped", holder);
holder.stopTimers?.();
}
const JSPI = typeof WebAssembly.Suspending === "function" && typeof WebAssembly.promising === "function";
const whenSettled = (value, next) => value instanceof Promise ? value.then(next) : next(value);
function runProgram(bytes, hooks, awaited = false) {
const holder = instantiateProgram(bytes, hooks, undefined, awaited && JSPI);
return holder.failure ? holder : runMain(holder, hooks);
}
function instantiateProgram(bytes, hooks, compiled, awaited = false) {
const holder = { warnings: [], pagePath: hooks.pagePath?.(), awaited };
try {
const module = compiled ?? new WebAssembly.Module(bytes);
holder.run = { module, bytes };
eachHostPart("started", holder.run);
holder.exports = new WebAssembly.Instance(holder.run.module, programImports(holder, hooks)).exports;
hooks.instantiated?.(holder);
} catch (failure) {
return { failure: String(failure.message ?? failure) };
}
return holder;
}
function runMain(holder, hooks) {
const { exports } = holder;
const main = holder.awaited ? WebAssembly.promising(exports.main) : exports.main;
holder.waiting = holder.awaited;
return whenSettled(outcomeOf(holder, hooks, () => withExitHandler(holder, exports, () => main())), outcome => {
holder.waiting = false;
const events = pageEvents(exports);
if ((events.length > 0 || holder.timers || holder.fetches || exports[PAGE_ROUTES_EXPORT] || exports[PAGE_SUBMITTED_EXPORT]) && outcome.result) hooks.listen?.(holder, events);
if (events.length > 0 && outcome.result) listeningRun = holder;
return outcome;
});
}
let listeningRun;
const PAGE_VALUE_EXPORT = "page·value";
const PAGE_RENDER_EXPORT = "page·render"; // src/lowering/page_html.rs PAGE_RENDER
function renderedHtml(exports, value) {
const render = exports[PAGE_RENDER_EXPORT];
if (!render || value === null || typeof value !== "object") return undefined;
try {
return plainOfTree(readNode(exports, render(value)));
} catch {
return undefined;
}
}
function withExitHandler(holder, exports, call) {
const handler = exports["on·exit"];
if (!handler) return call();
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
const failed = trap => {
if (holder.exitCode !== undefined) runHandler();
throw trap;
};
let result;
try {
result = call();
} catch (trap) {
failed(trap);
}
if (result instanceof Promise) return result.then(value => (runHandler(), value), failed);
runHandler();
return result;
}
function outcomeOf(holder, hooks, call) {
const { warnings, exports } = holder;
const finished = result => {
const unread = eachHostPart("finished", holder, hooks).find(Boolean);
if (unread) return { failure: unread, warnings };
return { result: readResult(exports, result), html: hooks.renders ? renderedHtml(exports, result) : undefined, warnings };
};
const failed = trap => {
eachHostPart("ended", holder);
if (holder.exitCode !== undefined) return { result: { kind: "0", data: null, chain: [] }, warnings };
if (holder.blockError !== undefined) return { error: holder.blockError, warnings };
if (holder.lastPromised && SUSPENSION_FAILURE.test(trap.message)) return { failure: `${holder.lastPromised}: ${NESTED_PROMISE_REFUSAL}`, warnings };
if (!(trap instanceof WebAssembly.RuntimeError || trap instanceof RangeError)) return { failure: String(trap.message ?? trap), warnings };
const detail = exports[TRAP_DETAIL_EXPORT]?.value;
return { trap: trap.message, trace: trap.stack ?? "", detail: detail ? readNode(exports, detail) : null, warnings };
};
try {
const result = call();
return result instanceof Promise ? result.then(finished).catch(failed) : finished(result);
} catch (trap) {
return failed(trap);
}
}
const PAGE_EVENT_HANDLER = /^on·((?:click|key|input)(?:·\d+)?)·node$/;
function pageEvents(exports) {
return Object.keys(exports).map(name => name.match(PAGE_EVENT_HANDLER)?.[1]).filter(Boolean);
}
function runPageEvent(holder, hooks, event, detail) {
holder.warnings = [];
const handler = holder.exports[`on·${event}·node`];
return outcomeOf(holder, hooks, () => handler(buildValue(holder.exports, treeOfPlain([detail]))));
}
function runSubmitted(holder, hooks, request) {
holder.warnings = [];
const submitted = holder.exports[PAGE_SUBMITTED_EXPORT];
if (!submitted) return { failure: `a form sent ${request.method} ${request.path}, but the program has no route (post "${request.path}" {…})`, warnings: [] };
return outcomeOf(holder, hooks, () => submitted(buildValue(holder.exports, treeOfPlain(request))));
}
function runTimer(holder, hooks, name) {
holder.warnings = [];
const handler = holder.exports[name];
return outcomeOf(holder, hooks, () => handler.length ? handler(holder.exports.new_empty()) : handler());
}
