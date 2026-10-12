// pictures and sound in the page (a part of host.js, which says how parts work): the host words paint, text_coverage and
// sound_samples (src/host.rs), with the int lists they cross in one call each; host-gpu.js builds its pixels with listOfInts

// free memory for `byteCount` bytes above the program's text heap, which its next text may take again (src/host.rs
// scratch, src/wasm_emitter/int_lists.rs); undefined without a text heap
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
// `size` pixels per em on one line, as src/text_raster.rs does natively: an OffscreenCanvas works in the run's Worker too
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

// the first `room` items of a list of Ints, read in one call of the program's list_to_ints (paint's pixels, their
// magnitudes); undefined for any other value or without it, which the caller reads node by node
function intsOfList(program, list, room) {
	const address = program.list_to_ints && scratch(program, room * BigInt64Array.BYTES_PER_ELEMENT);
	if (address === undefined) return undefined;
	const count = program.list_to_ints(list, address, room);
	return count < 0 ? undefined : Array.from(new BigInt64Array(program.memory.buffer, address, count), Number);
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
			const room = Number(width) * Number(height);
			let painted = intsOfList(program(), pixels, room) ?? plainOfTree(readNode(program(), pixels));
			if (typeof painted === "string") {
				if (!holder.gpuRendered) throw new Error("paint: a shader needs WebGPU (host-gpu.js), which this page has not");
				const rendered = (holder.gpuPainted ?? holder.gpuRendered)(painted, width, height, values); // pixels, or the id of a canvas drawn into
				if (typeof rendered === "number") return hooks.paint(undefined, Number(width), Number(height), rendered);
				painted = Array.from(rendered);
			}
			hooks.paint(painted, Number(width), Number(height));
		},
		// sound_samples(samples, count, rate, channels) (src/host.rs, lib/sound.warp; stereo interleaved left, right):
		// the page plays them with WebAudio
		// (playground.js playSound); a run without a page (tests, node) has no speakers and stays silent, as natively
		// render_sound writes them, played or not, into a WAV (host-files.js STD_ADAPTERS.sound.render)
		sound_samples: (samples, count, rate, channels) => {
			const values = intsOfList(program(), samples, Number(count)) ?? plainOfTree(readNode(program(), samples));
			const sound = { samples: values, rate: Number(rate), channels: Number(channels) };
			const { place, at } = self.voiceSounded?.(values.length / sound.channels / sound.rate) ?? {};
			(holder.run.unrenderedSounds ??= []).push({ ...sound, place });
			hooks.sound?.({ ...sound, at });
		},
	}),
});
