// card code-completion: the playground's gray continuation asks the Messages API as assistant.js does and gets text,
// and a failed request is an error with a reason, never a silent nothing. Real requests, no browser:
//   ANTHROPIC_API_KEY=… node probes/web/completion_request.mjs
// assistant.js runs as a classic script in one global, with the page's localStorage holding the key
import { readFileSync } from "node:fs";
import vm from "node:vm";

const PLAYGROUND = new URL("../../web/playground/", import.meta.url);
const KEY = process.env.ANTHROPIC_API_KEY ?? process.env.ANTHROPIC_API_KEY_VISION;
const PROGRAMS = ["fib(n) := if n < 2 then n else ‸", "def quicksort(xs) := if count(xs) < 2 then xs else ‸", "people = [{name: \"Ann\", age: 31}]\nfor person in people ‸"];
if (!KEY) { console.log("FAIL no ANTHROPIC_API_KEY"); process.exit(1); }

const storage = new Map();
// the page's own files (guide.md, …) as the site serves them, the API itself over the network
const pageFetch = (url, options) => /^\w+:/.test(url) ? fetch(url, options) : Promise.resolve(new Response(readFileSync(new URL(url, PLAYGROUND))));
const page = vm.createContext({ fetch: pageFetch, Response, console, localStorage: { getItem: key => storage.get(key) ?? null } });
vm.runInContext("function stored(key) { return localStorage.getItem(key); }", page);
vm.runInContext(readFileSync(new URL("assistant.js", PLAYGROUND), "utf8"), page, { filename: "assistant.js" });
const ask = (key, content) => {
	storage.set(vm.runInContext("API_KEY_STORAGE", page), key);
	page.content = content;
	return vm.runInContext("askClaude({ model: COMPLETION_MODEL, system: 'Complete the warp program at ‸. Answer with only the text to insert there.', messages: [{ role: 'user', content }], maxTokens: COMPLETION_TOKENS, thinking: NO_THINKING })", page);
};

let failures = 0;
const expect = (what, holds, shown) => {
	console.log(`${holds ? "ok  " : "FAIL"} ${what}: ${JSON.stringify(shown)}`);
	failures += !holds;
};
for (const program of PROGRAMS) {
	const text = await ask(KEY, program).catch(error => `error: ${error.message}`);
	expect(`a continuation of ${JSON.stringify(program)}`, text.trim() !== "" && !text.startsWith("error:"), text.slice(0, 60));
}
// card completion-terse: with the real prompt (the current guide) the shown continuation is code, one line here
const TERSE_PROGRAMS = ["[1 2 3] where it > 1‸", "xs = [3 1 2]\nsorted = ‸"];
for (const program of TERSE_PROGRAMS) {
	storage.set(vm.runInContext("API_KEY_STORAGE", page), KEY);
	page.content = program;
	const shown = await vm.runInContext("systemPrompt(COMPLETION_TASK).then(system => askClaude({ model: COMPLETION_MODEL, system, messages: [{ role: 'user', content }], maxTokens: COMPLETION_TOKENS, thinking: NO_THINKING })).then(answer => [answer, terseContinuation(answer)])", page).catch(error => ["", `error: ${error.message}`]);
	expect(`terse continuation of ${JSON.stringify(program)} (answer ${JSON.stringify(shown[0])})`, !shown[1].startsWith("error:") && !shown[1].includes("\n") && !shown[1].includes("//") && !vm.runInContext(`isProse(${JSON.stringify(shown[1])})`, page), shown[1]);
}
const refused = await ask("not-a-key", PROGRAMS[0]).then(text => `no error, text ${JSON.stringify(text)}`, error => error.message);
expect("a refused key is an error with its reason", /invalid|authentication|x-api-key/i.test(refused), refused);
process.exit(failures ? 1 : 0);
