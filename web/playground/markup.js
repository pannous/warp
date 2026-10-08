// Markup in the page, shared by the playground and the pages warp builds (card web-ssr): a program's markup shown anew
// changes only what differs (morphChildren), and an event inside it names its element's handler (elementEvent). The page
// also keeps the values of `stored x = v` and those a `warp dev` page keeps across reloads (keptValues, keepValue).

const INSTANCE_ATTRIBUTE = "data-warp-instance"; // a component instance's elements (src/lowering/element_events.rs)
const KEY_ATTRIBUTE = "data-warp-key"; // a list item's element (lib/markup.warp)
const NUMBER_FIELDS = ["number", "range"]; // fields whose bound value is a number
const LEAVING_ATTRIBUTE = "data-warp-leaving"; // an element animating out: no longer matched, removed when done
const DEV_STORE = "warp-dev"; // the store of the values a `warp dev` page keeps (src/lowering/stored_values.rs DEV_STORE)
const STORED_PREFIX = "warp stored "; // a stored value in localStorage, kept across visits
const DEV_PREFIX = "warp dev "; // a value a `warp dev` page keeps in sessionStorage, across its reloads
const SESSION_STORE = "warp-session"; // the store of `session[k]` (src/lowering/stored_values.rs SESSION_STORE)
const SESSION_PREFIX = "warp session "; // a value of `session[k]` in sessionStorage, while the tab lasts
// a store whose file ends so is `database[k]`'s, kept in IndexedDB (src/lowering/stored_values.rs DATABASE_STORE)
const DATABASE_STORE = "database.json";
const DATABASE = { name: "warp", version: 1, objects: "values" }; // the IndexedDB database and its object store

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
	for (; current; current = nextLive(current.nextSibling)) leave(current);
	placesBefore.forEach((before, element) => moveFrom(element, before));
}

const isLive = node => !node.hasAttribute?.(LEAVING_ATTRIBUTE);

function nextLive(node) {
	while (node && !isLive(node)) node = node.nextSibling;
	return node;
}

// without transitions an element just comes and goes; markup-transitions.js, loaded after this script by pages whose
// elements have a CSS transition (src/site.rs), replaces these four
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

// an event inside shown markup, for the handler of the element it happened in (data-warp-click="1": click·1), with
// its detail; in a component (data-warp-instance="2", element_events.rs) the detail names its instance
function elementEvent(event, happened, detail) {
	const attribute = `data-warp-${event}`;
	const path = happened.composedPath();
	const element = path.find(node => node.getAttribute?.(attribute));
	const instance = path.find(node => node.getAttribute?.(INSTANCE_ATTRIBUTE))?.getAttribute(INSTANCE_ATTRIBUTE);
	return element && { event: `${event}·${element.getAttribute(attribute)}`, detail: instance ? { ...detail, instance: Number(instance) } : detail };
}

// what a form field holds now: `input{ bind: name }` sets name to event.value (a number from a number or range field)
function inputDetail(field) {
	return { value: NUMBER_FIELDS.includes(field.type) ? field.valueAsNumber : field.value, checked: field.checked ?? false };
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
	if (isDatabase(file)) return keepInDatabase(name, value);
	try {
		const [storage, prefix] = keptStorage(file);
		value === undefined ? storage.removeItem(prefix + name) : storage.setItem(prefix + name, JSON.stringify(value));
	} catch (failure) {
		console.error(`${name} could not be kept:`, failure);
	}
}

const isDatabase = file => file?.endsWith(DATABASE_STORE);

// the IndexedDB object store of `database[k]` in a transaction of `mode`; a page without IndexedDB rejects
function databaseObjects(mode) {
	return new Promise((resolve, reject) => {
		const opening = indexedDB.open(DATABASE.name, DATABASE.version);
		opening.onupgradeneeded = () => opening.result.createObjectStore(DATABASE.objects);
		opening.onsuccess = () => resolve(opening.result.transaction(DATABASE.objects, mode).objectStore(DATABASE.objects));
		opening.onerror = () => reject(opening.error);
	});
}

const requested = request => new Promise((resolve, reject) => {
	request.onsuccess = () => resolve(request.result);
	request.onerror = () => reject(request.error);
});

// the values of `database[k]` by name, for host-files.js STD_ADAPTERS.store (databaseValues); none when IndexedDB fails
async function keptDatabase() {
	try {
		const objects = await databaseObjects("readonly");
		const [names, values] = await Promise.all([requested(objects.getAllKeys()), requested(objects.getAll())]);
		return Object.fromEntries(names.map((name, index) => [name, values[index]]));
	} catch (failure) {
		console.error("database values could not be read:", failure);
		return {};
	}
}

function keepInDatabase(name, value) {
	databaseObjects("readwrite")
		.then(objects => requested(value === undefined ? objects.delete(name) : objects.put(value, name)))
		.catch(failure => console.error(`${name} could not be kept in the database:`, failure));
}
