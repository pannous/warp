// The routes of a page (a part of host.js, which says how parts work): the path that picks its route (page_path),
// going to another path (navigate) and, in a built site, its links and the back button (followSiteLinks)

const ROOT_PATH = "/"; // the page path before any navigation (page_path)
const PAGE_ROUTED_EXPORT = "page·routed"; // the value of the route the path picks

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

addHostPart({
	words: (holder, hooks, { program }) => ({
		// the path of the page shown, which picks its route (src/lowering/routes.rs); navigate changes it
		page_path: () => buildValue(program(), treeOfPlain(holder.pagePath ?? ROOT_PATH)),
	}),
});
