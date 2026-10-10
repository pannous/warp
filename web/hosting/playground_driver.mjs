// Drives the playground's deploy buttons (web/playground/deploy.js) in headless agent-browser, for the hosting tests
// (test_hosting.mjs, test_lambda.mjs): the GitHub session already stored, the program in the editor, a button clicked.
import { execFileSync, spawn } from "node:child_process";
import { join, resolve } from "node:path";
import assert from "node:assert/strict";

const PLAYGROUND = join(resolve(import.meta.dirname, "../.."), "web/playground");
const SESSION_KEY = "warp-hosting-session"; // deploy.js SESSION_KEYS.github
const TRIES = 60;

const sleep = ms => new Promise(done => setTimeout(done, ms));

export async function until(condition, failure) {
	for (let i = 0; i < TRIES; i++) {
		if (condition()) return;
		await sleep(1000);
	}
	assert.fail(failure);
}

// the address the playground links once `button` deployed `source` as `name` through `hosting`, else a failure with its status
export async function deployedByPlayground({ hosting, button, name, source, token, port, session }) {
	// agent-browser's eval prints the value as JSON
	const browser = (...args) => {
		const output = execFileSync("agent-browser", ["--session", session, ...args]).toString().trim();
		try { return JSON.parse(output); } catch { return output; }
	};
	const server = spawn("python3", ["-m", "http.server", String(port), "--bind", "127.0.0.1"], { cwd: PLAYGROUND, stdio: "ignore" });
	try {
		await sleep(1000);
		browser("open", `http://localhost:${port}/?hosting=${encodeURIComponent(hosting)}`);
		await until(() => browser("eval", `typeof playground`) === "object", "the playground never started");
		browser("eval", `localStorage.setItem(${JSON.stringify(SESSION_KEY)}, ${JSON.stringify(token)})`);
		browser("eval", `playground.setCode(${JSON.stringify(source)})`);
		browser("eval", `document.getElementById("deploy-name").value = ${JSON.stringify(name)}`);
		browser("eval", `document.getElementById(${JSON.stringify(button)}).click()`);
		let status;
		for (let i = 0; i < TRIES; i++) {
			const link = browser("eval", `document.querySelector("#deployed a")?.href ?? ""`);
			if (link) return link.replace(/\/$/, "");
			status = browser("eval", `document.getElementById("status").textContent`);
			if (status.startsWith("not deployed") || status.includes("name to deploy")) break;
			await sleep(1000);
		}
		assert.fail(`the playground did not deploy: ${status}`);
	} finally {
		browser("close");
		server.kill();
	}
}
