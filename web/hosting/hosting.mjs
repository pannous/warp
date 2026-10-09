// warp-hosting: the control Worker that hosts warp programs (notes/hosting.md, card cloud-hosting), at
// https://lambda.pannous.com (pannous.com's Apache proxies it to warp-hosting.pannous.workers.dev).
// Users log in with GitHub and deploy a program's module, which becomes the Worker warp-<name> in our Cloudflare
// account, at https://warp-<name>.pannous.workers.dev. "Deploy to my own account" does the same upload with the
// user's own Cloudflare token, through here, as api.cloudflare.com allows no browser (CORS) calls.
// The program's Worker is the one `warp deploy` builds (src/deploy.rs): the host scripts its module needs, then
// cloud-worker.js, composed here from our own copies, so a deploy carries no JavaScript of its own.
import readerScript from "../playground/reader.js";
import hostScript from "../playground/host.js";
import importsScript from "../playground/imports.js";
import filesScript from "../playground/host-files.js";
import hashesScript from "../playground/host-hashes.js";
import tasksScript from "../playground/host-tasks.js";
import foreignScript from "../playground/host-foreign.js";
import compilerScript from "../playground/host-compiler.js";
import routesScript from "../playground/host-routes.js";
import gpuScript from "../playground/host-gpu.js";
import timersScript from "../playground/host-timers.js";
import randomScript from "../playground/host-random.js";
import cloudWorkerScript from "../playground/cloud-worker.js";

// the host scripts by the names src/site.rs host_scripts_of gives them (HOST_SCRIPTS, HOST_PARTS)
const HOST_SCRIPTS = {
	"reader.js": readerScript,
	"host.js": hostScript,
	"host-files.js": filesScript,
	"host-hashes.js": hashesScript,
	"host-tasks.js": importsScript + tasksScript,
	"host-foreign.js": foreignScript,
	"host-compiler.js": compilerScript,
	"host-routes.js": importsScript + routesScript,
	"host-gpu.js": gpuScript,
	"host-timers.js": timersScript,
	"host-random.js": randomScript,
};
const ALWAYS_SCRIPTS = ["reader.js", "host.js"];

const CLOUDFLARE_API = "https://api.cloudflare.com/client/v4";
const GITHUB_AUTHORIZE = "https://github.com/login/oauth/authorize";
const GITHUB_TOKEN = "https://github.com/login/oauth/access_token";
const GITHUB_USER = "https://api.github.com/user";
const CLOUDFLARE_AUTHORIZE = "https://dash.cloudflare.com/oauth2/auth";
const CLOUDFLARE_TOKEN = "https://dash.cloudflare.com/oauth2/token";
const CLOUDFLARE_SCOPES = "workers-scripts.edit user-details.read account-settings.read";
const SCRIPT_PREFIX = "warp-";
const CALLBACK_PATH = "/callback";
const NAME_PATTERN = /^[a-z0-9]([a-z0-9-]{0,38}[a-z0-9])?$/;
const SESSION_PREFIX = "warp.";
const SESSION_SECONDS = 30 * 24 * 3600;
const STATE_SECONDS = 600;
const MAX_WASM_BYTES = 10 * 1024 * 1024; // Workers' paid script limit; the free plan's 3 MB gzipped is checked by Cloudflare
const WASM_MAGIC = [0, 0x61, 0x73, 0x6d];
const COMPATIBILITY_DATE = "2026-09-01";
const ALLOWED_ORIGINS = [/^https:\/\/warp\.pannous\.com$/, /^https:\/\/pannous\.github\.io$/, /^http:\/\/(localhost|127\.0\.0\.1)(:\d+)?$/];
const USER_AGENT = "warp-hosting";

class Refusal extends Error {
	constructor(status, message) { super(message); this.status = status; }
}

const json = (value, status = 200, headers = {}) => Response.json(value, { status, headers });

// ---- signed values (sessions, OAuth states): base64url(JSON).base64url(HMAC-SHA256) ----------------------------

const encoder = new TextEncoder();
const base64url = bytes => btoa(String.fromCharCode(...new Uint8Array(bytes))).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");
const fromBase64url = text => Uint8Array.from(atob(text.replace(/-/g, "+").replace(/_/g, "/")), c => c.charCodeAt(0));

async function hmacKey(env) {
	if (!env.SESSION_SECRET) throw new Refusal(500, "warp-hosting has no SESSION_SECRET: wrangler secret put SESSION_SECRET");
	return crypto.subtle.importKey("raw", encoder.encode(env.SESSION_SECRET), { name: "HMAC", hash: "SHA-256" }, false, ["sign", "verify"]);
}

async function signed(env, value, seconds) {
	const payload = base64url(encoder.encode(JSON.stringify({ ...value, expires: Date.now() + seconds * 1000 })));
	const signature = await crypto.subtle.sign("HMAC", await hmacKey(env), encoder.encode(payload));
	return `${payload}.${base64url(signature)}`;
}

async function unsigned(env, text) {
	const [payload, signature] = (text ?? "").split(".");
	if (!payload || !signature) return null;
	const valid = await crypto.subtle.verify("HMAC", await hmacKey(env), fromBase64url(signature), encoder.encode(payload));
	if (!valid) return null;
	const value = JSON.parse(new TextDecoder().decode(fromBase64url(payload)));
	return value.expires > Date.now() ? value : null;
}

// ---- who is asking ---------------------------------------------------------------------------------------------

async function githubUser(token) {
	const reply = await fetch(GITHUB_USER, { headers: { authorization: `Bearer ${token}`, "user-agent": USER_AGENT, accept: "application/vnd.github+json" } });
	if (!reply.ok) throw new Refusal(401, `GitHub refused the token (${reply.status})`);
	const { id, login } = await reply.json();
	return { id, login };
}

// a session of our GitHub login (warp.<signed>), or a GitHub token itself (a CLI: gh auth token)
async function user(request, env) {
	const token = request.headers.get("authorization")?.replace(/^Bearer\s+/i, "");
	if (!token) throw new Refusal(401, "log in with GitHub first");
	if (!token.startsWith(SESSION_PREFIX)) return githubUser(token);
	const session = await unsigned(env, token.slice(SESSION_PREFIX.length));
	if (!session) throw new Refusal(401, "the login expired: log in with GitHub again");
	return { id: session.id, login: session.login };
}

// ---- OAuth logins in a popup: the callback page hands the result to the playground that opened it --------------

function allowedOrigin(origin) {
	return origin && ALLOWED_ORIGINS.some(pattern => pattern.test(origin)) ? origin : null;
}

function handOver(origin, message) {
	const script = `opener && opener.postMessage(${JSON.stringify(message)}, ${JSON.stringify(origin)}); close();`;
	const escaped = text => text.replace(/</g, "&lt;");
	const login = message.warpHosting?.login ? ` as ${escaped(message.warpHosting.login)}` : "";
	const page = `<!doctype html><meta charset="utf-8"><title>warp hosting</title><p>${message.error ? "Login failed: " + escaped(message.error) : `Logged in${login}, you can close this window.`}</p><script>${script}</script>`;
	return new Response(page, { headers: { "content-type": "text/html; charset=utf-8" } });
}

// the GitHub OAuth app's callback URL is PUBLIC_ORIGIN/callback; Cloudflare's goes below it
function callbackOf(env, url, provider) {
	return `${env.PUBLIC_ORIGIN ?? url.origin}${CALLBACK_PATH}${provider === "github" ? "" : "/" + provider}`;
}

async function pkcePair() {
	const verifier = base64url(crypto.getRandomValues(new Uint8Array(32)));
	const challenge = base64url(await crypto.subtle.digest("SHA-256", encoder.encode(verifier)));
	return { verifier, challenge };
}

async function startLogin(request, env, provider) {
	const url = new URL(request.url);
	const origin = allowedOrigin(url.searchParams.get("origin"));
	if (!origin) throw new Refusal(400, "origin must be the playground's");
	const callback = callbackOf(env, url, provider);
	if (provider === "github") {
		if (!env.GITHUB_CLIENT_ID) throw new Refusal(501, "GitHub login is not set up yet (GITHUB_CLIENT_ID, notes/hosting.md)");
		const state = await signed(env, { origin }, STATE_SECONDS);
		return Response.redirect(`${GITHUB_AUTHORIZE}?${new URLSearchParams({ client_id: env.GITHUB_CLIENT_ID, redirect_uri: callback, state, scope: "" })}`, 302);
	}
	if (!env.CLOUDFLARE_OAUTH_CLIENT_ID) throw new Refusal(501, "Cloudflare login is not set up yet: paste an API token instead (notes/hosting.md)");
	const { verifier, challenge } = await pkcePair();
	const state = await signed(env, { origin, verifier }, STATE_SECONDS);
	return Response.redirect(`${CLOUDFLARE_AUTHORIZE}?${new URLSearchParams({ client_id: env.CLOUDFLARE_OAUTH_CLIENT_ID, redirect_uri: callback,
		response_type: "code", scope: CLOUDFLARE_SCOPES, state, code_challenge: challenge, code_challenge_method: "S256" })}`, 302);
}

async function finishLogin(request, env, provider) {
	const url = new URL(request.url);
	const state = await unsigned(env, url.searchParams.get("state"));
	if (!state) throw new Refusal(400, "the login took too long or did not start here");
	const code = url.searchParams.get("code");
	if (!code) return handOver(state.origin, { error: url.searchParams.get("error_description") ?? url.searchParams.get("error") ?? "no code" });
	const callback = callbackOf(env, url, provider);
	if (provider === "github") {
		const reply = await fetch(GITHUB_TOKEN, { method: "POST", headers: { accept: "application/json", "user-agent": USER_AGENT },
			body: new URLSearchParams({ client_id: env.GITHUB_CLIENT_ID, client_secret: env.GITHUB_CLIENT_SECRET, code, redirect_uri: callback }) });
		const { access_token, error_description } = await reply.json();
		if (!access_token) return handOver(state.origin, { error: error_description ?? "GitHub gave no token" });
		const { id, login } = await githubUser(access_token);
		return handOver(state.origin, { warpHosting: { session: SESSION_PREFIX + await signed(env, { id, login }, SESSION_SECONDS), login } });
	}
	const form = { grant_type: "authorization_code", client_id: env.CLOUDFLARE_OAUTH_CLIENT_ID, code, redirect_uri: callback, code_verifier: state.verifier };
	if (env.CLOUDFLARE_OAUTH_CLIENT_SECRET) form.client_secret = env.CLOUDFLARE_OAUTH_CLIENT_SECRET;
	const reply = await fetch(CLOUDFLARE_TOKEN, { method: "POST", headers: { accept: "application/json" }, body: new URLSearchParams(form) });
	const { access_token, error_description } = await reply.json();
	if (!access_token) return handOver(state.origin, { error: error_description ?? "Cloudflare gave no token" });
	return handOver(state.origin, { warpHosting: { cloudflareToken: access_token } });
}

// ---- Cloudflare: one script per program ------------------------------------------------------------------------

async function cloudflare(token, path, init = {}) {
	const reply = await fetch(`${CLOUDFLARE_API}${path}`, { ...init, headers: { authorization: `Bearer ${token}`, ...init.headers } });
	const answer = await reply.json().catch(() => ({ success: false, errors: [{ message: `HTTP ${reply.status}` }] }));
	if (!answer.success) throw new Refusal(reply.status === 401 || reply.status === 403 ? 403 : 502,
		`Cloudflare ${path.split("/").slice(3, 5).join("/")}: ${(answer.errors ?? []).map(error => error.message).join("; ")}`);
	return answer.result;
}

function checkedWasm(bytes) {
	if (bytes.byteLength === 0) throw new Refusal(400, "send the program's .wasm as the body");
	if (bytes.byteLength > MAX_WASM_BYTES) throw new Refusal(413, `the module is ${bytes.byteLength} bytes, over ${MAX_WASM_BYTES}`);
	if (!WASM_MAGIC.every((byte, i) => new Uint8Array(bytes)[i] === byte)) throw new Refusal(400, "the body is not a WebAssembly module");
	return bytes;
}

// POST /deploy?name=…&scripts=…, the module as the body
async function programOf(request) {
	return { wasm: checkedWasm(await request.arrayBuffer()), scripts: checkedScripts(new URL(request.url).searchParams.get("scripts")) };
}

function checkedName(name) {
	if (!NAME_PATTERN.test(name ?? "")) throw new Refusal(400, "name: lowercase letters, digits and inner dashes, up to 40");
	return name;
}

// the host scripts a deploy names (?scripts=host-files.js,…), reader.js and host.js always first
function checkedScripts(names) {
	const asked = (names ?? "").split(",").filter(Boolean).filter(name => !ALWAYS_SCRIPTS.includes(name));
	const unknown = asked.filter(name => !HOST_SCRIPTS[name]);
	if (unknown.length) throw new Refusal(400, `warp-hosting has no host script ${unknown.join(", ")}: it needs a redeploy with this warp's web/playground`);
	return [...ALWAYS_SCRIPTS, ...asked];
}

// the program's Worker as src/deploy.rs worker_files makes it: worker.js, app.wasm compiled, app.bin as bytes
function scriptForm({ wasm, scripts }) {
	const worker = [...scripts.map(name => HOST_SCRIPTS[name]), cloudWorkerScript].join("");
	const form = new FormData();
	form.append("metadata", JSON.stringify({ main_module: "worker.js", compatibility_date: COMPATIBILITY_DATE }));
	form.append("worker.js", new File([worker], "worker.js", { type: "application/javascript+module" }));
	form.append("app.wasm", new File([wasm], "app.wasm", { type: "application/wasm" }));
	form.append("app.bin", new File([wasm], "app.bin", { type: "application/octet-stream" }));
	return form;
}

async function upload({ token, account, script, program }) {
	await cloudflare(token, `/accounts/${account}/workers/scripts/${script}`, { method: "PUT", body: scriptForm(program) });
	await cloudflare(token, `/accounts/${account}/workers/scripts/${script}/subdomain`, { method: "POST",
		headers: { "content-type": "application/json" }, body: JSON.stringify({ enabled: true, previews_enabled: false }) });
	const { subdomain } = await cloudflare(token, `/accounts/${account}/workers/subdomain`);
	return `https://${script}.${subdomain}.workers.dev`;
}

// a name belongs to the first GitHub user who deploys it; the caps keep spam within the free plan (notes/hosting.md
// "Limits"), which never bills: past its daily requests the Workers answer errors until the next day
async function claim(env, name, owner) {
	const holder = await env.NAMES.get(name);
	if (holder && holder !== String(owner.id)) throw new Refusal(409, `${name} belongs to someone else: choose another name`);
	if (holder) return;
	const { keys } = await env.NAMES.list();
	if (keys.length >= Number(env.MAX_PROGRAMS)) throw new Refusal(507, "warp-hosting is full for now: deploy to your own Cloudflare account");
	const own = keys.filter(key => key.metadata?.owner === String(owner.id)).map(key => key.name);
	if (own.length >= Number(env.MAX_PROGRAMS_PER_USER)) throw new Refusal(429, `you host ${own.join(", ")} already, ${env.MAX_PROGRAMS_PER_USER} at most: redeploy one of those names or remove one (DELETE /deploy?name=…)`);
	await env.NAMES.put(name, String(owner.id), { metadata: { owner: String(owner.id), login: owner.login } });
}

async function deployHere(request, env, name) {
	const owner = await user(request, env);
	if (!env.CLOUDFLARE_API_TOKEN) throw new Refusal(501, "warp-hosting has no CLOUDFLARE_API_TOKEN yet (notes/hosting.md)");
	const program = await programOf(request);
	await claim(env, name, owner);
	const url = await upload({ token: env.CLOUDFLARE_API_TOKEN, account: env.ACCOUNT_ID, script: SCRIPT_PREFIX + name, program });
	return json({ url, owner: owner.login });
}

async function deployToOwnAccount(request, name) {
	const token = request.headers.get("x-cloudflare-token");
	let account = request.headers.get("x-cloudflare-account");
	if (!account) {
		const accounts = await cloudflare(token, "/accounts");
		if (accounts.length !== 1) throw new Refusal(400, `the token reaches ${accounts.length} accounts: say which (X-Cloudflare-Account)`);
		account = accounts[0].id;
	}
	const program = await programOf(request);
	return json({ url: await upload({ token, account, script: name, program }) });
}

async function undeploy(request, env, name) {
	const owner = await user(request, env);
	if (await env.NAMES.get(name) !== String(owner.id)) throw new Refusal(403, `${name} is not yours`);
	await cloudflare(env.CLOUDFLARE_API_TOKEN, `/accounts/${env.ACCOUNT_ID}/workers/scripts/${SCRIPT_PREFIX}${name}`, { method: "DELETE" });
	await env.NAMES.delete(name);
	return json({ deleted: name });
}

// ---- routing --------------------------------------------------------------------------------------------------

async function route(request, env) {
	const url = new URL(request.url);
	const login = url.pathname.match(/^\/auth\/(github|cloudflare)$/);
	if (login) return startLogin(request, env, login[1]);
	const callback = url.pathname.match(/^\/callback(?:\/(cloudflare))?$/);
	if (callback) return finishLogin(request, env, callback[1] ?? "github");
	if (url.pathname === "/me") return json(await user(request, env));
	if (url.pathname === "/deploy") {
		const name = checkedName(url.searchParams.get("name"));
		if (request.method === "DELETE") return undeploy(request, env, name);
		if (request.method !== "POST") throw new Refusal(405, "POST the module to /deploy?name=…&scripts=…");
		return request.headers.has("x-cloudflare-token") ? deployToOwnAccount(request, name) : deployHere(request, env, name);
	}
	if (url.pathname === "/") return new Response("warp-hosting: deploys warp programs, see https://github.com/pannous/warp/blob/main/notes/hosting.md\n");
	throw new Refusal(404, `no ${url.pathname} here`);
}

function withCors(response, request) {
	const origin = allowedOrigin(request.headers.get("origin"));
	if (!origin) return response;
	const headers = new Headers(response.headers);
	headers.set("access-control-allow-origin", origin);
	headers.set("access-control-allow-methods", "GET, POST, DELETE");
	headers.set("access-control-allow-headers", "authorization, content-type, x-cloudflare-token, x-cloudflare-account");
	headers.set("vary", "origin");
	return new Response(response.body, { status: response.status, headers });
}

export default {
	async fetch(request, env) {
		if (request.method === "OPTIONS") return withCors(new Response(null, { status: 204 }), request);
		const response = await route(request, env).catch(error => json({ error: error.message }, error.status ?? 500));
		return withCors(response, request);
	},
};
