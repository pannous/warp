// A warp program as a Cloudflare Worker (card cloud-deploy, src/deploy.rs): `warp deploy app.warp` bundles the host
// scripts the program needs (reader.js, host.js, its parts) with this module and the program's app.wasm. Each request
// goes to the program's routes (`get "/" {…}`, lowering/serve.rs page·submitted) as {method, path, query, body}; the
// value answers as src/web_server.rs answers it: a text as text/plain, any other value as JSON; no route is 404, a
// failure 500 with its message. The program's main runs once per isolate, so its variables live across requests.
import compiledProgram from "./app.wasm";
import programBytes from "./app.bin";

const NO_ROUTE = "no route answers"; // lowering/serve.rs: page·submitted of a request no route takes
const JSON_TYPE = "application/json";
const TEXT_TYPE = "text/plain; charset=utf-8";
const workerHooks = { print: text => console.log(text) };

let program;
function startedProgram() {
	if (program) return program;
	const holder = instantiateProgram(programBytes, workerHooks, compiledProgram);
	if (holder.failure) throw new Error(holder.failure);
	const outcome = runMain(holder, workerHooks);
	if (outcome.result === undefined) throw new Error(failureOf(outcome));
	return program = holder;
}

// a thrown value (`throw "…"`) is the trap's detail
const failureOf = outcome => String(outcome.detail ? plainOfTree(outcome.detail) : outcome.failure ?? outcome.error?.message ?? outcome.error ?? outcome.trap);

async function requestOf(request) {
	const url = new URL(request.url);
	const text = await request.text();
	const isJson = (request.headers.get("content-type") ?? "").startsWith(JSON_TYPE);
	const body = text && isJson ? JSON.parse(text) : text;
	return { method: request.method, path: url.pathname, query: Object.fromEntries(url.searchParams), body };
}

function answer(value, status = 200) {
	const isText = typeof value === "string";
	return new Response(isText ? value : JSON.stringify(value), { status, headers: { "content-type": isText ? TEXT_TYPE : JSON_TYPE } });
}

export default {
	async fetch(request) {
		try {
			const outcome = runSubmitted(startedProgram(), workerHooks, await requestOf(request));
			if (outcome.result !== undefined) return answer(plainOfTree(outcome.result));
			const failure = failureOf(outcome);
			return answer(failure, failure.includes(NO_ROUTE) ? 404 : 500);
		} catch (failure) {
			return answer(String(failure.message ?? failure), 500);
		}
	},
};
