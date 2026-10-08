// Prints every console error and warning of a browser as JSON lines, workers included: `node console_watch.mjs <cdp url>`
// (agent-browser get cdp-url). agent-browser's own `console` and `errors` see only the page, while the playground's
// compiler runs in a Worker; this attaches to every target over the DevTools protocol (test_in_browser.py --examples).
// A failed request (status 400 or more) is an error too: Chrome's console shows those of a worker, its Log domain not.
// A line is {level, text, url}; "ready" is printed once the browser's targets are attached.
const REPORTED_LEVELS = new Set(["error", "warning", "assert"]);
const FAILED_STATUS = 400;

const socket = new WebSocket(process.argv[2]);
let nextId = 1;
const send = (method, params = {}, sessionId) => socket.send(JSON.stringify({ id: nextId++, method, params, sessionId }));
const report = (level, text, url = "") => REPORTED_LEVELS.has(level) && console.log(JSON.stringify({ level, text, url }));

// a page or worker's console, exceptions, browser log and requests; its own workers attach too
function watch(sessionId) {
	for (const domain of ["Runtime", "Log", "Network"]) send(`${domain}.enable`, {}, sessionId);
	send("Target.setAutoAttach", { autoAttach: true, waitForDebuggerOnStart: false, flatten: true }, sessionId);
}

const watched = new Set(); // target ids: a worker attaches through the browser and through its page
const handlers = {
	"Target.attachedToTarget": ({ sessionId, targetInfo }) => watched.has(targetInfo.targetId) || (watched.add(targetInfo.targetId), watch(sessionId)),
	"Runtime.consoleAPICalled": ({ type, args, stackTrace }) =>
		report(type === "warn" ? "warning" : type, args.map(arg => arg.value ?? arg.description ?? arg.type).join(" "), stackTrace?.callFrames[0]?.url),
	"Runtime.exceptionThrown": ({ exceptionDetails: details }) =>
		report("error", details.exception?.description ?? details.text, details.url),
	// a failed request comes from Network.responseReceived, for pages and workers alike
	"Log.entryAdded": ({ entry }) => entry.source === "network" || report(entry.level, entry.text, entry.url),
	"Network.responseReceived": ({ response }) =>
		response.status >= FAILED_STATUS && report("error", `${response.status} ${response.statusText}`.trim(), response.url),
};

socket.onopen = () => {
	send("Target.setDiscoverTargets", { discover: true });
	send("Target.setAutoAttach", { autoAttach: true, waitForDebuggerOnStart: false, flatten: true });
	setTimeout(() => console.log("ready"), 500);
};
socket.onmessage = message => {
	const { method, params } = JSON.parse(message.data);
	handlers[method]?.(params);
};
socket.onclose = () => process.exit(0);
