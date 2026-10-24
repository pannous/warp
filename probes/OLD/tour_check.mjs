// Runs every tour example (web/playground/examples.js) with the warp CLI and lists those whose value differs from the
// one the tour shows (card consistency; the page itself is checked in CI by test_in_browser.py --examples).
// Usage: node probes/tour_check.mjs <warp binary> [name…]   (from the repository root)
// The CLI writes a top-level text bare where the page quotes it, and has no page: examples that render, click, listen
// or load a playground component are listed as "page only"; a shown value counts when any output line is it.
import { existsSync, readdirSync, readFileSync, writeFileSync, mkdirSync } from "node:fs";
import { execFileSync } from "node:child_process";
import vm from "node:vm";

const SNIPPET_DIRECTORY = "data/tour_check";
const SECONDS_PER_EXAMPLE = 30;
const PAGE_ONLY_FIELDS = ["clicks", "typed", "wait", "kept", "keyed", "animated", "address"];
const PAGE_ONLY_CODE = /\b(div|button|ul|svg|canvas|page|every|use component)\b|rust_demo/;

// samples/<chapter>/<name>.warp
const sampleFile = name => readdirSync("samples").map(chapter => `samples/${chapter}/${name}.warp`).find(existsSync);
const [warp, ...only] = process.argv.slice(2);
const context = {
	SAMPLES: new Proxy({}, { get: (_, name) => readFileSync(sampleFile(String(name)), "utf8") }),
};
vm.runInNewContext(`${readFileSync("web/playground/examples.js", "utf8")}; this.EXAMPLES = EXAMPLES`, context);
mkdirSync(SNIPPET_DIRECTORY, { recursive: true });

function shown(code) {
	const path = `${SNIPPET_DIRECTORY}/snippet.warp`;
	writeFileSync(path, code);
	try {
		return execFileSync(warp, [path], {
			env: { ...process.env, WARP_NO_WINDOW: "1" },
			timeout: SECONDS_PER_EXAMPLE * 1000,
			encoding: "utf8",
			stdio: ["ignore", "pipe", "pipe"],
		}).trim();
	} catch (failure) {
		return `${failure.stdout ?? ""}${failure.stderr ?? ""}`.trim() || String(failure.message);
	}
}

const unquoted = (value) => value.replace(/^"(.*)"$/s, "$1");

for (const [name, example] of Object.entries(context.EXAMPLES)) {
	if (only.length && !only.includes(name)) continue;
	if (example.value === undefined) continue;
	if (PAGE_ONLY_FIELDS.some((field) => field in example) || PAGE_ONLY_CODE.test(example.code)) {
		console.log(`page only  ${name}`);
		continue;
	}
	const output = shown(example.code);
	const lines = output.split("\n").map((line) => line.trim());
	if (!lines.includes(example.value) && !lines.includes(unquoted(example.value))) {
		console.log(`DIFFERS    ${name}\n  tour: ${example.value}\n  cli:  ${output.split("\n").slice(-3).join("\n        ")}`);
	}
}
