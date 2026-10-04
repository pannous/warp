// The warp compiler (warp.wasm, built by build.sh) and the programs it compiles (host.js), run off the page's thread:
// a worker may compile any module synchronously and block on a synchronous fetch, which the host calls need.

importScripts("reader.js", "host.js");
prepareTaskPool(); // task Workers start while this worker is idle (host.js)

// warp.wasm, the optimized build, or the one the page names (?compiler=warp.debug.wasm, build.sh)
const COMPILER_URL = new URL(self.location.href).searchParams.get("compiler") ?? "warp.wasm";

let compiler; // the compiler instance's exports
let panicMessage; // the compiler's last panic message

const post = message => self.postMessage(message);
const hooks = {
	print: (text, stream) => post({ type: "print", text, stream }),
	module: bytes => post({ type: "module", bytes }),
	panicked: message => { panicMessage = message; },
};

const compilerText = (pointer, length) => readText(compiler, pointer, length);

async function loadCompiler() {
	const response = await fetch(COMPILER_URL);
	if (!response.ok) throw new Error(`${COMPILER_URL}: HTTP ${response.status}; build it with web/playground/build.sh`);
	const { instance } = await WebAssembly.instantiate(await response.arrayBuffer(), { warp_host: warpHost(() => compiler.memory, hooks) });
	compiler = instance.exports;
}

function passText(text) {
	const bytes = utf8.encode(text);
	const pointer = compiler.web_alloc(bytes.length);
	new Uint8Array(compiler.memory.buffer, pointer, bytes.length).set(bytes);
	return [pointer, bytes.length];
}

function evaluate(code, acknowledged) {
	panicMessage = undefined;
	const codeText = passText(code);
	const acknowledgedText = passText(JSON.stringify(acknowledged));
	try {
		const length = compiler.web_evaluate(...codeText, ...acknowledgedText);
		const report = JSON.parse(compilerText(compiler.web_report(), length));
		compiler.web_free(...codeText);
		compiler.web_free(...acknowledgedText);
		return report;
	} catch (crash) {
		compiler = undefined; // a panic leaves the compiler's memory in an unknown state: load it again
		return { value: `compiler crashed: ${panicMessage ?? crash.message}`, error: true, crashed: true, warnings: [], runtime_warnings: [], hints: [], notes: [] };
	}
}

const ready = loadCompiler().then(() => post({ type: "ready" }), failure => post({ type: "failed", message: failure.message }));

self.onmessage = async ({ data }) => {
	await ready;
	if (!compiler) await loadCompiler();
	const started = performance.now();
	const report = evaluate(data.code, data.acknowledged ?? {});
	post({ type: "report", id: data.id, report, milliseconds: performance.now() - started });
};
