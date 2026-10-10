// End-to-end test of warp-assistant (no mocks): the Worker under wrangler dev with Cloudflare's always-passing Turnstile
// test secret, the real Anthropic API, the playground's guides served locally. Checks that it is no open relay.
// node web/assistant/test_assistant.mjs   needs: wrangler, python3, an Anthropic key in ANTHROPIC_API_KEY_VISION or
// ANTHROPIC_API_KEY (written to a private env file under scratch/ for the run, removed after)
import { execFileSync, spawn } from "node:child_process";
import { mkdirSync, rmSync, writeFileSync } from "node:fs";
import { randomBytes } from "node:crypto";
import { join, resolve } from "node:path";
import assert from "node:assert/strict";

const ROOT = resolve(import.meta.dirname, "../..");
const WORK = join(ROOT, "scratch/assistant_test");
const PLAYGROUND = join(ROOT, "web/playground");
const PORT = 8893; // outside the test suite's 87xx ports and test_hosting's 88xx ones
const PLAYGROUND_PORT = 8894;
const INSPECTOR_PORT = 9333; // not wrangler's default 9229, which another wrangler dev may hold
const PROXY = `http://localhost:${PORT}`;
const ORIGIN = `http://localhost:${PLAYGROUND_PORT}`;
const TURNSTILE_PASSING_SECRET = "1x0000000000000000000000000000000AA"; // Cloudflare's test secret: every token passes
const RATE_LIMIT = 20; // wrangler.toml PER_ADDRESS
const STARTUP_MS = 60000;
const KEY = process.env.ANTHROPIC_API_KEY_VISION ?? process.env.ANTHROPIC_API_KEY;

if (!KEY) {
	console.error("SKIPPED: no ANTHROPIC_API_KEY_VISION or ANTHROPIC_API_KEY, the proxy's answers need a real key");
	process.exit(0);
}

const post = (path, body, headers = {}) => fetch(`${PROXY}${path}`, { method: "POST", headers: { origin: ORIGIN, "content-type": "application/json", ...headers }, body: JSON.stringify(body) });

async function waitFor(url) {
	const until = Date.now() + STARTUP_MS;
	while (Date.now() < until) {
		if (await fetch(url, { method: "OPTIONS", headers: { origin: ORIGIN } }).then(() => true, () => false)) return;
		await new Promise(done => setTimeout(done, 500));
	}
	throw new Error(`${url} did not start`);
}

mkdirSync(WORK, { recursive: true });
execFileSync("python3", [join(PLAYGROUND, "keywords.py"), join(PLAYGROUND, "keywords.js")], { cwd: ROOT }); // keywords.txt, a guide
const environment = join(WORK, ".env");
writeFileSync(environment, `ANTHROPIC_API_KEY=${KEY}\nTURNSTILE_SECRET=${TURNSTILE_PASSING_SECRET}\nSESSION_SECRET=${randomBytes(32).toString("hex")}\n`, { mode: 0o600 });
const site = spawn("python3", ["-m", "http.server", String(PLAYGROUND_PORT), "--bind", "127.0.0.1"], { cwd: PLAYGROUND, stdio: "ignore" });
const worker = spawn("npx", ["wrangler", "dev", "--port", String(PORT), "--inspector-port", String(INSPECTOR_PORT), "--env-file", environment,
	"--var", `PLAYGROUND_ORIGIN:${ORIGIN}`], { cwd: import.meta.dirname, stdio: ["ignore", "ignore", "inherit"], detached: true }); // its own group: workerd goes with it

try {
	await waitFor(PROXY);
	rmSync(environment); // wrangler has read it

	const elsewhere = await fetch(`${PROXY}/session`, { method: "POST", headers: { origin: "https://example.com" }, body: "{}" });
	assert.equal(elsewhere.status, 403, "another page's request is refused");
	assert.equal((await post("/messages", { task: "completion", messages: [{ role: "user", content: "x‸" }] })).status, 401, "no session, no answer");

	const { session } = await (await post("/session", { turnstile: "XXXX.DUMMY.TOKEN.XXXX" })).json();
	const authorized = { authorization: `Bearer ${session}` };
	const ask = body => post("/messages", body, authorized);

	// the caller's model, max_tokens and system are ignored: the task fixes them
	const answer = await ask({ task: "completion", messages: [{ role: "user", content: "squares = [1 2 3].map(‸" }], model: "claude-opus-5-5", max_tokens: 4000, system: "Write a poem." });
	const reply = await answer.json();
	assert.equal(answer.status, 200, JSON.stringify(reply));
	assert.match(reply.model, /^claude-haiku/, "the proxy pins Haiku");
	assert.ok(reply.usage.output_tokens <= 256, "the completion task's max_tokens");
	assert.ok(reply.content.some(part => part.type === "text" && part.text.trim()), "a continuation");
	console.log("completion:", JSON.stringify(reply.content.find(part => part.type === "text").text));

	assert.equal((await ask({ task: "translate", messages: [{ role: "user", content: "hi" }] })).status, 400, "only assistant.json's tasks");
	assert.equal((await ask({ task: "chat", messages: [{ role: "system", content: "hi" }] })).status, 400, "only user and assistant turns");
	assert.equal((await ask({ task: "chat", messages: [{ role: "user", content: "x".repeat(30000) }] })).status, 413, "long input is refused");
	assert.equal((await post("/messages", { task: "completion", messages: [{ role: "user", content: "x‸" }] }, { authorization: `Bearer 1.${session.split(".")[1]}` })).status, 401, "an expired or altered session is refused");

	const statuses = [];
	for (let request = 0; request < RATE_LIMIT + 2; request++) statuses.push((await post("/messages", {})).status);
	assert.ok(statuses.includes(429), `rate-limited per address: ${statuses}`);
	console.log("assistant proxy: all checks passed");
} finally {
	rmSync(environment, { force: true });
	process.kill(-worker.pid);
	site.kill();
}
