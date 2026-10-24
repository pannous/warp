// Runs playground samples or snippets in headless WebKit (Safari's engine): node probes/webkit_samples.mjs <url> <name>…
// needs playwright: scratch/webkit (npm install playwright; npx playwright install webkit)
import { createRequire } from "module";
const require = createRequire(new URL("../scratch/webkit/package.json", import.meta.url));
const { webkit } = require("playwright");
const [url = "https://warp.pannous.com/", ...names] = process.argv.slice(2);
const isolated = process.env.ISOLATED !== "no"; // ISOLATED=no blocks the coi service worker: a page without shared memory
const browser = await (process.env.ENGINE === "chromium" ? require("playwright").chromium : webkit).launch();
const page = await (await browser.newContext({ serviceWorkers: isolated ? "allow" : "block" })).newPage();
page.on("pageerror", error => console.log("pageerror:", error.message));
await page.goto(url);
if (isolated) await page.waitForFunction(() => crossOriginIsolated, null, { timeout: 15000 }).catch(() => console.log("never cross-origin isolated"));
await page.waitForFunction(() => window.playground && document.querySelector("#examples option"), null, { timeout: 30000 });
console.log("crossOriginIsolated:", await page.evaluate(() => crossOriginIsolated));
for (const name of names) {
	const shown = await page.evaluate(async ([name, wait]) => {
		window.playground.chooseExample(name);
		await new Promise(done => setTimeout(done, wait));
		return document.querySelector("#output").innerText.slice(0, 400);
	}, [name, Number(process.env.WAIT ?? 5000)]);
	console.log(`== ${name}\n${shown}`);
}
await browser.close();
