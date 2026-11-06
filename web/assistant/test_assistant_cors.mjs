// CORS and COEP opt-in of warp-assistant (assistant.mjs), without wrangler or an API key: the Worker answers only the
// playground Origin, with ACAO + CORP for that Origin and Vary on every answer so a refusal is not reused for it.
import assert from "node:assert/strict";
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { pathToFileURL } from "node:url";

const PLAYGROUND = "https://warp.pannous.com";
const source = readFileSync(new URL("./assistant.mjs", import.meta.url), "utf8")
	.replace(/^import ASSISTANT from .*?;\n/m, 'const ASSISTANT = { intro: "", guides: [], tasks: {}, proxy_model: "claude-haiku-5-5" };\n');
const work = mkdtempSync(join(tmpdir(), "assistant-cors-"));
const file = join(work, "assistant.mjs");
writeFileSync(file, source);
const { default: worker } = await import(pathToFileURL(file).href);
rmSync(work, { recursive: true, force: true });

const env = { PLAYGROUND_ORIGIN: PLAYGROUND };
const ask = (url, init) => worker.fetch(new Request(url, init), env);

{
	const refused = await ask("https://warp-assistant.test/session", {
		method: "POST",
		headers: { origin: "https://example.com", "content-type": "application/json" },
		body: "{}",
	});
	assert.equal(refused.status, 403, "another Origin is refused");
	assert.equal(refused.headers.get("access-control-allow-origin"), null, "a foreign Origin is not reflected");
	assert.equal(refused.headers.get("cross-origin-resource-policy"), null, "CORP is only for the playground");
	assert.match(refused.headers.get("vary") ?? "", /origin/i, "refusals Vary on Origin");
}

{
	const preflight = await ask("https://warp-assistant.test/session", {
		method: "OPTIONS",
		headers: { origin: PLAYGROUND, "access-control-request-method": "POST" },
	});
	assert.equal(preflight.status, 200);
	assert.equal(preflight.headers.get("access-control-allow-origin"), PLAYGROUND);
	assert.equal(preflight.headers.get("cross-origin-resource-policy"), "cross-origin");
	assert.match(preflight.headers.get("access-control-allow-methods") ?? "", /POST/);
	assert.match(preflight.headers.get("vary") ?? "", /origin/i);
}

{
	// allowed Origin, missing secrets: still CORS-readable so the page can show the proxy's own reason
	const unset = await ask("https://warp-assistant.test/session", {
		method: "POST",
		headers: { origin: PLAYGROUND, "content-type": "application/json" },
		body: '{"turnstile":"x"}',
	});
	assert.equal(unset.status, 503);
	assert.equal(unset.headers.get("access-control-allow-origin"), PLAYGROUND);
	assert.equal(unset.headers.get("cross-origin-resource-policy"), "cross-origin");
	const body = await unset.json();
	assert.match(body.error.message, /not set up/i);
}

console.log("assistant CORS: all checks passed");
