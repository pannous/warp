// The canvas of paint (src/host.rs paint, lib/draw.warp) and the pointer over it, for the playground (playground.js)
// and for a built site that paints (card site-frames, src/web/site.rs, site-thread.js)

const PAINT_SHOWN_SIDE = 288; // a small painting is shown this wide (or high), scaled by a whole factor so pixels stay square
// mouse_x, mouse_y, mouse_down (host.js system_value): the pointer over a canvas in shared memory, which a running
// animation reads at once (its worker takes no message while it runs); the worker gets the buffer and these names
const POINTER_NAMES = ["mouse_x", "mouse_y", "mouse_down", "key"];
// a shader's $key (host-gpu.js builtinValues): the held key's code point, the arrows as a Mac's function keys
// the held key's code for key_down and held_key, as the native window gives it (src/graphics/paint_window.rs key_code)
const KEY_CODES = { ArrowUp: 0xF700, ArrowDown: 0xF701, ArrowLeft: 0xF702, ArrowRight: 0xF703, Enter: 13, Escape: 27, Backspace: 8, Tab: 9 };
const TYPING_TARGETS = ".CodeMirror, input, textarea, [contenteditable]";
// window.width = 800 (host.js set_system_value): the size in CSS pixels the run's paintings show at, scaled to fit
let paintingWindow;
const pointer = globalThis.SharedArrayBuffer ? new Int32Array(new SharedArrayBuffer(POINTER_NAMES.length * Int32Array.BYTES_PER_ELEMENT)) : undefined;

// paint(pixels, width, height): one canvas per call, its pixels shaded by the program's Worker (host-media.js
// paintFrame), which sends of a painting the size of the one before only the rectangle that changed. The painting it
// changes is the last one the page got: the whole of each painting is kept here, as an ImageData over its words
let lastPainting;
function wholePainting(painting) {
	const { frame, width, height } = painting;
	if (!frame) return painting; // a canvas the GPU draws into
	const whole = frame.columns === width && frame.rows === height;
	const words = whole ? frame.words : lastPainting.words;
	for (let row = 0; !whole && row < frame.rows; row++) words.set(frame.words.subarray(row * frame.columns, (row + 1) * frame.columns), (frame.y + row) * width + frame.x);
	const image = whole ? new ImageData(new Uint8ClampedArray(words.buffer), width, height) : lastPainting.image;
	return lastPainting = { ...painting, words, image };
}

// a canvas of the painting's size, shown scaled up by a whole factor; the GPU's own canvas of a painted shader as it is
const paintingCanvas = painting => painting.canvas ?? drawnWhole(paintingElement(painting), painting);

function paintingElement(painting) {
	const canvas = document.createElement("canvas");
	Object.assign(canvas, { width: painting.width, height: painting.height, className: "painting" });
	canvas.style.width = `${painting.width * Math.max(1, Math.floor(PAINT_SHOWN_SIDE / Math.max(painting.width, painting.height, 1)))}px`;
	// pixels painted one by one stay sharp squares; a shader's render scales smoothly (card make-shader)
	canvas.style.imageRendering = painting.shaded ? "auto" : "pixelated";
	canvas.style.setProperty("--aspect", painting.width / Math.max(painting.height, 1));
	// a finger on it paints instead of scrolling, zooming or selecting the page: else Safari takes the touch over and
	// cancels its pointer events after the first (card register-fingers)
	Object.assign(canvas.style, { touchAction: "none", webkitUserSelect: "none", userSelect: "none" });
	framed(canvas);
	return canvas;
}

// a painting in the window the run set, if it set one
function framed(canvas) {
	if (!paintingWindow) return;
	canvas.style.width = `${paintingWindow.width}px`;
	canvas.style.height = `${paintingWindow.height}px`;
}

// window.width = …: the paintings shown take that size, and the ones to come
function resizeWindow({ width, height }, paintings) {
	paintingWindow = { width, height };
	paintings.querySelectorAll("canvas").forEach(framed);
}

const sameSize = (one, other) => one.width === other.width && one.height === other.height;

// a painting drawn into a canvas that shows the one before: what changed, at the screen's pace. An animation's frames may
// come quicker than the screen shows them; their rectangles are joined and drawn once per screen frame
function drawn(canvas, painting) {
	if (!painting.image) return canvas;
	canvas.waiting = joined(canvas.waiting, painting.frame);
	canvas.waitingImage = painting.image;
	if (!canvas.drawScheduled) requestAnimationFrame(() => drawWaiting(canvas));
	canvas.drawScheduled = true;
	return canvas;
}

function drawWaiting(canvas) {
	const { x, y, columns, rows } = canvas.waiting;
	canvas.drawScheduled = false;
	canvas.waiting = undefined;
	if (columns && rows) canvas.getContext("2d").putImageData(canvas.waitingImage, 0, 0, x, y, columns, rows);
}

// the rectangle around two (an empty one has no columns or rows)
function joined(one, other) {
	if (!one?.columns || !one.rows) return other;
	if (!other.columns || !other.rows) return one;
	const [x, y] = [Math.min(one.x, other.x), Math.min(one.y, other.y)];
	const right = Math.max(one.x + one.columns, other.x + other.columns), bottom = Math.max(one.y + one.rows, other.y + other.rows);
	return { x, y, columns: right - x, rows: bottom - y };
}

// a new canvas, drawn whole at once
function drawnWhole(canvas, painting) {
	canvas.getContext("2d").putImageData(painting.image, 0, 0);
	return canvas;
}

// the pointer message for the program's worker (worker.js, site-worker.js: self.pagePointer)
const pointerMessage = () => pointer && { pointer: { buffer: pointer.buffer, names: POINTER_NAMES } };

const laterFinger = event => event.pointerType === "touch" && !event.isPrimary;

// pointer events over a canvas store where the pointer is, in the canvas's pixels, and whether it is down; of several
// fingers the first one down, so a second finger doesn't make the stroke jump between them
function trackPointer(event) {
	const canvas = event.target.closest?.("canvas");
	if (!pointer || !canvas || laterFinger(event)) return;
	const { x, y } = pointOn(canvas, event);
	[x, y, event.buttons & 1].forEach((value, index) => Atomics.store(pointer, index, value));
}

// the button or first finger lifted, or the touch taken over by the browser (pointercancel)
const releasePointer = event => pointer && !laterFinger(event) && Atomics.store(pointer, POINTER_NAMES.indexOf("mouse_down"), 0);

// a key held outside the editor and the fields, its code while down
function trackKey(event) {
	const code = KEY_CODES[event.key] ?? ([...event.key].length === 1 ? event.key.codePointAt(0) : undefined);
	if (!pointer || code === undefined || event.target.closest?.(TYPING_TARGETS)) return;
	Atomics.store(pointer, POINTER_NAMES.indexOf("key"), event.type === "keydown" ? code : 0);
}

// where the event is on the element, in a canvas's own pixels
function pointOn(target, event) {
	const bounds = target.getBoundingClientRect();
	const scale = target.width ? target.width / bounds.width : 1;
	return { x: Math.floor((event.clientX - bounds.left) * scale), y: Math.floor((event.clientY - bounds.top) * scale) };
}

// the pointer tracked over the canvases inside `element`, released anywhere
function followPointer(element) {
	for (const event of ["pointermove", "pointerdown", "pointerup"]) element.addEventListener(event, trackPointer);
	for (const event of ["pointerup", "pointercancel"]) document.addEventListener(event, releasePointer);
	for (const event of ["keydown", "keyup"]) document.addEventListener(event, trackKey);
}
