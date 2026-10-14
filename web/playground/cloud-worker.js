// A warp program as a Cloudflare Worker (card cloud-deploy, src/web/deploy.rs): `warp deploy app.warp` bundles the host
// scripts the program needs (reader.js, host.js, its parts) with this module and the program's app.wasm. Each request
// goes to the program's routes (`get "/" {…}`, lowering/web/serve.rs page·submitted) as {method, path, query, body, headers}; the
// value answers as src/web/web_server.rs answers it: a text as text/plain, any other value as JSON, with the route's
// headers and status; no route is 404, a failure 500 with its message. The program's main runs once per isolate, so its variables live across requests.
import compiledProgram from "./app.wasm";
import programBytes from "./app.bin";

const NO_ROUTE = "no route answers"; // lowering/web/serve.rs: page·submitted of a request no route takes
const JSON_TYPE = "application/json";
const TEXT_TYPE = "text/plain; charset=utf-8";
const STATUS_HEADER = "Status"; // src/host.rs STATUS_HEADER: `status 418` in a route
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
	return { method: request.method, path: url.pathname, query: Object.fromEntries(url.searchParams), body, headers: headersOf(request.headers) };
}

// {"X-Warp": "yes"}: names as src/web/web_server.rs header_name gives them (the Fetch API lowercases them)
const headersOf = headers => Object.fromEntries([...headers].map(([name, value]) => [name.split("-").map(part => part.charAt(0).toUpperCase() + part.slice(1)).join("-"), value]));

// the route's `header "name" "value"` lines go with it, its `status 418` is the status (host.js response_header)
function answer(value, status = 200, { [STATUS_HEADER]: setStatus, ...headers } = {}) {
	const isText = typeof value === "string";
	const body = isText ? value : JSON.stringify(value);
	// a Headers object: the route's Content-Type replaces the default content-type (names are case-insensitive)
	const answerHeaders = new Headers({ "content-type": isText ? TEXT_TYPE : JSON_TYPE });
	for (const [name, value] of Object.entries(headers)) answerHeaders.set(name, value);
	return new Response(body, { status: setStatus ? Number(setStatus) : status, headers: answerHeaders });
}

export default {
	async fetch(request) {
		try {
			const outcome = runSubmitted(startedProgram(), workerHooks, await requestOf(request));
			if (outcome.result !== undefined) return answer(plainOfTree(outcome.result), 200, outcome.headers);
			const failure = failureOf(outcome);
			return answer(failure, failure.includes(NO_ROUTE) ? 404 : 500);
		} catch (failure) {
			return answer(String(failure.message ?? failure), 500);
		}
	},
};
