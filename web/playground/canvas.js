// The canvas of paint (src/host.rs paint, lib/draw.warp) and the pointer over it, for the playground (playground.js)
// and for a built site that paints (card site-frames, src/web/site.rs, site-thread.js)

// the gray levels of paint: a nonzero pixel, a zero pixel; from PAINT_COLOR_FROM on a value is a color 0xAARRGGBB
// (src/graphics/paint.rs shade, lib/draw.warp)
const PAINT_INK = 29;
const PAINT_PAPER = 250;
const PAINT_COLOR_FROM = 2 ** 24;
const PAINT_SHOWN_SIDE = 288; // a small painting is shown this wide (or high), scaled by a whole factor so pixels stay square
// mouse_x, mouse_y, mouse_down (host.js system_value): the pointer over a canvas in shared memory, which a running
// animation reads at once (its worker takes no message while it runs); the worker gets the buffer and these names
const POINTER_NAMES = ["mouse_x", "mouse_y", "mouse_down", "key"];
// a shader's $key (host-gpu.js builtinValues): the held key's code point, the arrows as a Mac's function keys
// the held key's code for key_down and held_key, as the native window gives it (src/graphics/paint_window.rs key_code)
const KEY_CODES = { ArrowUp: 0xF700, ArrowDown: 0xF701, ArrowLeft: 0xF702, ArrowRight: 0xF703, Enter: 13, Escape: 27, Backspace: 8, Tab: 9 };
const TYPING_TARGETS = ".CodeMirror, input, textarea, [contenteditable]";
const pointer = globalThis.SharedArrayBuffer ? new Int32Array(new SharedArrayBuffer(POINTER_NAMES.length * Int32Array.BYTES_PER_ELEMENT)) : undefined;

// paint(pixels, width, height): one canvas per call, a pixel dark where its value is nonzero (true), light where 0
// what a pixel value shows, as src/graphics/paint.rs shade: paper for 0, its color for a value with an alpha byte, else ink
// as one opaque pixel of ImageData's bytes R, G, B, A read as a little-endian 32-bit word
const OPAQUE_WORD = 0xFF000000;
const grayWord = level => (OPAQUE_WORD | level << 16 | level << 8 | level) >>> 0;
const PAPER_WORD = grayWord(PAINT_PAPER);
const INK_WORD = grayWord(PAINT_INK);
function paintShade(value) {
	if (!value) return PAPER_WORD;
	const number = Number(value);
	if (number < PAINT_COLOR_FROM) return INK_WORD;
	return (OPAQUE_WORD | (number % 256) << 16 | (Math.floor(number / 256) % 256) << 8 | Math.floor(number / 65536) % 256) >>> 0;
}

// a canvas of the painting's size, shown scaled up by a whole factor; the GPU's own canvas of a painted shader as it is
const paintingCanvas = painting => painting.canvas ?? drawn(paintingElement(painting), painting);

function paintingElement(painting) {
	const canvas = document.createElement("canvas");
	Object.assign(canvas, { width: painting.width, height: painting.height, className: "painting" });
	canvas.style.width = `${painting.width * Math.max(1, Math.floor(PAINT_SHOWN_SIDE / Math.max(painting.width, painting.height, 1)))}px`;
	canvas.style.imageRendering = "pixelated";
	canvas.style.setProperty("--aspect", painting.width / Math.max(painting.height, 1));
	return canvas;
}

const sameSize = (one, other) => one.width === other.width && one.height === other.height;

function drawn(canvas, { pixels, width, height }) {
	const image = canvas.getContext("2d").createImageData(width, height);
	// a word per pixel, no array per pixel: a 1280×800 frame of samples/finger_paint took Safari far too long (card abysmally-slow)
	const words = new Uint32Array(image.data.buffer);
	for (let index = 0; index < width * height; index++) words[index] = paintShade(pixels[index]);
	canvas.getContext("2d").putImageData(image, 0, 0);
	return canvas;
}

// the pointer message for the program's worker (worker.js, site-worker.js: self.pagePointer)
const pointerMessage = () => pointer && { pointer: { buffer: pointer.buffer, names: POINTER_NAMES } };

// pointer events over a canvas store where the pointer is, in the canvas's pixels, and whether it is down
function trackPointer(event) {
	const canvas = event.target.closest?.("canvas");
	if (!pointer || !canvas) return;
	const { x, y } = pointOn(canvas, event);
	[x, y, event.buttons & 1].forEach((value, index) => Atomics.store(pointer, index, value));
}

const releasePointer = () => pointer && Atomics.store(pointer, POINTER_NAMES.indexOf("mouse_down"), 0);

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
	document.addEventListener("pointerup", releasePointer);
	for (const event of ["keydown", "keyup"]) document.addEventListener(event, trackKey);
}
