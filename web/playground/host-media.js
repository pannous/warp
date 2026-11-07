// pictures in the page (a part of host.js, which says how parts work): the host words paint and text_coverage
// (src/host.rs), with the int lists they cross (values.js newGuestInts, guestInts); host-sound.js reads sound_samples
// with intsOfList

// [width, height, then how much each pixel is covered from 0 to 255, row by row] of the words set in a sans-serif font
// `size` pixels per em on one line, as src/graphics/text_raster.rs does natively: an OffscreenCanvas works in the run's Worker too
const TEXT_FONT = "sans-serif";
function textCoverage(words, size) {
	const font = `${size}px ${TEXT_FONT}`;
	const measured = new OffscreenCanvas(1, 1).getContext("2d");
	measured.font = font;
	const metrics = measured.measureText(words);
	const width = Math.max(1, Math.ceil(metrics.width));
	const height = Math.max(1, Math.ceil(metrics.fontBoundingBoxAscent + metrics.fontBoundingBoxDescent));
	const context = new OffscreenCanvas(width, height).getContext("2d");
	context.font = font;
	context.fillText(words, 0, metrics.fontBoundingBoxAscent);
	const pixels = context.getImageData(0, 0, width, height).data;
	const covered = new Uint32Array(2 + width * height);
	covered.set([width, height]);
	for (let pixel = 0; pixel < width * height; pixel++) covered[2 + pixel] = pixels[pixel * 4 + 3];
	return covered;
}

// the first `room` items of a list of Ints (paint's pixels, sound's samples), their magnitudes as the low and high 32
// bits of each (Int32Array); undefined for a text, which the caller reads as a shader
const intHalvesOfList = (program, list, room) => isGuestText(program, list) ? undefined : halvesOf(guestInts(program, list, Math.min(room, guestCount(program, list))));
// a picture's pixels as intHalvesOfList gives them, read a chunk at a time by the module's pixels_at (4 bytes a pixel in
// its memory), as the native warp reads them: one call a pixel took 120 to 280 ms a frame of 1280×800 (finger_paint)
const PIXEL_CHUNK = 16372; // core/runtime/show.warp pixel_chunk(), as core/native/driver.warp PIXEL_CHUNK
function pictureHalves(program, picture, room) {
	if (!program.pixels_at || isGuestText(program, picture)) return intHalvesOfList(program, picture, room);
	const count = Math.min(room, guestCount(program, picture));
	const halves = new Int32Array(2 * count);
	for (let from = 0; from < count; from += PIXEL_CHUNK) {
		const chunk = Math.min(PIXEL_CHUNK, count - from);
		const address = Number(program.pixels_at(picture, BigInt(from + 1), BigInt(chunk)));
		const words = new Int32Array(program.memory.buffer.slice(address, address + 4 * chunk));
		for (let at = 0; at < chunk; at++) halves[2 * (from + at)] = words[at];
	}
	return halves;
}
const intsOfList = (program, list, room) => {
	const halves = intHalvesOfList(program, list, room);
	return halves && Array.from(new BigInt64Array(halves.buffer, halves.byteOffset, halves.length / 2), Number);
};

// the halves of plain pixel values (yes/no, numbers, a shader's u32 colors), as intHalvesOfList gives them
function halvesOf(values) {
	const halves = new Int32Array(2 * values.length);
	for (let index = 0; index < values.length; index++) {
		const value = Number(values[index]);
		halves[2 * index] = value % 2 ** 32;
		halves[2 * index + 1] = Math.floor(value / 2 ** 32);
	}
	return halves;
}

// the gray levels of paint: a nonzero pixel, a zero pixel; from PAINT_COLOR_FROM on a value is a color 0xAARRGGBB
// (src/graphics/pixels.rs, which tests/programs/test_paint_shades.rs holds these to; lib/draw.warp). A pixel shown is
// one opaque word of ImageData's R, G, B, A bytes read little-endian, a signed 32-bit int like the Int32Array it is kept in
const PAINT_INK = 29;
const PAINT_PAPER = 250;
const PAINT_COLOR_FROM = 2 ** 24;
const OPAQUE_WORD = 0xFF000000;
const grayWord = level => OPAQUE_WORD | level << 16 | level << 8 | level;
const PAPER_WORD = grayWord(PAINT_PAPER);
const INK_WORD = grayWord(PAINT_INK);
function shadeWord(low, high) {
	if (high === 0 && low >>> 0 < PAINT_COLOR_FROM) return low === 0 ? PAPER_WORD : INK_WORD;
	return OPAQUE_WORD | (low & 0xFF) << 16 | low & 0xFF00 | low >>> 16 & 0xFF;
}

// a paint call as the page draws it (canvas.js wholePainting): its pixels shown as words, and when the painting before
// had the same size only the rectangle that changed since, often empty. A stroke of samples/graphics/finger_paint changes a few
// hundred of 1280×800 pixels: the whole frame shaded as Numbers, cloned to the page and shaded there again took
// hundreds of milliseconds each in Safari (card abysmally-slow). {x, y, columns, rows, words}, words row by row
const paintFrame = (holder, halves, width, height) => changedFrame(holder, shadedWords(halves, width * height), width, height);
function shadedWords(halves, count) {
	const words = new Int32Array(count);
	for (let at = 0; at < count; at++) words[at] = shadeWord(halves[2 * at], halves[2 * at + 1]);
	return words;
}
// the frame of the words shown (a painting's, a window's) against the frame before: whole when the size changed, else
// the rectangle that did
function changedFrame(holder, words, width, height) {
	const last = holder.paintedFrame;
	holder.paintedFrame = { width, height, words };
	if (last?.width !== width || last.height !== height) return { x: 0, y: 0, columns: width, rows: height, words: words.slice() };
	let [left, right, top, bottom] = [width, -1, height, -1];
	for (let row = 0, at = 0; row < height; row++) {
		for (let column = 0; column < width; column++, at++) {
			if (words[at] === last.words[at]) continue;
			left = Math.min(left, column);
			right = Math.max(right, column);
			top = Math.min(top, row);
			bottom = row;
		}
	}
	const [columns, rows] = [Math.max(right - left + 1, 0), Math.max(bottom - top + 1, 0)];
	const changed = new Int32Array(columns * rows);
	for (let row = 0; row < rows; row++) changed.set(words.subarray((top + row) * width + left, (top + row) * width + left + columns), row * columns);
	return { x: Math.min(left, width), y: Math.min(top, height), columns, rows, words: changed };
}

addHostPart({
	words: (holder, hooks, { program }) => ({
		// text_coverage(words, size) (label of use draw, card paint-text): the words set by the browser's canvas
		text_coverage: (words, size) => {
			const covered = textCoverage(String(guestPlain(program(), words)), Number(size));
			return newGuestInts(program(), covered);
		},
		// paint(pixels, width, height) (src/host.rs): the page draws them on a canvas (playground.js showPaintings);
		// paint(shader, width, height, values) renders the WGSL fragment shader first, as gpu_render (P234), in the
		// playground straight into the page's canvas (host-gpu.js gpuPainted), which the hook gets by its id
		paint: (pixels, width, height, values) => {
			if (!hooks.paint) throw new Error("paint: no canvas here; it draws in the playground page");
			[width, height] = [Number(width), Number(height)];
			const halves = pictureHalves(program(), pixels, width * height);
			const painted = halves ?? guestPlain(program(), pixels);
			if (typeof painted === "string") {
				if (!holder.gpuRendered) throw new Error("paint: a shader needs WebGPU (host-gpu.js), which this page has not");
				const rendered = (holder.gpuPainted ?? holder.gpuRendered)(painted, width, height, values); // pixels, or the id of a canvas drawn into
				holder.paintedFrame = undefined;
				if (typeof rendered === "number") return hooks.paint(undefined, width, height, rendered);
				return hooks.paint(paintFrame(holder, halvesOf(rendered), width, height), width, height, undefined, true);
			}
			hooks.paint(paintFrame(holder, halves ?? halvesOf(painted), width, height), width, height);
		},
		// save_picture(pixels, width, height, file) (src/host.rs, card save-painting): use draw's canvas, shaded as paint
		// shows it, which the page makes a PNG of and downloads (playground.js savePicture)
		save_picture: (pixels, width, height, file) => {
			if (!hooks.savePicture) throw new Error("save: no page here to download the picture to");
			[width, height] = [Number(width), Number(height)];
			const halves = pictureHalves(program(), pixels, width * height) ?? halvesOf(guestPlain(program(), pixels));
			hooks.savePicture(String(guestPlain(program(), file)), shadedWords(halves, width * height), width, height);
		},
	}),
});

// A C window library's calls (lib/native/raylib.warp, lib/native/SDL2.warp; natively libraylib and libSDL2 open a
// window) drawn in the page: the window is an OffscreenCanvas in the run's Worker, and each finished frame (EndDrawing,
// SDL_RenderPresent) a painting, as paint's (changedFrame). A raylib color is one i32, 0xAABBGGRR, the bytes R, G, B, A
// that are an ImageData word; the mouse and keys are the page's shared pointer (canvas.js), the frame rate the run's
// own sleep, so the playground shows each frame as an animation's
const WINDOW_FONT = TEXT_FONT;
const RAYLIB_EXIT_KEY = 256; // KEY_ESCAPE: WindowShouldClose turns yes while it is held, as natively
// raylib's key codes for the keys canvas.js KEY_CODES names; a letter is its capital (KEY_A 65), held as typed
const RAYLIB_PAGE_KEYS = { 256: 27, 257: 13, 258: 9, 259: 8, 262: 0xF703, 263: 0xF702, 264: 0xF701, 265: 0xF700 };
const RAYLIB_LEFT_BUTTON = 0;
const SDL_WINDOW = 1, SDL_RENDERER = 2; // the handles of the one window and its renderer
const rgbaOf = color => `rgba(${color & 0xFF}, ${color >> 8 & 0xFF}, ${color >> 16 & 0xFF}, ${(color >>> 24) / 255})`;
const pointerValue = name => self.pagePointer ? Atomics.load(self.pagePointer.values, self.pagePointer.names.indexOf(name)) : 0;
const heldKeys = () => (self.pagePointer?.names ?? []).filter(name => name === "key" || name.startsWith("held ")).map(pointerValue);
const pageKeysOf = key => RAYLIB_PAGE_KEYS[key] !== undefined ? [RAYLIB_PAGE_KEYS[key]] : [key, String.fromCodePoint(key).toLowerCase().codePointAt(0)];

// a window opened shows at once, black until the program draws, as a native one does
const WINDOW_GROUND = "black";
function windowOpened(holder, hooks, width, height) {
	const canvas = new OffscreenCanvas(Number(width), Number(height));
	const context = canvas.getContext("2d", { willReadFrequently: true });
	context.fillStyle = WINDOW_GROUND;
	context.fillRect(0, 0, canvas.width, canvas.height);
	holder.window = { canvas, context, opened: performance.now(), frameStarted: performance.now(), frameSeconds: 0, frameMilliseconds: 0, pressedSeen: pointerValue("keys pressed") };
	holder.paintedFrame = undefined;
	windowShown(holder, hooks);
}
function openWindow(holder) {
	if (!holder.window) throw new Error("no window: InitWindow (or SDL_CreateWindow) opens it first");
	return holder.window;
}
// the frame drawn so far shown as a painting, then the run waits out the rest of the frame's time
function windowShown(holder, hooks) {
	const { canvas, context } = openWindow(holder);
	if (!hooks.paint) throw new Error("a window draws in the playground page, not here");
	const pixels = context.getImageData(0, 0, canvas.width, canvas.height).data;
	hooks.paint(changedFrame(holder, new Int32Array(pixels.buffer), canvas.width, canvas.height), canvas.width, canvas.height);
	const window = holder.window;
	const rest = window.frameMilliseconds - (performance.now() - window.frameStarted);
	if (rest > 0) holder.sleep(rest);
	window.frameSeconds = (performance.now() - window.frameStarted) / 1000;
	window.frameStarted = performance.now();
	window.pressedSeen = pointerValue("keys pressed");
}
function filled(holder, color, draw) {
	const { context } = openWindow(holder);
	context.fillStyle = context.strokeStyle = rgbaOf(Number(color));
	draw(context);
	return 0;
}
const circle = (context, x, y, radius) => { context.beginPath(); context.arc(Number(x), Number(y), Number(radius), 0, 2 * Math.PI); };
const line = (context, x1, y1, x2, y2) => { context.beginPath(); context.moveTo(Number(x1) + 0.5, Number(y1) + 0.5); context.lineTo(Number(x2) + 0.5, Number(y2) + 0.5); context.stroke(); };
const fontOf = size => `${Number(size)}px ${WINDOW_FONT}`;
// the raylib code of a key the page holds by its code: a named key's, a letter's capital; 0 of none
const raylibKeyOf = code => code === 0 ? 0 : Number(Object.keys(RAYLIB_PAGE_KEYS).find(key => RAYLIB_PAGE_KEYS[key] === code) ?? String.fromCodePoint(code).toUpperCase().codePointAt(0));
const keyPressed = holder => holder.window && pointerValue("keys pressed") !== holder.window.pressedSeen ? pointerValue("key code pressed") : 0;

const raylibWords = (holder, hooks) => ({
	InitWindow: (width, height) => { windowOpened(holder, hooks, width, height); },
	CloseWindow: () => { holder.window = undefined; },
	WindowShouldClose: () => heldKeys().includes(RAYLIB_PAGE_KEYS[RAYLIB_EXIT_KEY]),
	IsWindowReady: () => holder.window !== undefined,
	SetTargetFPS: fps => { openWindow(holder).frameMilliseconds = Number(fps) > 0 ? 1000 / Number(fps) : 0; },
	SetTraceLogLevel: () => {},
	GetFrameTime: () => openWindow(holder).frameSeconds,
	GetTime: () => (performance.now() - openWindow(holder).opened) / 1000,
	GetFPS: () => Math.round(1 / Math.max(openWindow(holder).frameSeconds, 1e-3)),
	GetScreenWidth: () => openWindow(holder).canvas.width,
	GetScreenHeight: () => openWindow(holder).canvas.height,
	BeginDrawing: () => { openWindow(holder); },
	EndDrawing: () => windowShown(holder, hooks),
	ClearBackground: color => filled(holder, color, context => context.fillRect(0, 0, context.canvas.width, context.canvas.height)),
	DrawPixel: (x, y, color) => filled(holder, color, context => context.fillRect(Number(x), Number(y), 1, 1)),
	DrawLine: (x1, y1, x2, y2, color) => filled(holder, color, context => line(context, x1, y1, x2, y2)),
	DrawCircle: (x, y, radius, color) => filled(holder, color, context => { circle(context, x, y, radius); context.fill(); }),
	DrawCircleLines: (x, y, radius, color) => filled(holder, color, context => { circle(context, x, y, radius); context.stroke(); }),
	DrawRectangle: (x, y, width, height, color) => filled(holder, color, context => context.fillRect(Number(x), Number(y), Number(width), Number(height))),
	DrawRectangleLines: (x, y, width, height, color) => filled(holder, color, context => context.strokeRect(Number(x) + 0.5, Number(y) + 0.5, Number(width) - 1, Number(height) - 1)),
	DrawText: (text, x, y, size, color) => filled(holder, color, context => { context.font = fontOf(size); context.textBaseline = "top"; context.fillText(text, Number(x), Number(y)); }),
	DrawFPS: (x, y) => filled(holder, 0xFF30E000, context => { context.font = fontOf(20); context.textBaseline = "top"; context.fillText(`${raylibWords(holder, hooks).GetFPS()} FPS`, Number(x), Number(y)); }),
	MeasureText: (text, size) => { const { context } = openWindow(holder); context.font = fontOf(size); return Math.round(context.measureText(text).width); },
	TextToInteger: text => parseInt(text, 10) || 0,
	GetRandomValue: (min, max) => Number(min) + Math.floor(Math.random() * (Number(max) - Number(min) + 1)),
	GetMouseX: () => pointerValue("mouse_x"),
	GetMouseY: () => pointerValue("mouse_y"),
	SetMouseOffset: () => {},
	IsMouseButtonDown: button => Number(button) === RAYLIB_LEFT_BUTTON && pointerValue("mouse_down") !== 0,
	IsKeyDown: key => pageKeysOf(Number(key)).some(code => heldKeys().includes(code)),
	IsKeyPressed: key => pageKeysOf(Number(key)).includes(keyPressed(holder)),
	GetKeyPressed: () => raylibKeyOf(keyPressed(holder)),
});

const sdlWords = (holder, hooks) => ({
	SDL_Init: () => { holder.sdlStarted = performance.now(); return 0; },
	SDL_Quit: () => { holder.window = undefined; },
	SDL_GetError: () => "",
	SDL_GetTicks: () => Math.round(performance.now() - (holder.sdlStarted ?? 0)),
	SDL_Delay: milliseconds => { holder.sleep(Number(milliseconds)); },
	SDL_CreateWindow: (title, x, y, width, height) => { windowOpened(holder, hooks, width, height); return SDL_WINDOW; },
	SDL_DestroyWindow: () => { holder.window = undefined; },
	SDL_CreateRenderer: () => { openWindow(holder).color = 0xFF000000; return SDL_RENDERER; },
	SDL_DestroyRenderer: () => {},
	SDL_SetRenderDrawColor: (renderer, red, green, blue, alpha) => { openWindow(holder).color = (Number(alpha) << 24 | Number(blue) << 16 | Number(green) << 8 | Number(red)) >>> 0; return 0; },
	SDL_RenderClear: () => filled(holder, openWindow(holder).color, context => context.fillRect(0, 0, context.canvas.width, context.canvas.height)),
	SDL_RenderDrawLine: (renderer, x1, y1, x2, y2) => filled(holder, openWindow(holder).color, context => line(context, x1, y1, x2, y2)),
	SDL_RenderDrawPoint: (renderer, x, y) => filled(holder, openWindow(holder).color, context => context.fillRect(Number(x), Number(y), 1, 1)),
	SDL_RenderPresent: () => windowShown(holder, hooks),
});

addHostPart({
	imports: (holder, hooks) => {
		const [raylib, sdl] = [raylibWords(holder, hooks), sdlWords(holder, hooks)];
		return { raylib: libraryInJs(holder, "raylib", name => raylib[name]), SDL2: libraryInJs(holder, "SDL2", name => sdl[name]) };
	},
});
