// Mouse drag over samples/finger_paint on the deployed playground, in headless Chromium and Firefox (Playwright), with
// web/playground/canvas.js as deployed or as given (card register-fingers: desktop finger paint said broken after it).
// Runs on pannous.com (decision browser-server): `node finger_paint_mouse_repro.mjs [canvas.js to serve instead]`.
// The page is made cross-origin isolated by the headers added here, service workers blocked (coi-serviceworker.js
// would reload it outside Playwright's routing).
import { chromium, firefox } from "playwright";
import { readFileSync } from "node:fs";

const PAGE = "https://warp.pannous.com/";
const SAMPLE = "finger_paint";
const PAPER_FROM = 240;
const STEPS = 25;
const replacementCanvas = process.argv[2] && readFileSync(process.argv[2], "utf8");

async function check(browserType) {
	const browser = await browserType.launch();
	const context = await browser.newContext({ serviceWorkers: "block", viewport: { width: 1280, height: 900 } });
	const page = await context.newPage();
	const messages = [];
	page.on("console", message => ["error", "warning"].includes(message.type()) && messages.push(`${message.type()}: ${message.text()}`));
	page.on("pageerror", error => messages.push(`pageerror: ${error.message}`));
	await page.route("**/*", async route => {
		const isCanvas = replacementCanvas && new URL(route.request().url()).pathname.endsWith("/canvas.js");
		const response = await route.fetch();
		const headers = { ...response.headers(), "cross-origin-opener-policy": "same-origin", "cross-origin-embedder-policy": "require-corp", "cross-origin-resource-policy": "cross-origin" };
		await route.fulfill(isCanvas ? { response, headers, body: replacementCanvas } : { response, headers });
	});
	await page.goto(`${PAGE}#${SAMPLE}`);
	await page.waitForFunction(() => self.crossOriginIsolated && window.playground, null, { timeout: 60000 });
	await page.click("#output");
	await page.evaluate(name => void playground.chooseExample(name), SAMPLE);
	const canvas = page.locator("#paintings canvas").last();
	await canvas.waitFor({ timeout: 60000 });
	await page.waitForTimeout(1500);
	const box = await canvas.boundingBox();
	const at = share => [box.x + box.width * share, box.y + box.height * share];
	await page.mouse.move(...at(0.2));
	await page.mouse.down();
	for (let step = 1; step <= STEPS; step++) await page.mouse.move(...at(0.2 + 0.6 * step / STEPS), { steps: 2 });
	await page.mouse.up();
	await page.waitForTimeout(1500);
	const shown = await page.evaluate(paper => {
		const canvas = [...document.querySelectorAll("#paintings canvas")].at(-1);
		const copy = new OffscreenCanvas(canvas.width, canvas.height).getContext("2d");
		copy.drawImage(canvas, 0, 0);
		const middle = [...copy.getImageData(canvas.width >> 1, canvas.height >> 1, 1, 1).data.slice(0, 3)];
		const style = getComputedStyle(canvas);
		return { painted: Math.min(...middle) < paper, middle, canvases: document.querySelectorAll("#paintings canvas").length,
			value: document.getElementById("value").textContent, status: document.getElementById("status").textContent,
			touchAction: style.touchAction, size: `${canvas.width}×${canvas.height} shown ${Math.round(canvas.getBoundingClientRect().width)}` };
	}, PAPER_FROM);
	console.log(browserType.name(), replacementCanvas ? "canvas.js given" : "canvas.js deployed", JSON.stringify(shown), messages.slice(0, 8));
	await browser.close();
}

for (const browserType of [chromium, firefox]) await check(browserType).catch(error => console.log(browserType.name(), "failed:", error.message));
