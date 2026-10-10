// card code-completion: the editor's word completion counts the words of the tour (examples.js) and the samples
// (samples.js, built by web/playground/build.sh), whose scripts declare them as top-level consts, no window properties.
// The scripts run as classic scripts in one global, as in the page:  node probes/web/completion_corpus.mjs
import { readFileSync, existsSync } from "node:fs";
import vm from "node:vm";

const PLAYGROUND = new URL("../../web/playground/", import.meta.url);
const TOUR_WORD = "Hello"; // the first example's "Hello, world"

const page = vm.createContext({});
page.window = page;
const run = file => vm.runInContext(readFileSync(new URL(file, PLAYGROUND), "utf8"), page, { filename: file });
run("examples.js");
if (existsSync(new URL("samples.js", PLAYGROUND))) run("samples.js");
else vm.runInContext("window.SAMPLES = {}", page); // index.html's onerror of samples.js
run("completion.js");

const uses = vm.runInContext(`corpus().get(${JSON.stringify(TOUR_WORD)}) ?? 0`, page);
console.log(`${uses > 0 ? "ok  " : "FAIL"} the tour's words are in the corpus: ${TOUR_WORD} × ${uses}`);
process.exit(uses > 0 ? 0 : 1);
