// The page of a built site (`warp build --site`, extensions/commands/site.warp): app.wasm runs in site-worker.js as the playground runs a program,
// and this page keeps the DOM. It morphs in the markup the program renders (lib/markup.warp page_render) after main,
// each element event (markup.js elementEvent: the handler page_event runs) and each timer. A program serving requests
// (lib/web.warp page_serve) gets the page's: its address first, then each link followed, the back button and each form,
// HTTP text both ways (host-routes.js requestText, answerOf). Shared memory needs cross-origin isolation, which a static
// host gives through coi-serviceworker.js (registered here, then the page reloads once).
// Needs markup.js, markup-forms.js, canvas.js, task-workers.js and host-routes.js.

const SITE_ROOT = "warp-root";
const SITE_MODULE = "app.wasm";
const SITE_WORKER = "web/playground/site-worker.js";

// resolves once no reload for isolation is coming: a Worker started before would have its request aborted (Firefox:
// NS_BINDING_ABORTED, card firefox-worker)
const siteStarts = new Promise(start => {
	if (self.crossOriginIsolated || !("serviceWorker" in navigator) || sessionStorage.getItem("isolating")) return start();
	navigator.serviceWorker.register("coi-serviceworker.js").then(() => {
		sessionStorage.setItem("isolating", "1");
		if (navigator.serviceWorker.controller) start();
		else location.reload();
	}, start);
});

const root = () => document.getElementById(SITE_ROOT);
let serving; // a serving program's: { shared } while it waits for a request, { waiting } a request it gets once it waits
let navigated = false; // the next page shown follows a link: the focus moves there

// a request for the serving program, kept until it waits
function request(method, target, body = "") {
	if (!serving?.shared) return serving = { waiting: [method, target, body] };
	const { shared } = serving;
	serving = {};
	answerInto(shared, async () => ({ request: requestText(method, target, body, location.pathname) })); // task-workers.js
}

// the serving program's answer: its page shown, a redirect followed (the address moves along), a failure on the console
function served(response) {
	const { redirect, failed, html, text } = answerOf(response);
	if (redirect) {
		history.replaceState(null, "", redirect);
		return request("GET", redirect);
	}
	if (failed || html === undefined) return console.error("warp:", text);
	show(html);
}

function show(html) {
	if (html === undefined) return;
	morphChildren(root(), parsedHtml(html));
	if (navigated) focusRoute(root());
	navigated = false;
}

function waitsForRequest(shared) {
	const first = serving === undefined;
	const waiting = first ? ["GET", location.pathname + location.search, ""] : serving.waiting;
	serving = { shared };
	if (first) followSiteLinks(path => { navigated = true; request("GET", path); });
	if (waiting) request(...waiting);
}

// a form with no handler of its own goes to the serving program, as a browser sends it to `warp serve`
function submitted(event, found) {
	if (found || !serving) return;
	event.preventDefault();
	const form = event.target;
	const method = (form.getAttribute("method") ?? "get").toUpperCase();
	const encoded = new URLSearchParams(new FormData(form, event.submitter)).toString();
	const path = new URL(form.getAttribute("action") ?? "", location.href).pathname;
	request(method, method === "GET" ? `${path}?${encoded}` : path, method === "GET" ? "" : encoded);
}

async function startSite() {
	await siteStarts;
	const worker = serveTaskWorkers(new Worker(SITE_WORKER)); // task-workers.js
	worker.onmessage = ({ data }) => {
		show(data.html);
		if (data.serving) waitsForRequest(data.serving);
		if (data.served !== undefined) served(data.served);
		if (data.dispatch) dispatchToPage(data.dispatch); // card page-custom-event
		if (data.stored) keepValue(data.stored.name, data.stored.value, data.stored.file);
		if (data.clipboard !== undefined) copyText(data.clipboard);
		if (data.clipboardRead) pasteInto(data.clipboardRead);
		if (data.soundDecode) decodeSoundInto(data.soundDecode.shared, data.soundDecode.url, data.soundDecode.bytes);
		if (data.pageCall) answerPageCall(data.pageCall.shared, data.pageCall.request, worker);
		if (data.paint) showPainting(root(), wholePainting(data.paint));
		if (data.print) (data.print.stream === 2 ? console.error : console.log)(data.print.text.replace(/\n$/, ""));
		if (data.failure) console.error("warp:", data.failure);
	};
	if (globalThis.pointerMessage) { // canvas.js: the pointer for the program's system values
		const shared = pointerMessage();
		if (shared) worker.postMessage(shared);
		followPointer(document.body);
	}
	worker.postMessage({ start: { module: new URL(SITE_MODULE, document.baseURI).href, stored: keptValues(), session: keptValues([SESSION_STORE]) } });
	listenToElements(root(), (found, happened) => {
		if (happened.type === "submit") submitted(happened, found);
		if (found) worker.postMessage(found);
	});
}

// a painting of the program (canvas.js) in the one canvas after the root, which the root's markup morphing leaves alone:
// an animation's frames are drawn into it, a painting of another size replaces it
function showPainting(root, painting) {
	const shown = root.nextElementSibling?.matches("canvas.painting") ? root.nextElementSibling : undefined;
	if (shown && sameSize(shown, painting)) return drawn(shown, painting);
	const canvas = paintingCanvas(painting);
	if (shown) shown.replaceWith(canvas);
	else root.after(canvas);
}

startSite();
