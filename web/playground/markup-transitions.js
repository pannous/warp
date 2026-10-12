// Transitions (a part of markup.js, card web-css, P188: stay close to HTML and CSS): an element's inline CSS
// transition (`li{ transition: opacity 200ms }`, src/lowering/transitions.rs) times it; its starting style
// (`starting-style: { opacity: 0 }`, CSS's @starting-style, which an inline style cannot hold) is what it enters from
// when it appears and leaves towards before it is removed; a keyed element whose transition covers transform glides to
// its new place (FLIP). Loaded after markup.js only by pages that have transitions (src/site.rs), it replaces
// markup.js's transitionPlaces, enter, leave and moveFrom.

const STARTING_STYLE_ATTRIBUTE = "data-warp-starting-style";
const KEYFRAMES_ATTRIBUTE = "data-warp-keyframes"; // `keyframes: [{…} {…}]`: frames of declarations, joined by " | "
const MOVING_PROPERTIES = ["transform", "all"]; // a CSS transition of these glides a keyed element to its new place

// the frames an element enters through and leaves through backwards (its keyframes, else from its starting style: CSS's
// @starting-style, which an inline style cannot hold), the timing of its CSS transition and how long a move takes; null without one or when motion is unwanted
function transitionOf(node) {
	if (!node.getAttribute || matchMedia("(prefers-reduced-motion: reduce)").matches) return null;
	const style = getComputedStyle(node);
	const durations = style.transitionDuration.split(", ").map(duration => parseFloat(duration) * 1000);
	const duration = Math.max(...durations);
	if (!duration) return null;
	const moving = style.transitionProperty.split(", ").findIndex(property => MOVING_PROPERTIES.includes(property));
	const starting = node.getAttribute(STARTING_STYLE_ATTRIBUTE);
	const frames = node.getAttribute(KEYFRAMES_ATTRIBUTE)?.split(" | ").map(declarations);
	const timing = { duration, easing: style.transitionTimingFunction.split(", ")[0] };
	return { entering: frames || (starting && [declarations(starting), {}]), timing, move: moving < 0 ? 0 : durations[moving % durations.length] };
}

// `opacity: 0; transform: scale(0.8)` as a keyframe
const declarations = css => Object.fromEntries(css.split(";").filter(declaration => declaration.includes(":")).map(declaration => {
	const [name, ...value] = declaration.split(":");
	return [name.trim().replace(/-(.)/g, (_, letter) => letter.toUpperCase()), value.join(":").trim()];
}));

function enter(node) {
	const transition = transitionOf(node);
	if (transition?.entering) node.animate(transition.entering, transition.timing);
}

// a node gone from the markup: with a starting style or keyframes it stays in place, unmatched, until it has animated
// out (through its keyframes backwards); then its moving siblings glide into its space (card web-leave-space)
function leave(node) {
	const transition = transitionOf(node);
	if (!transition?.entering) return node.remove();
	node.setAttribute(LEAVING_ATTRIBUTE, "");
	const removed = () => {
		const places = node.parentNode ? transitionPlaces(node.parentNode) : new Map();
		node.remove();
		places.forEach((before, element) => moveFrom(element, before));
	};
	node.animate([...transition.entering].reverse(), { ...transition.timing, fill: "forwards" }).finished.then(removed, removed);
}

// where each keyed element that moves with a transition is shown now: after the morph it glides from there (FLIP)
function transitionPlaces(shown) {
	return new Map([...shown.children].filter(child => isLive(child) && child.hasAttribute(KEY_ATTRIBUTE) && transitionOf(child)?.move).map(child => [child, child.getBoundingClientRect()]));
}

function moveFrom(element, before) {
	const transition = transitionOf(element);
	if (!element.isConnected || !isLive(element) || !transition?.move) return;
	const after = element.getBoundingClientRect();
	const [x, y] = [before.left - after.left, before.top - after.top];
	if (x || y) element.animate([{ transform: `translate(${x}px, ${y}px)` }, {}], { ...transition.timing, duration: transition.move });
}
