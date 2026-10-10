// Headless Firefox for test_in_browser.py --firefox, over WebDriver BiDi: `node firefox_driver.mjs`, then one JSON5
// command per line on stdin (single quotes, comments, trailing commas; a line of only a comment is skipped), one JSON
// answer per line on stdout:
//   ["open", url] → true once loaded · ["eval", js] → the value as agent-browser prints it (JSON text)
//   ["messages"] → the console errors and warnings and failed requests since the last ask, workers included
//   ["close"] → quits
// A command still unanswered after COMMAND_SECONDS answers {"timeout": what, the page's state and console}; a Firefox that went away ends the driver
// with exit code 1 and the reason on stderr (test_in_browser.py stops loudly on either, card deploy-firefox-hang).
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
// a command not answered by then (a page that never finishes loading, an example that never ends) fails the run loudly
// (FIREFOX_COMMAND_SECONDS: shorter, for a probe of that path)
const COMMAND_SECONDS = Number(process.env.FIREFOX_COMMAND_SECONDS ?? 120);
const STATE_SECONDS = 10; // then the page's state, asked after such a timeout
// Firefox writes its profile until it has quit (card firefox-profile: ENOTEMPTY removing it right after the socket closed)
const QUIT_SECONDS = 30;
const REPORTED_LEVELS = new Set(["error", "warn", "warning", "assert"]);
const FAILED_STATUS = 400;

const freePort = () => new Promise(found => {
	const server = createServer().listen(0, "127.0.0.1", () => {
		const { port } = server.address();
		server.close(() => found(port));
	});
});
const sleep = milliseconds => new Promise(done => setTimeout(done, milliseconds));

// the BiDi socket of a Firefox started now, once it listens, and a promise of its exit (`open -W` waits for the app)
async function startFirefox(profile) {
	const port = await freePort();
	const options = ["--headless", "--no-remote", "--profile", profile, "--remote-debugging-port", String(port)];
	const firefox = spawn(MAC ? "open" : FIREFOX, MAC ? ["-n", "-g", "-W", "-a", FIREFOX, "--args", ...options] : options, { stdio: "ignore" });
	const exited = new Promise(done => firefox.on("exit", done));
	for (const deadline = Date.now() + START_SECONDS * 1000; Date.now() < deadline; await sleep(250)) {
		const socket = new WebSocket(`ws://127.0.0.1:${port}/session`);
		const opened = await new Promise(settled => { socket.onopen = () => settled(true); socket.onerror = () => settled(false); });
		if (opened) return { socket, exited };
	}
	throw new Error(`Firefox (${FIREFOX}) did not listen for WebDriver BiDi in ${START_SECONDS} s`);
}

const profile = mkdtempSync(join(tmpdir(), "warp-firefox-"));
const { socket, exited } = await startFirefox(profile);
let nextId = 1;
const pending = new Map();
const messages = [];
const report = (level, text, url = "") => REPORTED_LEVELS.has(level) && messages.push({ level: level === "warn" ? "warning" : level, text, url });
const events = {
	"log.entryAdded": entry => report(entry.level, entry.text ?? entry.args?.map(arg => arg.value ?? arg.type).join(" "), entry.source?.realm ? entry.stackTrace?.callFrames?.[0]?.url : ""),
	"network.responseCompleted": ({ response }) => response.status >= FAILED_STATUS && report("error", `${response.status} ${response.statusText}`.trim(), response.url),
};
socket.onclose = () => {
	console.error(`firefox_driver.mjs: Firefox closed its WebDriver BiDi connection; ${pending.size} commands unanswered`);
	process.exit(1);
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
		await Promise.race([exited, sleep(QUIT_SECONDS * 1000)]);
		try {
			rmSync(profile, { recursive: true, force: true, maxRetries: 20, retryDelay: 250 });
		} catch (error) {
			console.error(`firefox_driver.mjs: warning: the profile ${profile} stays behind: ${error.message}`);
		}
		process.exit(0);
	},
};

// what the page was doing when a command got no answer: where it is, whether it is isolated, the playground's status
const PAGE_STATE = `({ address: location.href, loaded: document.readyState, isolated: self.crossOriginIsolated,
	controlled: !!navigator.serviceWorker?.controller, status: document.getElementById("status")?.textContent,
	loading: document.getElementById("loading")?.hidden === false ? document.getElementById("loading").textContent : "",
	playground: window.playground?.state?.() })`;
const unanswered = seconds => sleep(seconds * 1000).then(() => `no answer in ${seconds} s`);

// the command's answer, or {timeout} naming it, the page's state and its console when none came in COMMAND_SECONDS
// (card firefox-hello-hang: a cold runner's first example once waited forever)
const answeredInTime = (answer, line) => Promise.race([answer, sleep(COMMAND_SECONDS * 1000).then(async () => {
	const state = await Promise.race([commands.eval(PAGE_STATE), unanswered(STATE_SECONDS)]);
	const logged = messages.splice(0).map(({ level, text, url }) => `${level}: ${text}${url ? ` (${url})` : ""}`);
	return { timeout: `no answer to ${line.slice(0, 200)} in ${COMMAND_SECONDS} s; the page: ${state}; its console: ${JSON.stringify(logged)}` };
})]);

// a JSON5 value (user: no more JSON's quoting and missing comments for what people write): JSON5 is a subset of
// JavaScript's literals, so the line is read as one; stdin is the driver's own caller, trusted like its scripts
const json5Value = text => new Function(`"use strict"; return (${text}\n);`)();
const isBlank = line => /^\s*(\/\/.*)?$/.test(line);

console.log(JSON.stringify("ready"));
for await (const line of createInterface({ input: process.stdin })) {
	if (isBlank(line)) continue;
	const [command, ...arguments_] = json5Value(line);
	console.log(JSON.stringify(await answeredInTime(commands[command](...arguments_), line)));
}
await commands.close();
