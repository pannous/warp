// The routes of a page (a part of host.js, which says how parts work): the path that picks its route (page_path),
// going to another path (navigate) and, in a built site, its links and the back button (followSiteLinks), the module
// of each route (loadRouteModule) and the focus after a route change (focusRoute)

const ROOT_PATH = "/"; // the page path before any navigation (page_path)
const PAGE_ROUTED_EXPORT = "page·routed"; // the value of the route the path picks
const ROUTE_INDEX_EXPORT = "page·route_index"; // the index of the route the path picks
const ROUTE_MODULE_PREFIX = "app-route-"; // src/route_split.rs: the module of route N is app-route-N.wasm
const PLACEHOLDER_PREFIX = "placeholder."; // wasm-split's import module of the functions a route's module fills in

// the page goes to `path` (a link, the back button): page_path() gives it from now on, hooks.navigated hears it (a pending
// fetch is dropped there); the caller shows the page anew (site.js, worker.js)
function navigate(holder, hooks, path) {
	holder.pagePath = path;
	hooks.navigated?.(holder, path);
}

// a built site (site.js): a click on a link to a page of this site, without a modifier key or another target, and the
// back button go through the program's routes with goTo(path) (History API), so the page morphs instead of reloading
function followSiteLinks(goTo) {
	document.addEventListener("click", happened => {
		const link = happened.target.closest?.("a[href]");
		if (!link || happened.defaultPrevented || happened.button !== 0 || happened.metaKey || happened.ctrlKey || happened.shiftKey || happened.altKey) return;
		if ((link.target && link.target !== "_self") || link.hasAttribute("download")) return;
		const url = new URL(link.href, location.href);
		if (url.origin !== location.origin) return;
		happened.preventDefault();
		if (url.pathname + url.search !== location.pathname + location.search) history.pushState(null, "", url);
		goTo(url.pathname);
	});
	addEventListener("popstate", () => goTo(location.pathname));
}

// after going to another route, focus moves to what the route shows (card web-i18n, notes/web_framework.md): its main
// heading, else its main region, else the page's root, so a screen reader reads the new page instead of staying on
// the link; made focusable without entering the tab order
const ROUTE_FOCUS_TARGETS = ["main h1", "h1", "main"];
function focusRoute(root) {
	const target = ROUTE_FOCUS_TARGETS.map(selector => root.querySelector(selector)).find(Boolean) ?? root;
	if (!target.hasAttribute("tabindex")) target.setAttribute("tabindex", "-1");
	target.focus();
}

const loadedRoutes = new Map(); // a route module's name → its loading
// a built site split by route (src/route_split.rs): the module of the route the path picks, unless loaded already; it
// fills the program's table slots of that route's functions, instantiated with the program's exports as `primary`
function loadRouteModule(holder) {
	const name = ROUTE_MODULE_PREFIX + holder.exports[ROUTE_INDEX_EXPORT]?.();
	if (!WebAssembly.Module.imports(holder.run.module).some(entry => entry.module === PLACEHOLDER_PREFIX + name)) return;
	if (!loadedRoutes.has(name)) {
		loadedRoutes.set(name, fetch(`${name}.wasm`).then(response => response.arrayBuffer())
			.then(bytes => WebAssembly.instantiate(bytes, { primary: holder.exports })));
	}
	return loadedRoutes.get(name);
}

addHostPart({
	words: (holder, hooks, { program }) => ({
		// the path of the page shown, which picks its route (src/lowering/routes.rs); navigate changes it
		page_path: () => buildValue(program(), treeOfPlain(holder.pagePath ?? ROOT_PATH)),
	}),
});
