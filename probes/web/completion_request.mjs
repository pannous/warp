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
const page = vm.createContext({ fetch, console, localStorage: { getItem: key => storage.get(key) ?? null } });
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
const refused = await ask("not-a-key", PROGRAMS[0]).then(text => `no error, text ${JSON.stringify(text)}`, error => error.message);
expect("a refused key is an error with its reason", /invalid|authentication|x-api-key/i.test(refused), refused);
process.exit(failures ? 1 : 0);
