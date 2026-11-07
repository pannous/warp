// The program of a built site in a Worker (scripts/build-site.sh): app.wasm runs as the playground runs a compiled
// program (driver.js programRan), off the page's thread, so a blocking wait may wait (Atomics.wait) and its tasks run on
// the pool of task Workers (host-tasks.js). The page (site.js) keeps the DOM: it sends the module's URL, the kept values
// and each element event; this worker answers with the page's markup (lib/markup.warp page_render) after main and after
// each handler (page_event) and timer. A program serving requests (lib/web.warp page_serve) waits for the page's
// (`serving`) and answers each (`served`), HTTP text both ways (host-routes.js).
// The site mirrors the repository's layout, this file at web/playground/, so FILE_ROOT (host-files.js) is the site's root.

importScripts("imports.js", "js-members.js", "std.js", "host.js");
importScripts(...HOST_PART_FILES, "components.js", "served-files.js", "wasi.js", "driver.js");
prepareTaskPool(); // host-tasks.js

const SITE_PROGRAM = "app.warp"; // the program's name, as `warp app.warp` names it (WARP_PROGRAM)
let site; // the program's run (driver.js programRan's holder)

const post = (message, transfer) => self.postMessage(message, transfer);
self.keepStored = (name, value, file) => post({ stored: { name, value, file } }); // host-files.js STD_ADAPTERS.store
self.writeClipboard = text => post({ clipboard: text }); // host-files.js STD_ADAPTERS.clipboard: the page has the clipboard
self.askPageClipboard = shared => post({ clipboardRead: shared }); // host.js clipboard_text
self.askPageDecode = (shared, url, bytes) => post({ soundDecode: { shared, url, bytes } }); // host-sound.js pageDecoded
self.askPageCall = (shared, request) => post({ pageCall: { shared, request } }); // host-foreign.js callOnPage
self.servingPage = shared => post({ serving: shared }); // host-routes.js PAGE_SERVING
self.servedPage = response => post({ served: response });
const hooks = {
	print: (text, stream) => post({ print: { text, stream } }),
	dispatch: event => post({ dispatch: event }), // site.js: markup.js dispatchToPage
	paint: (frame, width, height, canvas, shaded) => post({ paint: { frame, width, height, shaded } }, frame && [frame.words.buffer]), // shown by the page (site.js)
	listen: holder => startTimers(holder, (id, answer) => timerShown(holder, id, answer)), // host-timers.js
};

// a timer fired (driver.js timerFired): the page anew, or the failure, which stops the timers
function timerShown(holder, id, answer) {
	const { failure } = timerFired(holder, id, answer);
	if (failure === undefined) return post({ html: pageHtml(holder) });
	post({ failure });
	holder.stopTimers();
}

async function start({ module, stored, session }) {
	Object.assign(storedValues, stored);
	Object.assign(sessionValues, session);
	await self.loadKept?.(); // host-files.js, when the program keeps values
	const bytes = new Uint8Array(await (await fetch(module)).arrayBuffer());
	await taskPoolReady(); // tasks run on loaded Workers, not inline
	const { status, said, holder } = programRan(bytes, SITE_PROGRAM, hooks);
	if (status !== 0) return post({ failure: said || `app.wasm ended with status ${status}` });
	site = holder;
	post({ html: pageHtml(holder) });
	if (holder.timers?.length) hooks.listen(holder);
}

// an element event (click·1): its handler, then the page anew
function handleEvent({ event, detail }) {
	if (!site?.exports.page_event) return post({ failure: `on ${event}: app.wasm handles no page events` });
	const { html, value, error } = pageEventHandled(site, event, detail);
	post(error ? { failure: value } : { html });
}

self.onmessage = ({ data }) => {
	if (data.pointer) return self.pagePointer = sharedPointer(data.pointer); // host.js system_value
	if (data.start) return start(data.start).catch(failure => post({ failure: String(failure.message ?? failure) }));
	if (data.event) handleEvent(data);
};
