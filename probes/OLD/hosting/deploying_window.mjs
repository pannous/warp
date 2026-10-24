// Runs the deploy window's script (hosting.mjs deployingPage) without a browser: `node deploying_window.mjs <hosting>
// <script.js>` prints what the window shows and where it goes, exit 0 once it opens the program within 20 s.
import { readFileSync } from "node:fs";
const [hosting, script] = process.argv.slice(2);
const fetchOfBrowser = fetch;
globalThis.fetch = address => fetchOfBrowser(new URL(address, hosting));
globalThis.document = { getElementById: () => ({ set innerHTML(html) { console.log("shown:", html); } }) };
globalThis.location = { replace: url => { console.log("opened:", url); process.exit(0); } };
setTimeout(() => { console.log("never opened the program"); process.exit(1); }, 20000);
eval(readFileSync(script, "utf8"));
