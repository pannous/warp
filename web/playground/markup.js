// Markup in the page, shared by the playground and the pages warp builds (card web-ssr): a program's markup shown anew
// changes only what differs (morphChildren), and an event inside it names its element's handler (elementEvent). The page
// also keeps the values of `stored x = v` and those a `warp dev` page keeps across reloads (keptValues, keepValue).

const INSTANCE_ATTRIBUTE = "data-wasp-instance"; // a component instance's elements (src/lowering/element_events.rs)
const KEY_ATTRIBUTE = "data-wasp-key"; // a list item's element (src/html.rs)
const NUMBER_FIELDS = ["number", "range"]; // fields whose bound value is a number
const TRANSITION_ATTRIBUTE = "data-wasp-transition"; // `li{ transition: fade 200ms }` (src/lowering/transitions.rs)
const LEAVING_ATTRIBUTE = "data-wasp-leaving"; // an element animating out: no longer matched, removed when done
const TRANSITION_DEFAULTS = { kind: "fade", duration: 200, easing: "ease" };
// the start of each kind of transition: an element enters from it and leaves towards it
const TRANSITION_FRAMES = {
	fade: { opacity: 0 },
	scale: { opacity: 0, transform: "scale(0.8)" },
	slide: { opacity: 0, transform: "translateY(-1em)" },
};
const DEV_STORE = "wasp-dev"; // the store of the values a `warp dev` page keeps (src/lowering/stored_values.rs DEV_STORE)
const STORED_PREFIX = "wasp stored "; // a stored value in localStorage, kept across visits
const DEV_PREFIX = "wasp dev "; // a value a `warp dev` page keeps in sessionStorage, across its reloads

// the children of shown become those of wanted: an element with a key (data-wasp-key, card web-keyed) is the shown one
// of that key, moved into place; any other node is matched by position; a node of another kind or tag is replaced.
// An element with a transition (card web-transitions) animates in, out and, keyed, to its new place; nothing animates
// when shown was empty (the first render)
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

// `fade 200ms ease-out` as its kind, duration and easing; null without a transition or when motion is unwanted
function transitionOf(node) {
	const spec = node.getAttribute?.(TRANSITION_ATTRIBUTE);
	if (spec == null || matchMedia("(prefers-reduced-motion: reduce)").matches) return null;
	return spec.split(/\s+/).filter(Boolean).reduce((transition, word) => {
		if (/^\d+ms$/.test(word)) return { ...transition, duration: parseInt(word) };
		if (word in TRANSITION_FRAMES) return { ...transition, kind: word };
		return { ...transition, easing: word };
	}, TRANSITION_DEFAULTS);
}

function enter(node) {
	const transition = transitionOf(node);
	if (transition) node.animate([TRANSITION_FRAMES[transition.kind], {}], transition);
}

// a node gone from the markup: with a transition it stays in place, unmatched, until it has animated out
function leave(node) {
	const transition = transitionOf(node);
	if (!transition) return node.remove();
	node.setAttribute(LEAVING_ATTRIBUTE, "");
	node.animate([{}, TRANSITION_FRAMES[transition.kind]], { ...transition, fill: "forwards" }).finished.then(() => node.remove(), () => node.remove());
}

// where each keyed element with a transition is shown now: after the morph a moved one glides from there (FLIP)
function transitionPlaces(shown) {
	return new Map([...shown.children].filter(child => isLive(child) && child.hasAttribute(KEY_ATTRIBUTE) && transitionOf(child)).map(child => [child, child.getBoundingClientRect()]));
}

function moveFrom(element, before) {
	const transition = transitionOf(element);
	if (!element.isConnected || !isLive(element) || !transition) return;
	const after = element.getBoundingClientRect();
	const [x, y] = [before.left - after.left, before.top - after.top];
	if (x || y) element.animate([{ transform: `translate(${x}px, ${y}px)` }, {}], transition);
}

function morphElement(shown, wanted) {
	[...shown.attributes].filter(({ name }) => !wanted.hasAttribute(name)).forEach(({ name }) => shown.removeAttribute(name));
	[...wanted.attributes].filter(({ name, value }) => shown.getAttribute(name) !== value).forEach(({ name, value }) => shown.setAttribute(name, value));
	// a field the user changed no longer follows its attributes: what it holds is set (card web-bind)
	if ("value" in shown && wanted.hasAttribute("value") && shown.value !== wanted.getAttribute("value")) shown.value = wanted.getAttribute("value");
	if ("checked" in shown) shown.checked = wanted.hasAttribute("checked");
	morphChildren(shown, wanted, true);
}

// an event inside shown markup, for the handler of the element it happened in (data-wasp-click="1": click·1), with
// its detail; in a component (data-wasp-instance="2", element_events.rs) the detail names its instance
function elementEvent(event, happened, detail) {
	const attribute = `data-wasp-${event}`;
	const path = happened.composedPath();
	const element = path.find(node => node.getAttribute?.(attribute));
	const instance = path.find(node => node.getAttribute?.(INSTANCE_ATTRIBUTE))?.getAttribute(INSTANCE_ATTRIBUTE);
	return element && { event: `${event}·${element.getAttribute(attribute)}`, detail: instance ? { ...detail, instance: Number(instance) } : detail };
}

// what a form field holds now: `input{ bind: name }` sets name to event.value (a number from a number or range field)
function inputDetail(field) {
	return { value: NUMBER_FIELDS.includes(field.type) ? field.valueAsNumber : field.value, checked: field.checked ?? false };
}

// where a value of a store is kept: the dev store in sessionStorage, any other in localStorage, as JSON
function keptStorage(file) {
	return file === DEV_STORE ? [sessionStorage, DEV_PREFIX] : [localStorage, STORED_PREFIX];
}

// the kept values of both stores by name, for host.js STD_ADAPTERS.store
function keptValues() {
	return Object.assign({}, ...["", DEV_STORE].map(file => {
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
		storage.setItem(prefix + name, JSON.stringify(value));
	} catch (failure) {
		console.error(`${name} could not be kept:`, failure);
	}
}
