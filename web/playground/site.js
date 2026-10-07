// The loader of a page `warp build --site` made (src/site.rs, card web-ssr): index.html already shows the program's
// markup, rendered when the site was built. This runs app.wasm with the playground's host (host.js) and hydrates the
// page: the DOM stays as it is, an event in an element runs that element's handler (markup.js elementEvent), and what the
// handler changed is shown by morphing in the HTML the program renders itself (its export page·html, std/markup.wasp's
// to_html, which also rendered index.html at build time; notes/web_framework.md "Built sites"). Main runs once here as it ran at build time, so the component instances count alike.

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
	if (!render) return console.error(`wasp: app.wasm exports no ${PAGE_HTML}: build it with warp build --site`);
	const template = document.createElement("template");
	template.innerHTML = plainOfTree(readNode(site.exports, render()));
	morphChildren(document.getElementById(SITE_ROOT), template.content);
}

async function hydrate() {
	const bytes = new Uint8Array(await (await fetch(SITE_MODULE)).arrayBuffer());
	// stored values and those a `warp dev` page keeps across reloads (host.js STD_ADAPTERS.store, markup.js)
	Object.assign(storedValues, keptValues());
	self.keepStored = keepValue;
	const outcome = runProgram(bytes, siteHooks);
	if (outcome.result === undefined) return console.error("wasp:", outcome.failure ?? outcome.trap ?? outcome.error);
	// kept values may differ from those the page was built with: show what main left (site is set once it listens)
	if (site) showAfter(outcome);
	const root = document.getElementById(SITE_ROOT);
	for (const [event, detail] of Object.entries(SITE_EVENTS)) {
		root.addEventListener(event, happened => {
			const found = site && elementEvent(event, happened, detail(happened));
			if (found && site.exports[`on·${found.event}·node`]) showAfter(runPageEvent(site, siteHooks, found.event, found.detail));
		});
	}
}

hydrate();
