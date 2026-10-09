// End-to-end test of warp-hosting against the real Cloudflare API (no mocks): runs the control Worker under
// wrangler dev, deploys samples/hosting.warp by `warp deploy --hosted` as warp-hosting-test, reads its live routes on
// workers.dev, removes it.
// node web/hosting/test_hosting.mjs   needs: scratch/warp (scripts/own-warp.sh), wrangler logged in, gh logged in.
// CLOUDFLARE_API_TOKEN defaults to wrangler's own OAuth token.
import { execFileSync, spawn } from "node:child_process";
import { copyFileSync, mkdirSync, openSync, readFileSync, writeFileSync } from "node:fs";
import { homedir } from "node:os";
import { join, resolve } from "node:path";
import assert from "node:assert/strict";

const ROOT = resolve(import.meta.dirname, "../..");
const WORK = join(ROOT, "scratch/hosting_test");
const PORT = 8797;
const INSPECTOR_PORT = 9331; // not wrangler's default 9229, which another wrangler dev may hold
const HOSTING = `http://localhost:${PORT}`;
const NAME = "hosting-test";
const SAMPLE = join(ROOT, "samples/hosting.warp");
const EXPECTED_ROOT = "hello from the edge";
const EXPECTED_FIB = { n: 20, fib: 6765 };
const WRANGLER_CONFIG = join(homedir(), "Library/Preferences/.wrangler/config/default.toml");
const PROPAGATION_TRIES = 30;

const sleep = ms => new Promise(done => setTimeout(done, ms));

function cloudflareToken() {
	if (process.env.CLOUDFLARE_API_TOKEN) return process.env.CLOUDFLARE_API_TOKEN;
	execFileSync("wrangler", ["whoami"], { stdio: "ignore" }); // refreshes an expired OAuth token
	return readFileSync(WRANGLER_CONFIG, "utf8").match(/oauth_token = "([^"]+)"/)[1];
}

// the sample as hosting-test.warp: the program's file name is its Worker's name
function deployedSample(token) {
	const source = join(WORK, `${NAME}.warp`);
	copyFileSync(SAMPLE, source);
	const output = execFileSync(join(ROOT, "scratch/warp"), ["deploy", "--hosted", source],
		{ env: { ...process.env, WARP_HOSTING: HOSTING, WARP_HOSTING_TOKEN: token } }).toString();
	return output.match(/deployed: (\S+)/)?.[1] ?? assert.fail(output);
}

async function startHosting() {
	const envFile = join(WORK, "dev.vars");
	writeFileSync(envFile, `SESSION_SECRET=${crypto.randomUUID()}\nCLOUDFLARE_API_TOKEN=${cloudflareToken()}\n`);
	const log = openSync(join(WORK, "wrangler.log"), "w");
	const dev = spawn("wrangler", ["dev", "--port", String(PORT), "--inspector-port", String(INSPECTOR_PORT), "--env-file", envFile],
		{ cwd: import.meta.dirname, stdio: ["ignore", log, log] });
	for (let i = 0; i < 60; i++) {
		if (await fetch(HOSTING).then(reply => reply.ok, () => false)) return dev;
		await sleep(1000);
	}
	dev.kill();
	throw new Error(`wrangler dev did not start: ${join(WORK, "wrangler.log")}`);
}

const WASM_HEADER = new Uint8Array([0, 0x61, 0x73, 0x6d, 1, 0, 0, 0]);
const deploy = (body, headers, query = "") => fetch(`${HOSTING}/deploy?name=${NAME}${query}`, { method: "POST", body, headers });

async function liveText(url) {
	for (let i = 0; i < PROPAGATION_TRIES; i++) {
		const reply = await fetch(url).catch(() => null);
		if (reply?.ok) return reply.text();
		await sleep(2000);
	}
	throw new Error(`${url} never answered`);
}

mkdirSync(WORK, { recursive: true });
const token = execFileSync("gh", ["auth", "token"]).toString().trim();
const github = { authorization: `Bearer ${token}` };
const dev = await startHosting();
try {
	assert.equal((await deploy(WASM_HEADER, {})).status, 401, "a deploy without login is refused");
	assert.equal((await fetch(`${HOSTING}/deploy?name=Bad_Name`, { method: "POST", body: WASM_HEADER, headers: github })).status, 400);
	assert.equal((await deploy(new Uint8Array([1, 2, 3]), github)).status, 400, "only a wasm module is deployed");
	assert.equal((await deploy(WASM_HEADER, github, "&scripts=evil.js")).status, 400, "only our own host scripts");
	const url = deployedSample(token);
	assert.match(url, /^https:\/\/warp-hosting-test\.[a-z0-9-]+\.workers\.dev$/);
	assert.equal(await liveText(url), EXPECTED_ROOT);
	assert.deepEqual(JSON.parse(await liveText(`${url}/fib/20`)), EXPECTED_FIB);
	console.log(`deployed and served: ${url}`);
	const removed = await fetch(`${HOSTING}/deploy?name=${NAME}`, { method: "DELETE", headers: github });
	assert.equal(removed.status, 200, await removed.text());
	console.log("hosting test passed");
} finally {
	dev.kill();
}
