// The program of a built site in a Worker (card site-worker): a site whose module starts tasks runs here, off the page's
// thread, so a blocking `await` may wait (Atomics.wait) and its tasks run on the pool of task Workers (host-tasks.js).
// The page (site.js) keeps the DOM: it sends the module's URL, the kept values (the session's apart), the page's path, the
// server's replies of a rendered page and each element event;
// this worker answers with the page's markup anew (its export page·html, as site.js show() renders it) after main and
// each path the page went to, with its holes anew (host.js holePatches, card web-fine-rest) after each handler and timer (a link or the back button: its route, its module loaded first).
// Its scripts come as ?scripts=…, resolved against this file, the site's root (src/web/site.rs).

const SITE_SCRIPTS = new URL(self.location.href).searchParams.get("scripts").split(",");
importScripts(...SITE_SCRIPTS);
self.siteScripts = SITE_SCRIPTS; // the task Workers load the same (host-tasks.js addTaskWorker)
self.prepareTaskPool?.(); // host-tasks.js, which a site that only paints does not ship (card site-frames)

const PAGE_HTML = "page·html";
let site; // the program's run (host.js runProgram's holder)
let pagePath = "/";

const post = (message, transfer) => self.postMessage(message, transfer);
self.keepStored = (name, value, file) => post({ stored: { name, value, file } }); // host-files.js STD_ADAPTERS.store
self.writeClipboard = text => post({ clipboard: text }); // host-files.js STD_ADAPTERS.clipboard: the page has the clipboard
self.askPageClipboard = shared => post({ clipboardRead: shared }); // host.js clipboard_text
self.askPageDecode = (shared, url, bytes) => post({ soundDecode: { shared, url, bytes } }); // host-sound.js pageDecoded
self.askPageCall = (shared, request) => post({ pageCall: { shared, request } }); // host-foreign.js callOnPage
const hooks = {
	pagePath: () => pagePath,
	instantiated: holder => { site = holder; },
	print: (text, stream) => post({ print: { text, stream } }),
	dispatch: event => post({ dispatch: event }), // site-thread.js: markup.js dispatchToPage
	paint: (frame, width, height, canvas, shaded) => post({ paint: { frame, width, height, shaded } }, frame && [frame.words.buffer]), // shown by the page (site-thread.js)
	listen: holder => holder.timers && startTimers(holder, handler => showAfter(runTimer(holder, hooks, handler))), // host-timers.js
	arrived: (holder, handler) => holder === site && showAfter(runTimer(holder, hooks, handler)),
};

// what main or a handler left: the page's markup anew (a handler's: its holes), or the failure (which stops the timers)
function showAfter(outcome, shown = changedMarkup) {
	if (outcome.result === undefined) {
		site?.stopTimers?.();
		return post({ failure: outcome.failure ?? outcome.trap ?? outcome.error });
	}
	post({ ...shown(), exited: outcome.exited });
}

// the holes of the page's markup anew, the whole markup when it has none
function changedMarkup() {
	const patches = holePatches(site.exports);
	return patches ? { patches } : pageMarkup();
}

// the page's markup anew, as a message to the page
function pageMarkup() {
	const render = site.exports[PAGE_HTML];
	if (!render) return { failure: `app.wasm exports no ${PAGE_HTML}: build it with warp build --site` };
	return { html: plainOfTree(readNode(site.exports, render())) };
}

async function start({ module, stored, session, path, replies }) {
	if (replies) serverReplies.push(...JSON.parse(replies)); // host-tasks.js: a page a server rendered for its path (P221)
	Object.assign(storedValues, stored);
	Object.assign(sessionValues, session);
	await self.loadKept?.(); // host-files.js, when the program keeps values
	pagePath = path;
	const bytes = new Uint8Array(await (await fetch(module)).arrayBuffer());
	await self.taskPoolReady?.(); // tasks run on loaded Workers, not inline
	const holder = instantiateProgram(bytes, hooks);
	if (holder.failure) return post({ failure: holder.failure });
	await globalThis.loadRouteModule?.(holder);
	showAfter(runMain(holder, hooks), pageMarkup);
}

// the page went to `path` (host-routes.js followSiteLinks): its route shown, the page told to move the focus there
async function goTo(path) {
	navigate(site, hooks, path);
	await loadRouteModule(site);
	post({ ...pageMarkup(), navigated: true });
}

self.onmessage = ({ data }) => {
	if (data.pointer) return self.pagePointer = sharedPointer(data.pointer); // host.js system_value
	if (data.start) return start(data.start);
	if (data.navigate !== undefined) return goTo(data.navigate).catch(failure => post({ failure: String(failure.message ?? failure) }));
	if (data.event && site?.exports[`on·${data.event}·node`]) showAfter(runPageEvent(site, hooks, data.event, data.detail));
};
