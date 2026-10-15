// Mouse drag over samples/finger_paint on the deployed playground, in headless Chromium and Firefox (Playwright), with
// web/playground/canvas.js as deployed or as given (card register-fingers: desktop finger paint said broken after it).
// Runs on pannous.com (decision browser-server): `node finger_paint_mouse_repro.mjs [canvas.js to serve instead]`;
// FRESH=1 drags right after the page loaded the sample from its address, without a click first, as the user does;
// DPR=2 a Retina screen; DARK=1 dark mode; NOT_ISOLATED=1 a page without shared memory, as after a hard reload; REAL=1 leaves the page as a visitor gets it (its coi-serviceworker.js, no headers added, canvas.js as deployed).
// The page is made cross-origin isolated by the headers added here, service workers blocked (coi-serviceworker.js
// would reload it outside Playwright's routing).
import { chromium, firefox } from "playwright";
import { readFileSync } from "node:fs";

const PAGE = "https://warp.pannous.com/";
const SAMPLE = "finger_paint";
const PAPER_FROM = 240;
const STEPS = 25;
const replacementCanvas = process.argv[2] && readFileSync(process.argv[2], "utf8");
const fresh = Boolean(process.env.FRESH);
const real = Boolean(process.env.REAL);
const notIsolated = Boolean(process.env.NOT_ISOLATED);

async function check(browserType) {
	const browser = await browserType.launch({ headless: !process.env.HEADED }); // HEADED=1 under xvfb-run: a real window
	const context = await browser.newContext({ serviceWorkers: real ? "allow" : "block", viewport: { width: 1280, height: 900 }, deviceScaleFactor: Number(process.env.DPR ?? 1), colorScheme: process.env.DARK ? "dark" : "light" });
	// the first pointerdown tells the program window_open from the user activation (playground.js tellSystemValues)
	await context.addInitScript(() => addEventListener("pointerdown", () => self.activeAtFirstPointerdown ??= navigator.userActivation?.hasBeenActive, { capture: true }));
	const page = await context.newPage();
	const messages = [];
	page.on("console", message => ["error", "warning"].includes(message.type()) && messages.push(`${message.type()}: ${message.text()}`));
	page.on("pageerror", error => messages.push(`pageerror: ${error.message}`));
	if (!real) await page.route("**/*", async route => {
		const isCanvas = replacementCanvas && new URL(route.request().url()).pathname.endsWith("/canvas.js");
		const response = await route.fetch();
		const isolation = { "cross-origin-opener-policy": "same-origin", "cross-origin-embedder-policy": "require-corp", "cross-origin-resource-policy": "cross-origin" };
		const headers = { ...response.headers(), ...(notIsolated ? {} : isolation) };
		await route.fulfill(isCanvas ? { response, headers, body: replacementCanvas } : { response, headers });
	});
	await page.goto(`${PAGE}#${SAMPLE}`);
	await page.waitForFunction(isolated => (isolated || self.crossOriginIsolated) && window.playground, notIsolated, { timeout: 60000 });
	if (!fresh) {
		await page.click("#output");
		await page.evaluate(name => void playground.chooseExample(name), SAMPLE);
	}
	const canvas = page.locator("#paintings canvas").last();
	await canvas.waitFor({ timeout: 60000 });
	await page.waitForTimeout(Number(process.env.WAIT_MS ?? 1500)); // WAIT_MS: a visitor looks a while before painting
	const statusBefore = await page.locator("#status").textContent();
	const box = await canvas.boundingBox();
	const at = share => [box.x + box.width * share, box.y + box.height * share];
	await page.mouse.move(...at(0.2));
	await page.mouse.down();
	for (let step = 1; step <= STEPS; step++) await page.mouse.move(...at(0.2 + 0.6 * step / STEPS), { steps: 2 });
	await page.mouse.up();
	await page.screenshot({ path: `${browserType.name()}${fresh ? "_fresh" : ""}.png` });
	await page.waitForTimeout(1500);
	const shown = await page.evaluate(paper => {
		const canvas = [...document.querySelectorAll("#paintings canvas")].at(-1);
		const copy = new OffscreenCanvas(canvas.width, canvas.height).getContext("2d");
		copy.drawImage(canvas, 0, 0);
		const middle = [...copy.getImageData(canvas.width >> 1, canvas.height >> 1, 1, 1).data.slice(0, 3)];
		const style = getComputedStyle(canvas);
		return { painted: Math.min(...middle) < paper, middle, canvases: document.querySelectorAll("#paintings canvas").length,
			value: document.getElementById("value").textContent, status: document.getElementById("status").textContent,
			touchAction: style.touchAction, note: document.querySelector(".pointer-unreachable")?.textContent, size: `${canvas.width}×${canvas.height} shown ${Math.round(canvas.getBoundingClientRect().width)}` };
	}, PAPER_FROM);
	shown.statusBefore = statusBefore;
	shown.activeAtFirstPointerdown = await page.evaluate(() => self.activeAtFirstPointerdown);
	shown.isolated = await page.evaluate(() => self.crossOriginIsolated);
	console.log(browserType.name(), real ? "as served" : "headers added", fresh ? "fresh" : "clicked first", replacementCanvas ? "canvas.js given" : "canvas.js deployed", JSON.stringify(shown), messages.slice(0, 8));
	await browser.close();
}

for (const browserType of [chromium, firefox]) await check(browserType).catch(error => console.log(browserType.name(), "failed:", error.message));
