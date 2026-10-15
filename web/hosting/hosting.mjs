// warp-hosting: the control Worker that hosts warp programs (notes/hosting.md, card cloud-hosting), at
// https://lambda.pannous.com (pannous.com's Apache proxies it to warp-hosting.pannous.workers.dev).
// Users log in with GitHub and deploy a program's module, which becomes the Worker warp-<name> in our Cloudflare
// account, at https://warp-<name>.pannous.workers.dev. "Deploy to my own account" does the same upload with the
// user's own Cloudflare token, through here, as api.cloudflare.com allows no browser (CORS) calls.
// The program's Worker is the one `warp deploy` builds (src/web/deploy.rs): the host scripts its module needs, then
// cloud-worker.js, composed here from our own copies, so a deploy carries no JavaScript of its own.
import readerScript from "../playground/reader.js";
import hostScript from "../playground/host.js";
import importsScript from "../playground/imports.js";
import exactScript from "../playground/exact.js";
import stdScript from "../playground/std.js";
import filesScript from "../playground/host-files.js";
import soundScript from "../playground/host-sound.js";
import hashesScript from "../playground/host-hashes.js";
import tasksScript from "../playground/host-tasks.js";
import jsMembersScript from "../playground/js-members.js";
import foreignScript from "../playground/host-foreign.js";
import compilerScript from "../playground/host-compiler.js";
import routesScript from "../playground/host-routes.js";
import gpuScript from "../playground/host-gpu.js";
import timersScript from "../playground/host-timers.js";
import randomScript from "../playground/host-random.js";
import mediaScript from "../playground/host-media.js";
import cloudWorkerScript from "../playground/cloud-worker.js";
import cloudSiteScript from "../playground/cloud-site.js";
import markupScript from "../playground/markup.js";
import markupTransitionsScript from "../playground/markup-transitions.js";
import markupFormsScript from "../playground/markup-forms.js";
import siteScript from "../playground/site.js";
import canvasScript from "../playground/canvas.js";
import siteThreadScript from "../playground/site-thread.js";
import taskWorkersScript from "../playground/task-workers.js";
import siteWorkerScript from "../playground/site-worker.js";
import taskWorkerScript from "../playground/task-worker.js";
import coiServiceWorkerScript from "../playground/coi-serviceworker.js";

// the host scripts by the names src/web/site.rs host_scripts_of gives them (HOST_SCRIPTS, HOST_PARTS)
const HOST_SCRIPTS = {
	"reader.js": readerScript,
	"host.js": hostScript,
	"host-files.js": stdScript + filesScript,
	"host-sound.js": soundScript,
	"host-hashes.js": stdScript + hashesScript,
	"host-tasks.js": importsScript + exactScript + tasksScript,
	"host-foreign.js": exactScript + jsMembersScript + foreignScript,
	"host-compiler.js": exactScript + compilerScript,
	"host-routes.js": importsScript + routesScript,
	"host-gpu.js": gpuScript,
	"host-timers.js": timersScript,
	"host-random.js": randomScript,
	"host-media.js": mediaScript,
};
const ALWAYS_SCRIPTS = ["reader.js", "host.js"];
// the files a browser program's site ships besides the host scripts, by the names src/host/page_parts.rs gives them
const SITE_FILES = {
	...HOST_SCRIPTS,
	"markup.js": markupScript,
	"markup-transitions.js": markupTransitionsScript,
	"markup-forms.js": markupFormsScript,
	"site.js": siteScript,
	"canvas.js": canvasScript,
	"site-thread.js": siteThreadScript,
	"task-workers.js": jsMembersScript + taskWorkersScript,
	"site-worker.js": siteWorkerScript,
	"task-worker.js": taskWorkerScript,
	"coi-serviceworker.js": coiServiceWorkerScript,
};

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
// a login popup's ticket (deploy.js): GitHub's login page cuts the popup off from the playground (window.opener is null
// after it), so the playground fetches the login by its ticket, and the popup becomes the deployed program's tab
const TICKET_PREFIX = "ticket:";
const TICKET_PATTERN = /^[A-Za-z0-9_-]{22,64}$/;
const TICKET_SECONDS = 600; // KV keeps a value at least 60 s
const TICKET_POLL_MS = 1500;
const DEPLOYING_PATH = "/deploying";
const PROGRAM_LATE_MS = 60 * 1000; // a program that doesn't answer a minute after its link shows gets a note
const UP_TIMEOUT_MS = 5000;
const CLOUDFLARE_ERROR = "error code:";
const PROGRAM_ADDRESS = /^https:\/\/[a-z0-9-]+(\.[a-z0-9-]+)*\.(workers\.dev|lambda\.pannous\.com)\/?$/;

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

// the login's answer to the playground: by postMessage while the popup still has its opener, always under the ticket;
// with a ticket the window then becomes the deploy window (deployingPage), which turns into the program's tab
async function handOver(env, state, message) {
	const { origin, ticket } = state;
	if (ticket) await storeTicket(env, ticket, { login: message });
	const posted = `if (opener) opener.postMessage(${JSON.stringify(message)}, ${JSON.stringify(origin)});`;
	const deploying = ticket && !message.error ? `location.replace("${DEPLOYING_PATH}?ticket=${ticket}");` : "";
	const login = message.warpHosting?.login ? ` as ${escapedHtml(message.warpHosting.login)}` : "";
	const text = message.error ? "Login failed: " + escapedHtml(message.error) : `Logged in${login}, ${ticket ? "deploying…" : "you can close this window."}`;
	return page(text, posted + deploying);
}

const page = (html, script = "") => new Response(`<!doctype html><meta charset="utf-8"><title>warp hosting</title><p id="status">${html}</p><script>${script}</script>`,
	{ headers: { "content-type": "text/html; charset=utf-8" } });

function escapedHtml(text) {
	return String(text).replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/"/g, "&quot;");
}

// what the deploy window says about a ticket, {login, program, error}; runs here and, as its source, in the window
function deployingStatus({ login, program, error }, late) {
	const who = login?.warpHosting?.login ? `Logged in as ${escapedHtml(login.warpHosting.login)}. ` : "";
	if (error) return `${who}Not deployed: ${escapedHtml(error)}`;
	if (!program) return `${who}Deploying: the program's link shows here as soon as it is known.`;
	const link = `<a href="${escapedHtml(program)}">${escapedHtml(program)}</a>`;
	return `${who}Deploying at ${link}: ${late ? "it does not answer yet, the link works once it does." : "this window opens it as soon as it answers…"}`;
}

// GET /deploying?ticket=: the deploy window (card user-click): the program's link as soon as the ticket knows it, and
// the program itself once it answers (/up), which on a fresh workers.dev or lambda.pannous.com name takes a while
async function deployingPage(request, env) {
	const ticket = checkedTicket(new URL(request.url).searchParams.get("ticket"));
	const state = await ticketState(env, ticket);
	const waiting = `${escapedHtml}${deployingStatus}
		const status = document.getElementById("status");
		const read = address => fetch(address).then(reply => reply.json()).catch(() => ({}));
		let known, shown;
		const wait = async () => {
			const state = await read("/ticket?ticket=${ticket}");
			known ??= state.program && Date.now();
			const late = known && Date.now() - known > ${PROGRAM_LATE_MS};
			const text = deployingStatus(state, late);
			if (shown !== text) status.innerHTML = shown = text;
			if (state.error) return;
			if (state.program && (await read("/up?program=" + encodeURIComponent(state.program))).up) return location.replace(state.program);
			setTimeout(wait, ${TICKET_POLL_MS});
		};
		wait();`;
	return page(deployingStatus(state), state.error ? "" : waiting);
}

function checkedProgram(program) {
	if (!PROGRAM_ADDRESS.test(program ?? "")) throw new Refusal(400, `${program} is no deployed program's address`);
	return program;
}

// GET /up?program=: does the program answer yet; Cloudflare answers "error code: 1042" for a workers.dev name that
// has no Worker (yet), and a new lambda.pannous.com name fails until its certificate is issued
async function upRequest(request) {
	const program = checkedProgram(new URL(request.url).searchParams.get("program"));
	const reply = await fetch(program, { redirect: "manual", signal: AbortSignal.timeout(UP_TIMEOUT_MS) }).catch(() => null);
	const up = Boolean(reply) && reply.status < 500 && !(await reply.text()).startsWith(CLOUDFLARE_ERROR);
	return json({ up });
}

function checkedTicket(ticket) {
	if (!TICKET_PATTERN.test(ticket ?? "")) throw new Refusal(400, "no ticket: ?ticket= of 22 to 64 letters, digits, - and _");
	return ticket;
}

const ticketState = async (env, ticket) => JSON.parse(await env.NAMES.get(TICKET_PREFIX + ticket) ?? "{}");

async function storeTicket(env, ticket, value) {
	const stored = await ticketState(env, ticket);
	await env.NAMES.put(TICKET_PREFIX + ticket, JSON.stringify({ ...stored, ...value }), { expirationTtl: TICKET_SECONDS });
}

// GET /ticket?ticket=: what the ticket holds, {login, program, error}; POST /ticket?ticket=&program= (or &error=): the
// playground says where the program went, and the login popup waiting on it goes there
async function ticketRequest(request, env) {
	const url = new URL(request.url);
	const ticket = checkedTicket(url.searchParams.get("ticket"));
	if (request.method === "GET") return json(await ticketState(env, ticket));
	const [program, error] = ["program", "error"].map(field => url.searchParams.get(field));
	if (program) checkedProgram(program);
	if (!program && !error) throw new Refusal(400, "POST /ticket?ticket=…&program=… or &error=…");
	await storeTicket(env, ticket, program ? { program } : { error: error.slice(0, 500) });
	return json({ stored: true });
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
	const ticket = url.searchParams.has("ticket") ? checkedTicket(url.searchParams.get("ticket")) : undefined;
	const callback = callbackOf(env, url, provider);
	if (provider === "github") {
		if (!env.GITHUB_CLIENT_ID) throw new Refusal(501, "GitHub login is not set up yet (GITHUB_CLIENT_ID, notes/hosting.md)");
		const state = await signed(env, { origin, ticket }, STATE_SECONDS);
		return Response.redirect(`${GITHUB_AUTHORIZE}?${new URLSearchParams({ client_id: env.GITHUB_CLIENT_ID, redirect_uri: callback, state, scope: "" })}`, 302);
	}
	if (!env.CLOUDFLARE_OAUTH_CLIENT_ID) throw new Refusal(501, "Cloudflare login is not set up yet: paste an API token instead (notes/hosting.md)");
	const { verifier, challenge } = await pkcePair();
	const state = await signed(env, { origin, verifier, ticket }, STATE_SECONDS);
	return Response.redirect(`${CLOUDFLARE_AUTHORIZE}?${new URLSearchParams({ client_id: env.CLOUDFLARE_OAUTH_CLIENT_ID, redirect_uri: callback,
		response_type: "code", scope: CLOUDFLARE_SCOPES, state, code_challenge: challenge, code_challenge_method: "S256" })}`, 302);
}

async function finishLogin(request, env, provider) {
	const url = new URL(request.url);
	const state = await unsigned(env, url.searchParams.get("state"));
	if (!state) throw new Refusal(400, "the login took too long or did not start here");
	const code = url.searchParams.get("code");
	if (!code) return handOver(env, state, { error: url.searchParams.get("error_description") ?? url.searchParams.get("error") ?? "no code" });
	const callback = callbackOf(env, url, provider);
	if (provider === "github") {
		const reply = await fetch(GITHUB_TOKEN, { method: "POST", headers: { accept: "application/json", "user-agent": USER_AGENT },
			body: new URLSearchParams({ client_id: env.GITHUB_CLIENT_ID, client_secret: env.GITHUB_CLIENT_SECRET, code, redirect_uri: callback }) });
		const { access_token, error_description } = await reply.json();
		if (!access_token) return handOver(env, state, { error: error_description ?? "GitHub gave no token" });
		const { id, login } = await githubUser(access_token);
		return handOver(env, state, { warpHosting: { session: SESSION_PREFIX + await signed(env, { id, login }, SESSION_SECONDS), login } });
	}
	const form = { grant_type: "authorization_code", client_id: env.CLOUDFLARE_OAUTH_CLIENT_ID, code, redirect_uri: callback, code_verifier: state.verifier };
	if (env.CLOUDFLARE_OAUTH_CLIENT_SECRET) form.client_secret = env.CLOUDFLARE_OAUTH_CLIENT_SECRET;
	const reply = await fetch(CLOUDFLARE_TOKEN, { method: "POST", headers: { accept: "application/json" }, body: new URLSearchParams(form) });
	const { access_token, error_description } = await reply.json();
	if (!access_token) return handOver(env, state, { error: error_description ?? "Cloudflare gave no token" });
	return handOver(env, state, { warpHosting: { cloudflareToken: access_token } });
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

// POST /deploy?name=…&scripts=…, the module as the body; a browser program's site (card deploy-browser-site) names
// its page's scripts, those of the program's Worker and every file shipped instead: ?site=…&worker=…&files=…
async function programOf(request, name) {
	const asked = new URL(request.url).searchParams;
	const wasm = checkedWasm(await request.arrayBuffer());
	if (!asked.has("site")) return { wasm, scripts: checkedScripts(asked.get("scripts")) };
	const [scripts, workerScripts, files] = ["site", "worker", "files"].map(key => checkedSiteFiles(asked.get(key)));
	return { wasm, site: { title: name, scripts, workerScripts, files: Object.fromEntries(files.map(file => [file, SITE_FILES[file]])) } };
}

function checkedSiteFiles(names) {
	const asked = (names ?? "").split(",").filter(Boolean);
	const unknown = asked.filter(name => !SITE_FILES[name]);
	if (unknown.length) throw new Refusal(400, `warp-hosting has no site file ${unknown.join(", ")}: it needs a redeploy with this warp's web/playground`);
	return asked;
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

// the program's Worker as src/web/deploy.rs worker_files makes it: worker.js, app.wasm compiled, app.bin as bytes; a
// site's worker.js serves the site (cloud-site.js) and needs only app.bin
function scriptForm({ wasm, scripts, site }) {
	const worker = site ? `const SITE = ${JSON.stringify(site)};\n${cloudSiteScript}` : [...scripts.map(name => HOST_SCRIPTS[name]), cloudWorkerScript].join("");
	const form = new FormData();
	form.append("metadata", JSON.stringify({ main_module: "worker.js", compatibility_date: COMPATIBILITY_DATE }));
	form.append("worker.js", new File([worker], "worker.js", { type: "application/javascript+module" }));
	if (!site) form.append("app.wasm", new File([wasm], "app.wasm", { type: "application/wasm" }));
	form.append("app.bin", new File([wasm], "app.bin", { type: "application/octet-stream" }));
	return form;
}

// the program's address goes under the deploy's ticket (?ticket=) before the upload, so the deploy window links it at once
async function upload(env, request, { token, account, script, program }) {
	const { subdomain } = await cloudflare(token, `/accounts/${account}/workers/subdomain`);
	const url = `https://${script}.${subdomain}.workers.dev`;
	const ticket = new URL(request.url).searchParams.get("ticket");
	if (ticket) await storeTicket(env, checkedTicket(ticket), { program: url });
	await cloudflare(token, `/accounts/${account}/workers/scripts/${script}`, { method: "PUT", body: scriptForm(program) });
	await cloudflare(token, `/accounts/${account}/workers/scripts/${script}/subdomain`, { method: "POST",
		headers: { "content-type": "application/json" }, body: JSON.stringify({ enabled: true, previews_enabled: false }) });
	return url;
}

// a Worker nobody claimed, as warp-hosting itself: the account's own, never a program's name
async function scriptExists(env, script) {
	const reply = await fetch(`${CLOUDFLARE_API}/accounts/${env.ACCOUNT_ID}/workers/scripts/${script}/settings`, { headers: { authorization: `Bearer ${env.CLOUDFLARE_API_TOKEN}` } });
	if (reply.status === 404) return false;
	if (!reply.ok) throw new Refusal(502, `Cloudflare workers/scripts: HTTP ${reply.status}`);
	return true;
}

// a name belongs to the first GitHub user who deploys it; the caps keep spam within the free plan (notes/hosting.md
// "Limits"), which never bills: past its daily requests the Workers answer errors until the next day
async function claim(env, name, owner) {
	const holder = await env.NAMES.get(name);
	if (holder && holder !== String(owner.id)) throw new Refusal(409, `${name} belongs to someone else: choose another name`);
	if (holder) return;
	if (await scriptExists(env, SCRIPT_PREFIX + name)) throw new Refusal(409, `${name} is taken by this account's own Worker: choose another name`);
	const keys = (await env.NAMES.list()).keys.filter(key => !key.name.startsWith(TICKET_PREFIX));
	if (keys.length >= Number(env.MAX_PROGRAMS)) throw new Refusal(507, "warp-hosting is full for now: deploy to your own Cloudflare account");
	const own = keys.filter(key => key.metadata?.owner === String(owner.id)).map(key => key.name);
	if (own.length >= Number(env.MAX_PROGRAMS_PER_USER)) throw new Refusal(429, `you host ${own.join(", ")} already, ${env.MAX_PROGRAMS_PER_USER} at most: redeploy one of those names or remove one (DELETE /deploy?name=…)`);
	await env.NAMES.put(name, String(owner.id), { metadata: { owner: String(owner.id), login: owner.login } });
}

async function deployHere(request, env, name) {
	const owner = await user(request, env);
	if (!env.CLOUDFLARE_API_TOKEN) throw new Refusal(501, "warp-hosting has no CLOUDFLARE_API_TOKEN yet (notes/hosting.md)");
	const program = await programOf(request, name);
	await claim(env, name, owner);
	const url = await upload(env, request, { token: env.CLOUDFLARE_API_TOKEN, account: env.ACCOUNT_ID, script: SCRIPT_PREFIX + name, program });
	return json({ url, owner: owner.login });
}

async function deployToOwnAccount(request, env, name) {
	const token = request.headers.get("x-cloudflare-token");
	let account = request.headers.get("x-cloudflare-account");
	if (!account) {
		const accounts = await cloudflare(token, "/accounts");
		if (accounts.length !== 1) throw new Refusal(400, `the token reaches ${accounts.length} accounts: say which (X-Cloudflare-Account)`);
		account = accounts[0].id;
	}
	const program = await programOf(request, name);
	return json({ url: await upload(env, request, { token, account, script: name, program }) });
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
	if (url.pathname === "/ticket") return ticketRequest(request, env);
	if (url.pathname === DEPLOYING_PATH) return deployingPage(request, env);
	if (url.pathname === "/up") return upRequest(request);
	if (url.pathname === "/deploy") {
		const name = checkedName(url.searchParams.get("name"));
		if (request.method === "DELETE") return undeploy(request, env, name);
		if (request.method !== "POST") throw new Refusal(405, "POST the module to /deploy?name=…&scripts=…");
		return request.headers.has("x-cloudflare-token") ? deployToOwnAccount(request, env, name) : deployHere(request, env, name);
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
