// Lists the playground examples and samples/ no `Examples: …; samples: …` line of guide.md names:
// the ones the example menu would put in "more samples" (web/playground/guide.js). Exit 1 if any.
import { readFileSync, readdirSync } from "node:fs";
const ROOT = new URL("..", import.meta.url).pathname;
const read = path => readFileSync(ROOT + path, "utf8");
const names = list => list.split(", ").map(name => name.trim().replace(/^"|"$/g, "")).filter(Boolean);
const named = new Set(read("web/playground/guide.md").split("\n").filter(line => line.startsWith("Examples: "))
	.flatMap(line => line.slice("Examples: ".length).split("; samples: ").flatMap(names)));
const excluded = new Set(read("web/playground/excluded_samples.txt").split("\n").filter(line => line && !line.startsWith("#")).map(line => line.split("\t")[0]));
// samples/<chapter>/<name>.warp; golf/ and wit/ hold no menu samples
const chapters = readdirSync(ROOT + "samples", { withFileTypes: true }).filter(entry => entry.isDirectory() && !["golf", "wit"].includes(entry.name));
const samples = chapters.flatMap(chapter => readdirSync(`${ROOT}samples/${chapter.name}`)).filter(file => file.endsWith(".warp")).map(file => file.slice(0, -5)).filter(name => !excluded.has(name));
const tour = [...read("web/playground/examples.js").matchAll(/^\t(?:"([^"]+)"|(\w+)):/gm)].map(match => match[1] ?? match[2]);
const unsorted = [...new Set([...tour, ...samples])].filter(name => !named.has(name)).sort();
console.log(unsorted.length ? `unsorted: ${unsorted.join(", ")}` : "every example and sample has a chapter");
process.exit(unsorted.length ? 1 : 0);
