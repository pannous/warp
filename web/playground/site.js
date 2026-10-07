// The loader of a page `warp build --site` made (src/site.rs, card web-ssr): index.html already shows the program's
// markup, rendered when the site was built. This runs app.wasm with the playground's host (host.js) and hydrates the
// page: the DOM stays as it is, an event in an element runs that element's handler (markup.js elementEvent), and what the
// handler changed is shown by morphing in the HTML the program renders itself (its export page·html, notes/web_framework.md
// "Built sites"). Main runs once here as it ran at build time, so the component instances count alike.

const SITE_ROOT = "wasp-root";
const SITE_MODULE = "app.wasm";
const PAGE_HTML = "page·html";
const SITE_EVENTS = { click: () => ({}), input: happened => inputDetail(happened.composedPath()[0]) };

let site; // the program's run (host.js runProgram's holder)

const siteHooks = {
	print: (text, stream) => (stream === 2 ? console.error : console.log)(text.replace(/\n$/, "")),
	listen: holder => { site = holder; },
	arrived: (holder, handler) => holder === site && showAfter(runTimer(holder, siteHooks, handler)),
};

// what a handler left, shown: the page's markup anew, or the failure on the console
function showAfter(outcome) {
	if (outcome.result === undefined) return console.error("wasp:", outcome.failure ?? outcome.trap ?? outcome.error);
	const render = site.exports[PAGE_HTML];
	if (!render) return console.warn(`wasp: the page changes only with the export ${PAGE_HTML} (card web-ssr)`);
	const template = document.createElement("template");
	template.innerHTML = readText(site.exports, ...render());
	morphChildren(document.getElementById(SITE_ROOT), template.content);
}

async function hydrate() {
	const bytes = new Uint8Array(await (await fetch(SITE_MODULE)).arrayBuffer());
	const outcome = runProgram(bytes, siteHooks);
	if (outcome.result === undefined) return console.error("wasp:", outcome.failure ?? outcome.trap ?? outcome.error);
	const root = document.getElementById(SITE_ROOT);
	for (const [event, detail] of Object.entries(SITE_EVENTS)) {
		root.addEventListener(event, happened => {
			const found = site && elementEvent(event, happened, detail(happened));
			if (found && site.exports[`on·${found.event}·node`]) showAfter(runPageEvent(site, siteHooks, found.event, found.detail));
		});
	}
}

hydrate();
