// Markup in the page, shared by the playground and the pages warp builds (card web-ssr): a program's markup shown anew
// changes only what differs (morphChildren), and an event inside it names its element's handler (elementEvent). The page
// also keeps the values of `stored x = v` and those a `warp dev` page keeps across reloads (keptValues, keepValue).

const KEY_ATTRIBUTE = "data-warp-key"; // a list item's element (lib/markup.warp)
const NUMBER_FIELDS = ["number", "range"]; // fields whose bound value is a number
const LEAVING_ATTRIBUTE = "data-warp-leaving"; // an element animating out: no longer matched, removed when done
const DEV_STORE = "warp-dev"; // the store of the values a `warp dev` page keeps (src/lowering/data/stored_values.rs DEV_STORE)
const STORED_PREFIX = "warp stored "; // a stored value in localStorage, kept across visits
const DEV_PREFIX = "warp dev "; // a value a `warp dev` page keeps in sessionStorage, across its reloads
const SESSION_STORE = "warp-session"; // the store of `session[k]` (src/lowering/data/stored_values.rs SESSION_STORE)
const SESSION_PREFIX = "warp session "; // a value of `session[k]` in sessionStorage, while the tab lasts

// the children of shown become those of wanted: an element with a key (data-warp-key, card web-keyed) is the shown one
// of that key, moved into place; any other node is matched by position; a node of another kind or tag is replaced.
// An element with a transition (card web-transitions, markup-transitions.js) animates in, out and, keyed, to its new
// place; nothing animates when shown was empty (the first render)
function morphChildren(shown, wanted, animated = shown.hasChildNodes()) {
	const keyOf = node => node.getAttribute?.(KEY_ATTRIBUTE) ?? null;
	const keyed = new Map([...shown.children].filter(child => keyOf(child) !== null && isLive(child)).map(child => [keyOf(child), child]));
	const placesBefore = animated ? transitionPlaces(shown) : new Map();
	// an item gone from the list leaves where it stands, before the others move past it
	const wantedKeys = new Set([...wanted.children].map(keyOf));
	keyed.forEach((child, key) => wantedKeys.has(key) || leave(child));
	let current = nextLive(shown.firstChild);
	for (const node of [...wanted.childNodes]) {
		const old = keyOf(node) !== null ? keyed.get(keyOf(node)) : current && keyOf(current) === null ? current : undefined;
		if (!old) {
			shown.insertBefore(node, current);
			if (animated) enter(node);
			continue;
		}
		if (old === current) current = nextLive(current.nextSibling);
		else shown.insertBefore(old, current);
		if (old.nodeName !== node.nodeName) old.replaceWith(node);
		else if (old.nodeType === Node.ELEMENT_NODE) morphElement(old, node);
		else if (old.nodeValue !== node.nodeValue) old.nodeValue = node.nodeValue;
	}
	// the next node is found before leave removes this one, which has no next sibling after
	while (current) {
		const next = nextLive(current.nextSibling);
		leave(current);
		current = next;
	}
	placesBefore.forEach((before, element) => moveFrom(element, before));
}

const isLive = node => !node.hasAttribute?.(LEAVING_ATTRIBUTE);

function nextLive(node) {
	while (node && !isLive(node)) node = node.nextSibling;
	return node;
}

// without transitions an element just comes and goes; markup-transitions.js, loaded after this script by pages whose
// elements have a CSS transition (src/web/site.rs), replaces these four
function transitionPlaces() { return new Map(); }
function enter() {}
function leave(node) { node.remove(); }
function moveFrom() {}

function morphElement(shown, wanted) {
	[...shown.attributes].filter(({ name }) => !wanted.hasAttribute(name)).forEach(({ name }) => shown.removeAttribute(name));
	[...wanted.attributes].filter(({ name, value }) => shown.getAttribute(name) !== value).forEach(({ name, value }) => shown.setAttribute(name, value));
	// a field the user changed no longer follows its attributes: what it holds is set (card web-bind)
	if ("value" in shown && wanted.hasAttribute("value") && shown.value !== wanted.getAttribute("value")) shown.value = wanted.getAttribute("value");
	if ("checked" in shown) shown.checked = wanted.hasAttribute("checked");
	morphChildren(shown, wanted, true);
}

// an `emit` no handler of the program receives (card page-custom-event, host.js dispatch_event): a CustomEvent of
// its name on the page, the event's data as detail, for the page's own scripts (`document.addEventListener(name, …)`)
function dispatchToPage({ name, detail }) {
	document.dispatchEvent(new CustomEvent(name, { detail }));
}

// an event inside shown markup, for the handler of the element it happened in (data-warp-click="1": click·1), with
// its detail
function elementEvent(event, happened, detail) {
	const attribute = `data-warp-${event}`;
	const element = happened.composedPath().find(node => node.getAttribute?.(attribute));
	return element && { event: `${event}·${element.getAttribute(attribute)}`, detail: { ...detail, ids: idValues(happened.currentTarget ?? document) } };
}

// what the shown markup's fields with an id hold, by id: `$who` in a handler reads it (lib/markup.warp page_value,
// decision page-ids); the markup shown in a host's shadow root (the playground's #rendered) is searched there
function idValues(root) {
	const fields = (root.shadowRoot ?? root).querySelectorAll?.("input[id], textarea[id], select[id]") ?? [];
	return Object.fromEntries([...fields].map(field => [field.id, field.type === "checkbox" ? field.checked : field.value]));
}

// the element events that are no page events (card click-key, event_signals.rs FIELD_EVENTS), by the DOM event that
// raises each: focus and blur do not bubble, their focusin and focusout do
const FIELD_EVENT_TYPES = { change: "change", focus: "focusin", blur: "focusout" };

// what a form field holds now: `input{ bind: name }` sets name to event.value (a number from a number or range field)
function inputDetail(field) {
	return { value: NUMBER_FIELDS.includes(field.type) ? field.valueAsNumber : field.value, checked: field.checked ?? false };
}

// each event of a page its program hears, to `handle`: its element's {event, detail} (elementEvent) or nothing, and the
// DOM event; a key pressed anywhere but in a text field is the program's `on key` (lib/keys.warp). A built site
// (site.js) and the native warp's app window (lib/host/native_host/webview_bridge.js) listen so; markup-forms.js adds submit
const KEY_EVENT = "key";
const fieldDetail = happened => inputDetail(happened.composedPath()[0]);
function listenToElements(root, handle) {
	const events = { click: () => ({}), input: fieldDetail, ...Object.fromEntries(Object.keys(FIELD_EVENT_TYPES).map(event => [event, fieldDetail])), ...self.FORM_EVENTS };
	for (const [event, detail] of Object.entries(events)) {
		root.addEventListener(FIELD_EVENT_TYPES[event] ?? event, happened => handle(elementEvent(event, happened, detail(happened)), happened));
	}
	document.addEventListener("keydown", pressed => isTyping(pressed.target) || handle({ event: KEY_EVENT, detail: { key: pressed.key } }, pressed));
}
function isTyping(target) {
	return target?.isContentEditable || ["INPUT", "TEXTAREA", "SELECT"].includes(target?.tagName);
}

// text on the page's clipboard (`clipboard.write`, host-files.js); the browser refuses it without a recent user action
// (a click), loudly on the console
function copyText(text) {
	navigator.clipboard.writeText(text).catch(failure => console.error("clipboard:", failure));
}

// where a value of a store is kept: the dev store and the session's in sessionStorage, any other in localStorage, as JSON
function keptStorage(file) {
	return file === DEV_STORE ? [sessionStorage, DEV_PREFIX] : file === SESSION_STORE ? [sessionStorage, SESSION_PREFIX] : [localStorage, STORED_PREFIX];
}

// the kept values of the stores by name, for host-files.js STD_ADAPTERS.store: the program's and the dev store
// (storedValues), else those named (the session's: sessionValues)
function keptValues(files = ["", DEV_STORE]) {
	return Object.assign({}, ...files.map(file => {
		try {
			const [storage, prefix] = keptStorage(file);
			const keys = Object.keys(storage).filter(key => key.startsWith(prefix));
			return Object.fromEntries(keys.map(key => [key.slice(prefix.length), JSON.parse(storage.getItem(key))]));
		} catch (failure) {
			console.error("kept values could not be read:", failure);
			return {};
		}
	}));
}

function keepValue(name, value, file) {
	try {
		const [storage, prefix] = keptStorage(file);
		value === undefined ? storage.removeItem(prefix + name) : storage.setItem(prefix + name, JSON.stringify(value));
	} catch (failure) {
		console.error(`${name} could not be kept:`, failure);
	}
}

// the HTML as a fragment of DOM, not in the page
function parsedHtml(html) {
	const template = document.createElement("template");
	template.innerHTML = html;
	return template.content;
}

