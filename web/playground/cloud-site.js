// A warp program using the browser (paint, the GPU, sound, `on frame`; card deploy-browser-site) as a Cloudflare
// Worker: the Worker serves the program's site and the visitor's browser runs it, as a site `warp build --site` builds
// runs (src/host/page_parts.rs). Composed before this (src/web/deploy.rs, web/hosting): SITE = {title, scripts,
// workerScripts, files}, the page's scripts and those of the program's Worker by name, and each shipped file's text.
import programBytes from "./app.bin";

const PAGE_FILE = "index.html";
const MODULE_FILE = "app.wasm";
// the page and the program's Worker share memory, which needs cross-origin isolation
const ISOLATION = { "cross-origin-opener-policy": "same-origin", "cross-origin-embedder-policy": "require-corp" };
const TYPES = { html: "text/html; charset=utf-8", js: "text/javascript; charset=utf-8", wasm: "application/wasm" };
const NOT_FOUND = { status: 404, headers: { "content-type": "text/plain; charset=utf-8" } };

const escaped = text => text.replace(/&/g, "&amp;").replace(/</g, "&lt;");
// the page as page_parts.rs page() writes it, without prerendered markup: the program shows itself once it runs
const sitePage = ({ title, scripts, workerScripts }) => `<!doctype html>
<html>
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>${escaped(title)}</title>
</head>
<body>
<div id="warp-root"${workerScripts.length ? ` data-warp-worker="${workerScripts.join(",")}"` : ""}></div>
${scripts.map(name => `<script src="${name}"></script>\n`).join("")}</body>
</html>
`;

function file(name) {
	if (name === PAGE_FILE) return sitePage(SITE);
	if (name === MODULE_FILE) return programBytes;
	return SITE.files[name];
}

export default {
	fetch(request) {
		const name = new URL(request.url).pathname.slice(1) || PAGE_FILE;
		const body = file(name);
		if (body === undefined) return new Response(`no file ${name}`, NOT_FOUND);
		const type = TYPES[name.split(".").pop()] ?? "application/octet-stream";
		return new Response(body, { headers: { "content-type": type, ...ISOLATION } });
	},
};
