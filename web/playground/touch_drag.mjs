// A finger dragged across the playground's last painting, as a touch screen sends it (Input.dispatchTouchEvent over the
// DevTools protocol): `node touch_drag.mjs <cdp url> <from> <to>` drags from `from` to `to` of the canvas's width and
// height, a diagonal, so neither a sideways nor an upright pan fits it; prints "dragged x1,y1 → x2,y2" in CSS pixels,
// or why not. test_in_browser.py --examples checks that samples/finger_paint painted along it (card
// register-fingers: the iPad's Safari took the touch over for scrolling, so no finger reached the pointer)
const STEPS = 20;
const STEP_MS = 30;

const [cdpUrl, FROM, TO] = [process.argv[2], Number(process.argv[3]), Number(process.argv[4])];
const socket = new WebSocket(cdpUrl);
let nextId = 1;
const answers = new Map();
const send = (method, params = {}, sessionId) => new Promise(answer => {
	const id = nextId++;
	answers.set(id, answer);
	socket.send(JSON.stringify({ id, method, params, sessionId }));
});
const pause = milliseconds => new Promise(done => setTimeout(done, milliseconds));
const finish = text => { console.log(text); process.exit(0); };

const LAST_CANVAS_BOUNDS = `JSON.stringify((() => {
	const bounds = [...document.querySelectorAll("#paintings canvas")].at(-1)?.getBoundingClientRect();
	return bounds && { left: bounds.left, top: bounds.top, width: bounds.width, height: bounds.height };
})())`;

async function drag() {
	const { targetInfos } = await send("Target.getTargets");
	const page = targetInfos.find(target => target.type === "page");
	if (!page) finish("no page to touch");
	const { sessionId } = await send("Target.attachToTarget", { targetId: page.targetId, flatten: true });
	await send("Emulation.setTouchEmulationEnabled", { enabled: true, maxTouchPoints: 5 }, sessionId);
	const { result } = await send("Runtime.evaluate", { expression: LAST_CANVAS_BOUNDS, returnByValue: true }, sessionId);
	const bounds = result.value && JSON.parse(result.value);
	if (!bounds) finish("no canvas to touch");
	const at = share => ({ x: bounds.left + bounds.width * share, y: bounds.top + bounds.height * share, id: 0 });
	const touch = (type, touchPoints) => send("Input.dispatchTouchEvent", { type, touchPoints }, sessionId);
	await touch("touchStart", [at(FROM)]);
	for (let step = 1; step <= STEPS; step++) {
		await pause(STEP_MS);
		await touch("touchMove", [at(FROM + (TO - FROM) * step / STEPS)]);
	}
	await pause(STEP_MS);
	await touch("touchEnd", []);
	const [start, end] = [at(FROM), at(TO)].map(({ x, y }) => `${Math.round(x)},${Math.round(y)}`);
	finish(`dragged ${start} → ${end}`);
}

socket.onopen = () => drag().catch(error => finish(`touch failed: ${error.message ?? error}`));
socket.onmessage = message => {
	const { id, result, error } = JSON.parse(message.data);
	answers.get(id)?.(error ? Promise.reject(new Error(error.message)) : result ?? {});
	answers.delete(id);
};
