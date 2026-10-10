// The page of a built site whose program runs in a Worker (card site-worker, src/site.rs): a module that starts tasks
// runs off the page's thread in site-worker.js, so a blocking `await` may wait and its tasks run together on the task
// Workers. The page keeps the DOM: site.js calls startSiteWorker instead of hydrate, which sends the Worker the module,
// the kept values, the path, the replies of a page a server rendered, each element event and each path a link or the back button goes to (host-routes.js
// followSiteLinks, shipped with routes), and morphs in the markup it answers. Shared memory needs cross-origin
// isolation, which a static host gives through coi-serviceworker.js (registered here, then the page reloads once).
// Relative URLs resolve against the site's root (the page's <base>).

const WORKER_ATTRIBUTE = "data-warp-worker";
const REPLIES_ELEMENT = "warp-replies"; // src/site.rs REPLIES_ID: the Worker answers its first fetches from them (host-tasks.js)

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

// the Worker running the program of the site's module (site.js SITE_MODULE), its markup shown in the root
async function startSiteWorker() {
	await siteStarts;
	const root = document.getElementById(SITE_ROOT);
	const worker = new Worker(`site-worker.js?scripts=${root.getAttribute(WORKER_ATTRIBUTE)}`);
	worker.onmessage = ({ data }) => {
		if (data.html !== undefined) {
			const template = document.createElement("template");
			template.innerHTML = data.html;
			morphChildren(root, template.content);
			if (data.navigated) focusRoute(root);
		}
		if (data.stored) keepValue(data.stored.name, data.stored.value, data.stored.file);
		if (data.clipboard !== undefined) copyText(data.clipboard);
		if (data.paint) showPainting(root, data.paint);
		if (data.print) (data.print.stream === 2 ? console.error : console.log)(data.print.text.replace(/\n$/, ""));
		if (data.failure) console.error("warp:", data.failure);
	};
	if (globalThis.pointerMessage) { // canvas.js, shipped when the program paints
		const shared = pointerMessage();
		if (shared) worker.postMessage(shared);
		followPointer(document.body);
	}
	worker.postMessage({ start: { module: new URL(SITE_MODULE, document.baseURI).href, stored: keptValues(), session: keptValues([SESSION_STORE]), path: location.pathname, replies: document.getElementById(REPLIES_ELEMENT)?.textContent } });
	listenToElements(found => found && worker.postMessage(found));
	globalThis.followSiteLinks?.(path => worker.postMessage({ navigate: path }));
}

// a painting of the program (card site-frames, canvas.js) in the one canvas after the root, which the root's markup
// morphing leaves alone: an animation's frames are drawn into it, a painting of another size replaces it
function showPainting(root, painting) {
	const shown = root.nextElementSibling?.matches("canvas.painting") ? root.nextElementSibling : undefined;
	if (shown && sameSize(shown, painting)) return drawn(shown, painting);
	const canvas = paintingCanvas(painting);
	if (shown) shown.replaceWith(canvas);
	else root.after(canvas);
}
