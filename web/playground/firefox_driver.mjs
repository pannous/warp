// Headless Firefox for test_in_browser.py --firefox, over WebDriver BiDi: `node firefox_driver.mjs`, then one JSON
// command per line on stdin, one JSON answer per line on stdout:
//   ["open", url] → true once loaded · ["eval", js] → the value as agent-browser prints it (JSON text)
//   ["messages"] → the console errors and warnings and failed requests since the last ask, workers included
//   ["close"] → quits
// FIREFOX names the binary (default `firefox`; on macOS the app, default /Applications/Firefox.app, started by `open`:
// a terminal's child may not read ~/Library/Application Support/Firefox, which Firefox needs even with --profile).
// A fresh profile per run, so the user's own Firefox is never touched.
import { spawn } from "node:child_process";
import { createServer } from "node:net";
import { mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { createInterface } from "node:readline";

const MAC = process.platform === "darwin";
const FIREFOX = process.env.FIREFOX ?? (MAC ? "/Applications/Firefox.app" : "firefox");
const START_SECONDS = 60;
const REPORTED_LEVELS = new Set(["error", "warn", "warning", "assert"]);
const FAILED_STATUS = 400;

const freePort = () => new Promise(found => {
	const server = createServer().listen(0, "127.0.0.1", () => {
		const { port } = server.address();
		server.close(() => found(port));
	});
});
const sleep = milliseconds => new Promise(done => setTimeout(done, milliseconds));

// the BiDi socket of a Firefox started now, once it listens
async function startFirefox(profile) {
	const port = await freePort();
	const options = ["--headless", "--no-remote", "--profile", profile, "--remote-debugging-port", String(port)];
	spawn(MAC ? "open" : FIREFOX, MAC ? ["-n", "-g", "-a", FIREFOX, "--args", ...options] : options, { stdio: "ignore" });
	for (const deadline = Date.now() + START_SECONDS * 1000; Date.now() < deadline; await sleep(250)) {
		const socket = new WebSocket(`ws://127.0.0.1:${port}/session`);
		const opened = await new Promise(settled => { socket.onopen = () => settled(true); socket.onerror = () => settled(false); });
		if (opened) return socket;
	}
	throw new Error(`Firefox (${FIREFOX}) did not listen for WebDriver BiDi in ${START_SECONDS} s`);
}

const profile = mkdtempSync(join(tmpdir(), "warp-firefox-"));
const socket = await startFirefox(profile);
let nextId = 1;
const pending = new Map();
const messages = [];
const report = (level, text, url = "") => REPORTED_LEVELS.has(level) && messages.push({ level: level === "warn" ? "warning" : level, text, url });
const events = {
	"log.entryAdded": entry => report(entry.level, entry.text ?? entry.args?.map(arg => arg.value ?? arg.type).join(" "), entry.source?.realm ? entry.stackTrace?.callFrames?.[0]?.url : ""),
	"network.responseCompleted": ({ response }) => response.status >= FAILED_STATUS && report("error", `${response.status} ${response.statusText}`.trim(), response.url),
};
socket.onmessage = message => {
	const data = JSON.parse(message.data);
	if (data.id && pending.has(data.id)) {
		pending.get(data.id)(data);
		pending.delete(data.id);
	} else events[data.method]?.(data.params);
};
const send = (method, params = {}) => new Promise(answered => {
	const id = nextId++;
	pending.set(id, answered);
	socket.send(JSON.stringify({ id, method, params }));
});

await send("session.new", { capabilities: {} });
await send("session.subscribe", { events: Object.keys(events) });
const context = (await send("browsingContext.getTree", {})).result.contexts[0].context;

// a BiDi remote value as plain JSON
const plain = value => ({
	undefined: () => undefined, null: () => null,
	array: () => value.value.map(plain),
	object: () => Object.fromEntries(value.value.map(([key, item]) => [typeof key === "string" ? key : plain(key), plain(item)])),
})[value.type]?.() ?? value.value;

const commands = {
	open: async url => (await send("browsingContext.navigate", { context, url, wait: "complete" })).error === undefined,
	eval: async expression => {
		const answer = await send("script.evaluate", { expression, target: { context }, awaitPromise: true, resultOwnership: "none" });
		const result = answer.result;
		if (answer.error || result.type === "exception") return `✗ ${answer.message ?? result.exceptionDetails?.text}`;
		const value = plain(result.result);
		return value === undefined ? "" : JSON.stringify(value);
	},
	messages: async () => messages.splice(0),
	close: async () => {
		const closed = new Promise(done => socket.onclose = done);
		await send("browser.close");
		await closed;
		// Firefox may still write its profile while it quits
		rmSync(profile, { recursive: true, force: true, maxRetries: 20, retryDelay: 250 });
		process.exit(0);
	},
};

console.log(JSON.stringify("ready"));
for await (const line of createInterface({ input: process.stdin })) {
	const [command, ...arguments_] = JSON.parse(line);
	console.log(JSON.stringify(await commands[command](...arguments_)));
}
await commands.close();
