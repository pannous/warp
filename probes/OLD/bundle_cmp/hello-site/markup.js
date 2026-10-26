const INSTANCE_ATTRIBUTE = "data-warp-instance"; // a component instance's elements (src/lowering/element_events.rs)
const KEY_ATTRIBUTE = "data-warp-key"; // a list item's element (lib/markup.warp)
const NUMBER_FIELDS = ["number", "range"]; // fields whose bound value is a number
const LEAVING_ATTRIBUTE = "data-warp-leaving"; // an element animating out: no longer matched, removed when done
const DEV_STORE = "warp-dev"; // the store of the values a `warp dev` page keeps (src/lowering/stored_values.rs DEV_STORE)
const STORED_PREFIX = "warp stored "; // a stored value in localStorage, kept across visits
const DEV_PREFIX = "warp dev "; // a value a `warp dev` page keeps in sessionStorage, across its reloads
const SESSION_STORE = "warp-session"; // the store of `session[k]` (src/lowering/stored_values.rs SESSION_STORE)
const SESSION_PREFIX = "warp session "; // a value of `session[k]` in sessionStorage, while the tab lasts
function morphChildren(shown, wanted, animated = shown.hasChildNodes()) {
const keyOf = node => node.getAttribute?.(KEY_ATTRIBUTE) ?? null;
const keyed = new Map([...shown.children].filter(child => keyOf(child) !== null && isLive(child)).map(child => [keyOf(child), child]));
const placesBefore = animated ? transitionPlaces(shown) : new Map();
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
function transitionPlaces() { return new Map(); }
function enter() {}
function leave(node) { node.remove(); }
function moveFrom() {}
function morphElement(shown, wanted) {
[...shown.attributes].filter(({ name }) => !wanted.hasAttribute(name)).forEach(({ name }) => shown.removeAttribute(name));
[...wanted.attributes].filter(({ name, value }) => shown.getAttribute(name) !== value).forEach(({ name, value }) => shown.setAttribute(name, value));
if ("value" in shown && wanted.hasAttribute("value") && shown.value !== wanted.getAttribute("value")) shown.value = wanted.getAttribute("value");
if ("checked" in shown) shown.checked = wanted.hasAttribute("checked");
morphChildren(shown, wanted, true);
}
function elementEvent(event, happened, detail) {
const attribute = `data-warp-${event}`;
const path = happened.composedPath();
const element = path.find(node => node.getAttribute?.(attribute));
const instance = path.find(node => node.getAttribute?.(INSTANCE_ATTRIBUTE))?.getAttribute(INSTANCE_ATTRIBUTE);
return element && { event: `${event}·${element.getAttribute(attribute)}`, detail: instance ? { ...detail, instance: Number(instance) } : detail };
}
function inputDetail(field) {
return { value: NUMBER_FIELDS.includes(field.type) ? field.valueAsNumber : field.value, checked: field.checked ?? false };
}
function copyText(text) {
navigator.clipboard.writeText(text).catch(failure => console.error("clipboard:", failure));
}
function keptStorage(file) {
return file === DEV_STORE ? [sessionStorage, DEV_PREFIX] : file === SESSION_STORE ? [sessionStorage, SESSION_PREFIX] : [localStorage, STORED_PREFIX];
}
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
