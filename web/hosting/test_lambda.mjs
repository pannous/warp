// End-to-end test of warp-lambda (web/hosting/server/warp_lambda.py), the native hosting on pannous.com, without
// systemd: the daemon runs its programs as child processes (WARP_LAMBDA_RUNNER=process), asks the live warp-hosting
// Worker who is logged in (a GitHub token, gh auth token), and serves the deployed program by its Host name. Then the
// playground's ☁ Deploy to pannous.com button deploys it too (headless agent-browser).
// node web/hosting/test_lambda.mjs   needs: scratch/warp (built from this checkout), gh logged in, python3, agent-browser.
import { execFileSync, spawn } from "node:child_process";
import { mkdirSync, readFileSync, rmSync } from "node:fs";
import { get } from "node:http";
import { join, resolve } from "node:path";
import assert from "node:assert/strict";
import { deployedByPlayground } from "./playground_driver.mjs";

const ROOT = resolve(import.meta.dirname, "../..");
const STATE = join(ROOT, "scratch/lambda_test");
const PORT = Number(process.env.WARP_LAMBDA_TEST_PORT ?? 8894); // outside the test suite's 87xx ports
const DAEMON = `http://127.0.0.1:${PORT}`;
const DOMAIN = "lambda.pannous.com";
const NAME = "lambda-test";
const PLAYGROUND_NAME = "lambda-ui-test";
const PLAYGROUND_PORT = Number(process.env.WARP_LAMBDA_PLAYGROUND_PORT ?? 8898);
const SAMPLE = join(ROOT, "samples/hosting.warp");
const EXPECTED_FIB = { n: 20, fib: 6765 };
const STARTS = 40;

const sleep = ms => new Promise(done => setTimeout(done, ms));
const token = execFileSync("gh", ["auth", "token"]).toString().trim();
const github = { authorization: `Bearer ${token}` };
const deploy = (name, source, headers = github) => fetch(`${DAEMON}/native/deploy?name=${name}`, { method: "POST", body: source, headers });
const remove = name => fetch(`${DAEMON}/native/deploy?name=${name}`, { method: "DELETE", headers: github });
const asked = async domain => (await fetch(`${DAEMON}/native/ask?domain=${domain}`)).status;
// the daemon reached as Ferron reaches it, the program's name in the Host header
const hosted = (name, path) => new Promise((done, failed) => {
	get({ host: "127.0.0.1", port: PORT, path, headers: { host: `${name}.${DOMAIN}` } }, reply => {
		let body = "";
		reply.on("data", chunk => body += chunk).on("end", () => done({ status: reply.statusCode, body }));
	}).on("error", failed);
});

async function startDaemon() {
	rmSync(STATE, { recursive: true, force: true });
	mkdirSync(STATE, { recursive: true });
	const env = { ...process.env, WARP_LAMBDA_PORT: String(PORT), WARP_LAMBDA_STATE: STATE, WARP_LAMBDA_RUNNER: "process",
		WARP_LAMBDA_WARP: join(ROOT, "scratch/warp"), WARP_LAMBDA_PROGRAM_ADDRESS: "127.0.0.1" };
	const daemon = spawn("python3", [join(ROOT, "web/hosting/server/warp_lambda.py")], { env, stdio: "inherit" });
	for (let i = 0; i < STARTS; i++) {
		if (await fetch(`${DAEMON}/native/ask?domain=x`).then(() => true, () => false)) return daemon;
		await sleep(250);
	}
	daemon.kill();
	throw new Error("warp-lambda did not start");
}

const daemon = await startDaemon();
try {
	assert.equal((await deploy(NAME, "1", {})).status, 401, "a deploy without login is refused");
	assert.equal((await deploy("Bad_Name", "1")).status, 400);
	assert.equal(await asked(`${NAME}.${DOMAIN}`), 404, "no certificate for a name nobody deployed");
	const deployed = await deploy(NAME, readFileSync(SAMPLE));
	assert.equal(deployed.status, 200, await deployed.clone().text());
	assert.equal((await deployed.json()).url, `https://${NAME}.${DOMAIN}`);
	assert.equal(await asked(`${NAME}.${DOMAIN}`), 200);
	assert.deepEqual(JSON.parse((await hosted(NAME, "/fib/20")).body), EXPECTED_FIB);
	assert.equal((await hosted("nobody", "/")).status, 404);
	// sandboxed: C is refused, the deploy says why
	const unsafe = await deploy("lambda-ffi-test", 'import strlen from "c"\nget "/" { strlen("ab") }');
	assert.equal(unsafe.status, 400);
	assert.match((await unsafe.json()).error, /capability denied/);
	const playgroundUrl = await deployedByPlayground({ hosting: DAEMON, button: "deploy-native", name: PLAYGROUND_NAME,
		source: readFileSync(SAMPLE, "utf8"), token, port: PLAYGROUND_PORT, session: "warp-lambda-test" });
	assert.equal(playgroundUrl, `https://${PLAYGROUND_NAME}.${DOMAIN}`);
	assert.deepEqual(JSON.parse((await hosted(PLAYGROUND_NAME, "/fib/20")).body), EXPECTED_FIB);
	assert.equal((await remove(PLAYGROUND_NAME)).status, 200);
	assert.equal((await remove(NAME)).status, 200);
	assert.equal((await hosted(NAME, "/")).status, 404);
	assert.equal(await asked(`${NAME}.${DOMAIN}`), 404);
	console.log("warp-lambda test passed");
} finally {
	daemon.kill();
}
