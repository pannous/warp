// Cross-origin isolation for a host that cannot send headers (GitHub Pages): this service worker adds them to every
// response of the page, so SharedArrayBuffer exists and tasks run on Workers (host.js prepareTaskPool). index.html
// registers it and reloads once; the test server sends the headers itself (test_in_browser.py).
self.addEventListener("install", () => self.skipWaiting());
self.addEventListener("activate", event => event.waitUntil(self.clients.claim()));
self.addEventListener("fetch", event => {
	const request = event.request;
	if (request.cache === "only-if-cached" && request.mode !== "same-origin") return;
	// isolation headers matter only on this site's own responses; a cross-origin request (the bug button's POST to
	// pannous.com) goes to the network untouched: refetched here it failed in Safari with "Load failed"
	if (new URL(request.url).origin !== self.location.origin) return;
	event.respondWith(fetch(request).then(response => {
		if (response.status === 0) return response; // opaque: nothing to add
		const headers = new Headers(response.headers);
		headers.set("Cross-Origin-Embedder-Policy", "require-corp");
		headers.set("Cross-Origin-Opener-Policy", "same-origin");
		return new Response(response.body, { status: response.status, statusText: response.statusText, headers });
	}));
});
