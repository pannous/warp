// pictures in the page (a part of host.js, which says how parts work): the host words paint and text_coverage
// (src/host.rs), with the int lists they cross in one call each; host-gpu.js builds its pixels with listOfInts and
// host-sound.js reads sound_samples with intsOfList

// free memory for `byteCount` bytes above the program's text heap, which its next text may take again (src/host.rs
// scratch, src/wasm_emitter/lists/int_lists.rs); undefined without a text heap
function scratch(program, byteCount) {
	const heap = program[TEXT_HEAP_EXPORT];
	if (!heap) return undefined;
	if (heap.value === 0) heap.value = program.memory.buffer.byteLength;
	const address = Math.ceil(heap.value / BigInt64Array.BYTES_PER_ELEMENT) * BigInt64Array.BYTES_PER_ELEMENT;
	const missing = address + byteCount - program.memory.buffer.byteLength;
	if (missing > 0) program.memory.grow(Math.ceil(missing / (1 << PAGE_BITS)));
	return address;
}

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

// u32s as a list of Ints, built in one call of the program's ints_to_list (gpu_render's pixels); undefined without it
function listOfInts(program, ints) {
	const address = program.ints_to_list && scratch(program, ints.byteLength);
	if (address === undefined) return undefined;
	new Uint32Array(program.memory.buffer, address, ints.length).set(ints);
	return program.ints_to_list(address, ints.length);
}

// the first `room` items of a list of Ints, read in one call of the program's list_to_ints (paint's pixels, sound's
// samples), their magnitudes as the low and high 32 bits of each (Int32Array): a view of the program's memory, valid
// until it runs on. Undefined for any other value or without list_to_ints, which the caller reads node by node
function intHalvesOfList(program, list, room) {
	const address = program.list_to_ints && scratch(program, room * BigInt64Array.BYTES_PER_ELEMENT);
	if (address === undefined) return undefined;
	const count = program.list_to_ints(list, address, room);
	return count < 0 ? undefined : new Int32Array(program.memory.buffer, address, 2 * count);
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
function paintFrame(holder, halves, width, height) {
	const last = holder.paintedFrame;
	const same = last?.width === width && last.height === height;
	const words = same ? last.words : new Int32Array(width * height);
	holder.paintedFrame = { width, height, words };
	if (!same) {
		for (let at = 0; at < width * height; at++) words[at] = shadeWord(halves[2 * at], halves[2 * at + 1]);
		return { x: 0, y: 0, columns: width, rows: height, words: words.slice() };
	}
	let [left, right, top, bottom] = [width, -1, height, -1];
	for (let row = 0, at = 0; row < height; row++) {
		for (let column = 0; column < width; column++, at++) {
			const word = shadeWord(halves[2 * at], halves[2 * at + 1]);
			if (word === words[at]) continue;
			words[at] = word;
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
			const covered = textCoverage(String(plainOfTree(readNode(program(), words))), Number(size));
			return listOfInts(program(), covered) ?? buildValue(program(), treeOfPlain(Array.from(covered)));
		},
		// paint(pixels, width, height) (src/host.rs): the page draws them on a canvas (playground.js showPaintings);
		// paint(shader, width, height, values) renders the WGSL fragment shader first, as gpu_render (P234), in the
		// playground straight into the page's canvas (host-gpu.js gpuPainted), which the hook gets by its id
		paint: (pixels, width, height, values) => {
			if (!hooks.paint) throw new Error("paint: no canvas here; it draws in the playground page");
			[width, height] = [Number(width), Number(height)];
			const halves = intHalvesOfList(program(), pixels, width * height);
			const painted = halves ?? plainOfTree(readNode(program(), pixels));
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
			const halves = intHalvesOfList(program(), pixels, width * height) ?? halvesOf(plainOfTree(readNode(program(), pixels)));
			const words = new Int32Array(width * height);
			for (let at = 0; at < words.length; at++) words[at] = shadeWord(halves[2 * at], halves[2 * at + 1]);
			hooks.savePicture(String(plainOfTree(readNode(program(), file))), words, width, height);
		},
	}),
});
