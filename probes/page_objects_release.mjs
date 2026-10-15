// The page frees the objects its Workers no longer hold (card page-pageobjects): `node --expose-gc probes/page_objects_release.mjs`.
// The Worker's side (host-foreign.js PageObject … fromPage) and the page's (task-workers.js pageObjects … forWorker) are
// taken from the files; a Worker's message reaches the page's releaseFrom directly.
import { readFileSync } from "node:fs";
import assert from "node:assert/strict";

const ROUNDS = 1000;
const document_ = { page: "document" }; // an object only the page has
const between = (file, from, to) => {
	const source = readFileSync(new URL(`../web/playground/${file}`, import.meta.url), "utf8");
	return source.slice(source.indexOf(from), source.indexOf(to, source.indexOf(from)));
};
const pageSide = new Function(`${between("task-workers.js", "const pageObjects = new Map()", "// `$who` is")}
	return { pageObjects, forWorker, onPage, releaseFrom, forgetHolder };`)();

// a Worker: its own copy of host-foreign.js's side, its messages to the page answered at once
function workerSide(holder) {
	const self = { postMessage: ({ releasePageObject }) => pageSide.releaseFrom(holder, releasePageObject) };
	return new Function("self", `${between("host-foreign.js", "class PageObject", "const toPage")} return fromPage;`)(self);
}
// the page's answer as the Worker reads it (JSON through the shared buffer)
const asked = (fromPage, holder, value) => fromPage(JSON.parse(JSON.stringify(pageSide.forWorker(value, holder))));

async function collected() {
	for (let round = 0; round < 5; round++) {
		globalThis.gc();
		await new Promise(done => setTimeout(done, 10));
	}
}

const first = {}, second = {};
const [workerA, workerB] = [first, second].map(workerSide);

// 1. the same object asked again and again: one PageObject, one entry, gone once dropped
let kept = Array.from({ length: 3 }, () => asked(workerA, first, document_));
assert.equal(new Set(kept).size, 1, "one PageObject per id");
assert.equal(pageSide.pageObjects.size, 1);
kept = undefined;
await collected();
assert.equal(pageSide.pageObjects.size, 0, "dropped in the Worker: freed on the page");

// 2. distinct objects (a fresh NodeList each time), none kept: all freed
for (let round = 0; round < ROUNDS; round++) asked(workerA, first, { round });
await collected();
assert.equal(pageSide.pageObjects.size, 0, `${ROUNDS} distinct objects: freed`);

// 3. two Workers hold one object: it stays until both let go (one collected, one terminated)
let held = asked(workerA, first, document_);
const heldByB = asked(workerB, second, document_);
const id = held.$page;
held = undefined;
await collected();
assert.equal(pageSide.onPage({ $page: id }), document_, "the other Worker still holds it");
assert.equal(heldByB.$page, id);
pageSide.forgetHolder(second);
assert.equal(pageSide.pageObjects.size, 0, "a terminated holder lets go");
assert.throws(() => pageSide.onPage({ $page: id, type: "Object" }), /no longer keeps/);
console.log("page objects: freed when no Worker holds them");
