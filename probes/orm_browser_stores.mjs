// card orm-browser-stores: the playground's table adapter (web/playground/host-files.js) on a big table, in node.
// Each row is its own IndexedDB key, so a change writes one row; this probe checks that inserting, paging and counting
// stay cheap per call as the table grows (no whole-table copy or spread per call).
// Run: node probes/orm_browser_stores.mjs [rows]
import { readFileSync } from "node:fs";

const ROWS = Number(process.argv[2] ?? 200000);
const PAGE_SIZE = 100;
const FILE = "people.database.json";
const SCHEMA = [["name", "text", null], ["age", "int", 0]];

const source = readFileSync(new URL("../web/playground/host-files.js", import.meta.url), "utf8");
let part;
const self = { location: { href: "http://localhost/web/playground/" } };
new Function("self", "addHostPart", source)(self, given => { part = given; });
const table = part.adapters.table;
const warnings = { warn: message => console.log("warning:", message) };

const timed = (label, run) => {
	const start = performance.now();
	const result = run();
	console.log(`${label}: ${(performance.now() - start).toFixed(1)} ms`);
	return result;
};

table.migrate.call(warnings, "people", SCHEMA, FILE);
timed(`insert ${ROWS} rows`, () => {
	for (let i = 1; i <= ROWS; i++) table.insert("people", ["name", "age"], [`p${i}`, i % 90], FILE);
});
console.log("count:", table.count("people", SCHEMA, FILE));
const last = timed("one page at the end", () => table.page("people", SCHEMA, FILE, ROWS - 1, PAGE_SIZE));
console.log("last page:", JSON.stringify(last));
timed(`walk ${ROWS / PAGE_SIZE} pages`, () => {
	for (let start = 1; start <= ROWS; start += PAGE_SIZE) table.page("people", SCHEMA, FILE, start, PAGE_SIZE);
});
timed(`update ${ROWS} rows`, () => {
	for (let id = 1; id <= ROWS; id++) table.update("people", id, "age", id % 7, FILE);
});
table.delete("people", 2, FILE);
const afterDelete = table.page("people", SCHEMA, FILE, 1, 3).map(([id, , age]) => [id, age]);
const added = table.insert("people", ["name", "age"], ["new", 1], FILE);
console.log("after deleting row 2:", JSON.stringify(afterDelete), "next id:", added);
const ok = last.length === 2 && last[1][0] === ROWS && last[1][1] === `p${ROWS}`
	&& JSON.stringify(afterDelete) === "[[1,1],[3,3],[4,4]]" && added === ROWS + 1;
console.log(ok ? "ok" : "FAILED");
process.exit(ok ? 0 : 1);
