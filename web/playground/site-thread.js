// The page of a built site whose program runs in a Worker (card site-worker, src/site.rs): a module that starts tasks
// runs off the page's thread in site-worker.js, so a blocking `await` may wait and its tasks run together on the task
// Workers. The page keeps the DOM: site.js calls startSiteWorker instead of hydrate, which sends the Worker the module,
// the kept values, the path and each element event, and morphs in the markup it answers. Shared memory needs cross-origin
// isolation, which a static host gives through coi-serviceworker.js (registered here, then the page reloads once).
// Relative URLs resolve against the site's root (the page's <base>).

const WORKER_ATTRIBUTE = "data-wasp-worker";

if (!self.crossOriginIsolated && "serviceWorker" in navigator && !sessionStorage.getItem("isolating")) {
	navigator.serviceWorker.register("coi-serviceworker.js").then(() => {
		sessionStorage.setItem("isolating", "1");
		if (!navigator.serviceWorker.controller) location.reload();
	}, () => {});
}

// the Worker running the program of the site's module (site.js SITE_MODULE), its markup shown in the root
function startSiteWorker() {
	const root = document.getElementById(SITE_ROOT);
	const worker = new Worker(`site-worker.js?scripts=${root.getAttribute(WORKER_ATTRIBUTE)}`);
	worker.onmessage = ({ data }) => {
		if (data.html !== undefined) {
			const template = document.createElement("template");
			template.innerHTML = data.html;
			morphChildren(root, template.content);
		}
		if (data.stored) keepValue(data.stored.name, data.stored.value, data.stored.file);
		if (data.print) (data.print.stream === 2 ? console.error : console.log)(data.print.text.replace(/\n$/, ""));
		if (data.failure) console.error("wasp:", data.failure);
	};
	worker.postMessage({ start: { module: new URL(SITE_MODULE, document.baseURI).href, stored: keptValues(), path: location.pathname } });
	listenToElements(found => found && worker.postMessage(found));
}
