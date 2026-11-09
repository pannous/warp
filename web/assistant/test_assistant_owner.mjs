// The owner's token opens a session without the Turnstile check (card owner-token-session), the Worker of assistant.mjs
// itself, without wrangler or an API key: a session is signed and needs no network. A wrong token, none, and a Worker
// without OWNER_TOKEN_HASH refuse; a visitor's Turnstile token still goes to Cloudflare.
import assert from "node:assert/strict";
import { createHash, timingSafeEqual } from "node:crypto";
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { pathToFileURL } from "node:url";

crypto.subtle.timingSafeEqual ??= timingSafeEqual; // the Workers runtime has it on crypto.subtle, Node beside it
const PLAYGROUND = "https://warp.pannous.com";
const OWNER_TOKEN = "owner-token-of-this-test";
const source = readFileSync(new URL("./assistant.mjs", import.meta.url), "utf8")
	.replace(/^import ASSISTANT from .*?;\n/m, 'const ASSISTANT = { intro: "", guides: [], tasks: {}, proxy_model: "claude-haiku-5-5" };\n');
const work = mkdtempSync(join(tmpdir(), "assistant-owner-"));
const file = join(work, "assistant.mjs");
writeFileSync(file, source);
const { default: worker } = await import(pathToFileURL(file).href);
rmSync(work, { recursive: true, force: true });

const environment = {
	PLAYGROUND_ORIGIN: PLAYGROUND, ANTHROPIC_API_KEY: "unused", TURNSTILE_SECRET: "unused", SESSION_SECRET: "session-secret-of-this-test",
	OWNER_TOKEN_HASH: createHash("sha256").update(OWNER_TOKEN).digest("hex"),
	PER_ADDRESS: { limit: async () => ({ success: true }) }, // Cloudflare's rate limiter binding, not this Worker's code
};
const openSession = (body, env = environment) => worker.fetch(new Request("https://warp-assistant.test/session", {
	method: "POST", headers: { origin: PLAYGROUND, "content-type": "application/json", "cf-connecting-ip": "203.0.113.7" }, body: JSON.stringify(body),
}), env);

{
	const opened = await openSession({ owner_token: OWNER_TOKEN });
	assert.equal(opened.status, 200, "the owner's token opens a session");
	const { session } = await opened.json();
	assert.match(session, /^\d+\.[\w-]+$/, "a signed session");
	const asked = await worker.fetch(new Request("https://warp-assistant.test/messages", {
		method: "POST", headers: { origin: PLAYGROUND, "content-type": "application/json", authorization: `Bearer ${session}`, "cf-connecting-ip": "203.0.113.7" },
		body: JSON.stringify({ task: "nothing", messages: [] }),
	}), environment);
	assert.equal(asked.status, 400, "the session is accepted: only the task is wrong");
}
assert.equal((await openSession({ owner_token: "another token" })).status, 403, "a wrong token is refused");
assert.equal((await openSession({ owner_token: "" })).status, 403, "an empty token is refused");
assert.equal((await openSession({ owner_token: OWNER_TOKEN }, { ...environment, OWNER_TOKEN_HASH: undefined })).status, 403, "no owner hash set: no owner");
{
	const visitor = await openSession({ turnstile: "XXXX.DUMMY.TOKEN.XXXX" });
	assert.equal(visitor.status, 403, "a visitor's Turnstile token is still checked at Cloudflare (a dummy fails against the real secret)");
	assert.match((await visitor.json()).error.message, /bot check failed/);
}
console.log("assistant owner token: all checks passed");
