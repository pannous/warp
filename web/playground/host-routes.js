// The routes of a page (a part of host.js, which says how parts work): a program serving the page's requests (lib/web.warp
// page_serve, WARP_SERVE=page) answers HTTP text, which the page writes (requestText) and reads (answerOf), the
// playground's address bar, links and forms or a built site's (followSiteLinks, focusRoute). A built site loads this
// file twice: the page for its requests, links and focus (without host.js), its Worker for the program's side

const ROOT_PATH = "/"; // the page path before any navigation (page_path)
// a form's fields as a browser posts them
const FORM_TYPE = "application/x-www-form-urlencoded";
// a redirect (a form's POST answered 303 See Other, lib/web.warp) is followed as a browser follows it: the GET of its
// Location
const REDIRECT_STATUSES = [301, 302, 303, 307, 308];

// a request of the page as HTTP text; a body is a form's fields, urlencoded, with the page it came from
function requestText(method, target, body = "", referer = ROOT_PATH) {
	const length = new TextEncoder().encode(body).length;
	const typed = body ? `Content-Type: ${FORM_TYPE}\r\nReferer: ${referer || ROOT_PATH}\r\n` : "";
	return `${method} ${target} HTTP/1.1\r\nHost: localhost\r\n${typed}Content-Length: ${length}\r\n\r\n${body}`;
}

// the program's HTTP answer: { redirect, the Location to GET } or { failed, html (an HTML body of a success), text }; a
// failing status's body is the error whatever its content type (lib/web.warp http_failure answers text/html)
function answerOf(response) {
	const [head, ...body] = response.split("\r\n\r\n");
	const text = body.join("\r\n\r\n");
	const status = Number(head.split(" ")[1]);
	const redirect = REDIRECT_STATUSES.includes(status) ? head.match(/^location:\s*(\S+)/im)?.[1] : undefined;
	if (redirect) return { redirect };
	const failed = status >= 400;
	return { failed, html: !failed && /^content-type:\s*text\/html/im.test(head) ? text : undefined, text };
}

// a built site (site.js): a click on a link to a page of this site, without a modifier key or another target, and the
// back button go to the program serving it with goTo(path) (History API), so the page morphs instead of reloading
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

// `serve` in the playground (lib/web.warp page_serve, WARP_SERVE=page): the program waits for the page's next request
// (its address bar, a link, a form) as a server waits for its next client, however long, and gives the page its
// answer; HTTP text both ways (requestText, answerOf)
const PAGE_SERVING = {
	request(port) {
		if (!self.servingPage) throw new Error(`serve ${port}: this page sends no requests`);
		return askPage(`serve ${port}`, shared => self.servingPage(shared, port), Infinity).request;
	},
	answer(response) {
		self.servedPage(response);
		return null;
	},
};

if (globalThis.addHostPart) addHostPart({
	adapters: { web: PAGE_SERVING },
	words: (holder, hooks, { program }) => ({
		// the path of the page shown (core/analysis/host_words.warp page_path)
		page_path: () => buildValue(program(), treeOfPlain(holder.pagePath ?? ROOT_PATH)),
	}),
});
