// The language guide beside the editor: guide.md, one chapter per `## title`, chapters from easy to advanced.
// The open chapter (the page's #anchor) shows its text; `try ▶` runs a snippet, an `Examples:` line links the tour
// examples and samples/ that show the chapter, and choosing such an example links back to its chapter.
// The expert guide (guide-expert.md) has the same chapters in compact form; a toggle switches, keeping the open
// chapter, and the page remembers the choice (?guide=expert names it in a link).
// Runs after playground.js (both deferred) and uses its $, element and window.playground.
const GUIDE_FILES = { beginner: "guide.md", expert: "guide-expert.md" };
const LEVEL_PARAMETER = "guide";
const LEVEL_KEY = "warp-guide-level";
const NARROW_SCREEN = "(max-width: 900px)"; // playground.css stacks the panes there: the guide starts closed
const EXAMPLES_PREFIX = "Examples: ";
const SAMPLES_SEPARATOR = "; samples: ";
const SNIPPET_FENCE = /^```warp(?: => (.*))?$/;
const PRINTED_FENCE = "```printed"; // right after a snippet: the lines it prints, shown above its value

const chapterOfExample = new Map();
const examplesOfChapter = new Map(); // in the guide's order: the example menu's groups
const MORE_SAMPLES = "more samples";

const escapeHtml = text => text.replace(/[&<>"]/g, char => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" })[char]);
const chapterId = title => title.toLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/^-|-$/g, "");

// `code`, **bold** and [text](url) in one line of escaped text
function inline(text) {
	return escapeHtml(text)
		.replace(/`([^`]+)`/g, "<code>$1</code>")
		.replace(/\*\*([^*]+)\*\*/g, "<strong>$1</strong>")
		.replace(/\[([^\]]+)\]\(([^)\s]+)\)/g, '<a href="$2">$1</a>');
}

// `welcome, "element events"` → ["welcome", "element events"]
const exampleNames = list => list.split(", ").map(name => name.trim().replace(/^"|"$/g, "")).filter(Boolean);

function exampleLinks(line, chapter) {
	const [tour, samples = ""] = line.slice(EXAMPLES_PREFIX.length).split(SAMPLES_SEPARATOR);
	const link = name => {
		if (!chapterOfExample.has(name)) {
			chapterOfExample.set(name, chapter);
			examplesOfChapter.set(chapter, [...examplesOfChapter.get(chapter) ?? [], name]);
		}
		return element("a", { href: `?example=${encodeURIComponent(name)}`, className: "guide-example", onclick: click => {
			click.preventDefault();
			window.playground.chooseExample(name);
			showChapterLink(chapterOfExample.get(name));
		} }, name);
	};
	const links = names => names.flatMap((name, index) => index ? [", ", link(name)] : [link(name)]);
	const parts = [element("strong", {}, "Examples: "), ...links(exampleNames(tour))];
	if (samples) parts.push(" · samples/: ", ...links(exampleNames(samples)));
	return element("p", { className: "guide-examples" }, ...parts);
}

function snippet(code, value, printed) {
	const run = element("button", { className: "guide-try", title: "load into the editor and run", onclick: () => window.playground.runCode(code) }, "try ▶");
	const shownPrinted = printed ? [element("pre", { className: "guide-printed" }, printed)] : [];
	const shown = value ? [element("div", { className: "guide-value" }, element("span", { className: "prompt" }, "» "), value)] : [];
	return element("div", { className: "guide-snippet" }, run, element("pre", {}, element("code", {}, code)), ...shownPrinted, ...shown);
}

// the lines of the fence that starts after lines[index]; index ends on its closing line
function fenceLines(lines, index) {
	const body = [];
	while (++index < lines.length && lines[index] !== "```") body.push(lines[index]);
	return [body.join("\n"), index];
}

function paragraph(lines) {
	const block = element("p");
	block.innerHTML = inline(lines.join(" "));
	return block;
}

// the Markdown subset guide.md uses: paragraphs, fences, `Examples:` lines; returns the chapter's elements
function renderBody(lines, chapter) {
	const blocks = [];
	let text = [];
	const flush = () => { if (text.length) blocks.push(paragraph(text)); text = []; };
	for (let index = 0; index < lines.length; index++) {
		const line = lines[index], fence = line.match(SNIPPET_FENCE);
		if (fence) {
			flush();
			let code, printed;
			[code, index] = fenceLines(lines, index);
			if (lines[index + 1] === PRINTED_FENCE) [printed, index] = fenceLines(lines, index + 1);
			blocks.push(snippet(code, fence[1], printed));
		} else if (line.startsWith(EXAMPLES_PREFIX)) {
			flush();
			blocks.push(exampleLinks(line, chapter));
		} else if (line.trim()) text.push(line);
		else flush();
	}
	flush();
	return blocks;
}

// guide.md → the intro and one <details> per chapter, with its anchor
function renderGuide(markdown) {
	const [head, ...sections] = markdown.split(/^## /m);
	const introLines = head.split("\n").filter(line => !line.startsWith("# "));
	const chapters = sections.map(section => {
		const [title, ...lines] = section.split("\n");
		const id = chapterId(title);
		const summary = element("summary", {}, element("a", { href: `#${id}` }, title));
		return element("details", { id, className: "guide-chapter", ontoggle: openedChapter }, summary, ...renderBody(lines, title));
	});
	return [...renderBody(introLines, ""), ...chapters];
}

const renderedGuides = {}; // level → its rendered intro and chapters

function chosenLevel() {
	const named = new URLSearchParams(location.search).get(LEVEL_PARAMETER);
	let remembered = null;
	try { remembered = localStorage.getItem(LEVEL_KEY); } catch { /* private window: the default */ }
	return [named, remembered].find(level => level in renderedGuides) ?? "beginner";
}

// shows the guide of that level, with the chapter open in the other one
function showLevel(level) {
	const open = document.querySelector(".guide-chapter[open]")?.id;
	$("guide-chapters").replaceChildren(...renderedGuides[level]);
	for (const button of document.querySelectorAll(".guide-level button")) button.setAttribute("aria-pressed", button.value === level);
	try { localStorage.setItem(LEVEL_KEY, level); } catch { /* private window: lasts for this page */ }
	if (open) document.getElementById(open).open = true;
}

function openedChapter(event) {
	const chapter = event.target;
	if (!chapter.open) return;
	for (const other of document.querySelectorAll(".guide-chapter[open]")) if (other !== chapter) other.open = false;
	if (location.hash !== `#${chapter.id}`) history.replaceState(null, "", `#${chapter.id}`);
	showChapterLink(chapter.querySelector("summary").textContent);
}

// the chapter the address names, else the first one
function openChapterOfAddress() {
	const chapter = location.hash ? document.getElementById(location.hash.slice(1)) : document.querySelector(".guide-chapter");
	if (!chapter?.classList.contains("guide-chapter")) return;
	if (location.hash) $("guide").open = true;
	chapter.open = true;
	if (location.hash) chapter.scrollIntoView({ block: "nearest" });
}

// the example menu in the guide's order: a group per chapter with its examples and samples, then the other samples
function groupExamplesByChapter() {
	const option = name => element("option", { value: name }, name);
	const group = (label, names) => element("optgroup", { label }, ...names.map(option));
	const chosen = $("examples").value;
	const others = [...Object.keys(EXAMPLES), ...Object.keys(SAMPLES).sort()].filter(name => !chapterOfExample.has(name));
	const chapters = [...examplesOfChapter].map(([chapter, names]) => group(chapter, names.filter(name => exampleSource(name) !== undefined)));
	// a sample no chapter links yet: guide.md should give it a chapter, Advanced at the latest
	$("examples").replaceChildren(...chapters, ...(others.length ? [group(MORE_SAMPLES, others)] : []));
	$("examples").value = chosen;
}

// the toolbar's link to the chapter last opened or listing the chosen example
function showChapterLink(chapter) {
	$("guide-link").hidden = !chapter;
	if (!chapter) return;
	$("guide-link").href = `#${chapterId(chapter)}`;
	$("guide-link").textContent = `guide: ${chapter}`;
}

async function startGuide() {
	if (matchMedia(NARROW_SCREEN).matches) $("guide").open = false;
	try {
		// the beginner guide first: its Examples lines group the example menu
		for (const [level, file] of Object.entries(GUIDE_FILES)) renderedGuides[level] = renderGuide(await (await fetch(file)).text());
	} catch (error) {
		$("guide-chapters").textContent = `the guide did not load: ${error.message}`;
		return;
	}
	for (const button of document.querySelectorAll(".guide-level button")) button.onclick = () => showLevel(button.value);
	showLevel(chosenLevel());
	groupExamplesByChapter();
	addEventListener("hashchange", openChapterOfAddress);
	$("examples").addEventListener("change", event => showChapterLink(chapterOfExample.get(event.target.value)));
	openChapterOfAddress();
	showChapterLink(chapterOfExample.get($("examples").value));
}

startGuide();
