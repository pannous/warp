// The loader of a page `warp build --site` made (src/web/site.rs, card web-ssr): index.html already shows the program's
// markup, rendered when the site was built. This runs app.wasm with the playground's host (host.js) and hydrates the
// page: the DOM stays as it is, an event in an element runs that element's handler (markup.js elementEvent), timers run theirs (host.js startTimers), and what the
// handler changed is shown by patching the holes of its markup (host.js holePatches, card web-fine-rest), else by morphing
// in the HTML the program renders itself (its export page·html, lib/markup.warp's to_html, which also rendered index.html
// at build time; notes/web_framework.md "Built sites"). Main runs once here as it ran at build time, so the component instances count alike.
// A link to a page of the same site goes through the program's routes (History API): the address changes, the program
// reads the new path (page_path) and the page morphs; the back button does the same (lowering/web/routes.rs).
// Each route's own code may come in a module of its own (src/web/route_split.rs): the page loads the one of the route it
// shows before running it (host-routes.js loadRouteModule).

const SITE_ROOT = "warp-root";
const SITE_MODULE = "app.wasm";
const PAGE_HTML = "page·html";
const KEY_EVENT = "key";
const fieldDetail = happened => inputDetail(happened.composedPath()[0]);
const SITE_EVENTS = { click: () => ({}), input: fieldDetail, ...Object.fromEntries(Object.keys(FIELD_EVENT_TYPES).map(event => [event, fieldDetail])), ...self.FORM_EVENTS };

let site; // the program's run (host.js runProgram's holder)

const siteHooks = {
	pagePath: () => location.pathname,
	instantiated: holder => { site = holder; },
	print: (text, stream) => (stream === 2 ? console.error : console.log)(text.replace(/\n$/, "")),
	listen: holder => {
		// only a page whose program has timers loads host-timers.js
		if (holder.timers) startTimers(holder, handler => showAfter(runTimer(holder, siteHooks, handler)));
	},
	arrived: (holder, handler) => holder === site && showAfter(runTimer(holder, siteHooks, handler)),
};

// what a handler left, shown: the holes of the page's markup anew (after main all of it), or the failure on the console
// (which stops the timers)
function showAfter(outcome, shown = showHoles) {
	if (outcome.result === undefined) {
		site.stopTimers?.();
		return console.error("warp:", outcome.failure ?? outcome.trap ?? outcome.error);
	}
	shown();
	if (outcome.exited !== undefined) self.endApp?.(outcome.exited); // `exit` in `warp app` (src/web/webview.rs) closes it
}

// the page's markup anew, morphed into the page
function show() {
	const render = site.exports[PAGE_HTML];
	if (!render) return console.error(`warp: app.wasm exports no ${PAGE_HTML}: build it with warp build --site`);
	morphChildren(document.getElementById(SITE_ROOT), parsedHtml(plainOfTree(readNode(site.exports, render()))));
}

// only the holes of the page's markup anew, the whole markup when it has none
function showHoles() {
	const patches = holePatches(site.exports);
	if (!patches) return show();
	patches.forEach(patch => showPatch(document.getElementById(SITE_ROOT), patch));
}

async function hydrate() {
	const bytes = new Uint8Array(await (await fetch(SITE_MODULE)).arrayBuffer());
	// stored values and those a `warp dev` page keeps across reloads (host-files.js STD_ADAPTERS.store, markup.js)
	Object.assign(storedValues, keptValues());
	Object.assign(sessionValues, keptValues([SESSION_STORE]));
	// `database[k]`'s values (host-files.js, shipped when the program keeps values)
	await self.loadKept?.();
	self.keepStored = keepValue;
	const holder = instantiateProgram(bytes, siteHooks);
	if (holder.failure) return console.error("warp:", holder.failure);
	await globalThis.loadRouteModule?.(holder);
	const outcome = runMain(holder, siteHooks);
	if (outcome.result === undefined) return console.error("warp:", outcome.failure ?? outcome.trap ?? outcome.error);
	// a program with routes imports page_path, which ships host-routes.js
	globalThis.followSiteLinks?.(goTo);
	// kept values may differ from those the page was built with: show what main left
	if (site) showAfter(outcome, show);
	listenToElements(found => found && site.exports[`on·${found.event}·node`] && showAfter(runPageEvent(site, siteHooks, found.event, found.detail)));
}

// each event of the page to `handle`: its element's {event, detail} (markup.js elementEvent), else nothing; a key
// pressed anywhere but in a text field is the program's `on key` (card key-app: `warp app` shows a built site)
function listenToElements(handle) {
	for (const [event, detail] of Object.entries(SITE_EVENTS)) {
		document.getElementById(SITE_ROOT).addEventListener(FIELD_EVENT_TYPES[event] ?? event, happened => handle(elementEvent(event, happened, detail(happened))));
	}
	document.addEventListener("keydown", pressed => isTyping(pressed.target) || handle({ event: KEY_EVENT, detail: { key: pressed.key } }));
}

function isTyping(target) {
	return target?.isContentEditable || ["INPUT", "TEXTAREA", "SELECT"].includes(target?.tagName);
}

// the page at `path` (host-routes.js followSiteLinks: a link was followed, or the back button pressed)
async function goTo(path) {
	navigate(site, siteHooks, path);
	await loadRouteModule(site);
	show();
	focusRoute(document.getElementById(SITE_ROOT));
}

// a module that starts tasks runs in a Worker (site-thread.js, shipped only then)
(globalThis.startSiteWorker ?? hydrate)();
