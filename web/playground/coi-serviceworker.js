// Cross-origin isolation for a host that cannot send headers (GitHub Pages): this service worker adds them to every
// response of the page, so SharedArrayBuffer exists and tasks run on Workers (host.js prepareTaskPool). index.html
// registers it and reloads once; the test server sends the headers itself (test_in_browser.py).
self.addEventListener("install", () => self.skipWaiting());
self.addEventListener("activate", event => event.waitUntil(self.clients.claim()));
self.addEventListener("fetch", event => {
	const request = event.request;
	if (request.cache === "only-if-cached" && request.mode !== "same-origin") return;
	event.respondWith(fetch(request).then(response => {
		if (response.status === 0) return response; // opaque: nothing to add
		const headers = new Headers(response.headers);
		headers.set("Cross-Origin-Embedder-Policy", "require-corp");
		headers.set("Cross-Origin-Opener-Policy", "same-origin");
		return new Response(response.body, { status: response.status, statusText: response.statusText, headers });
	}));
});
