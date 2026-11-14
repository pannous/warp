// WebGPU (a part of host.js, which says how parts work; card web-apis, notes/web_framework.md "web-apis: WebGPU"):
// gpu_compute(shader, numbers, workgroups) runs a WGSL compute shader whose entry point `main` reads and writes the
// numbers as array<f32> at @group(0) @binding(0), dispatched over `workgroups` workgroups, and gives back the numbers it
// left; given a map of named arrays, it binds each where the shader declares the storage array of that name and gives
// them back by name. gpu_render(shader, width, height, values) runs the fragment shader `main` over every pixel of the image and gives
// the pixels as paint takes them, 0xFFRRGGBB row by row; the shader reads the map `values` as a uniform,
// `values.<name>` (src/graphics/gpu.rs does both natively). WebGPU only answers
// asynchronously, so a task Worker (host-tasks.js) does the GPU work while the program's worker waits for its answer in
// shared memory, as a task's result. In the playground, paint of a shader renders straight into the page's canvas: the
// page hands the canvas's OffscreenCanvas to the task Worker, which draws through its GPUCanvasContext, and no pixels
// come back (card web-apis-rest).

const GPU_ENTRY_POINT = "main";
const NO_GPU_DRAWING = "use draw draws through pixels on a page (gpu_drawing is no here), so draw_shape, draw_show and draw_shader never run";
const GPU_BINDING = 0;
// what a shader that does not compile is said as, natively too (lib/host/native_host/gpu.warp shader_failure)
const SHADER_SUBJECT = "the shader: ";
// `@group(0) @binding(1) var<storage, read_write> ys: array<f32>`: its binding, name and element type (src/graphics/gpu.rs STORAGE_ARRAY)
const STORAGE_ARRAY = /@binding\((\d+)\)(?:\s*@group\(\d+\))?\s*var(?:<\s*storage\s*(?:,\s*(\w+)\s*)?>)?\s+(\w+)\s*:\s*array<(\w+)/g;
// an array<f16>'s cells as their raw bits: it takes only a GPU list [f16], whose cells are bound as they are
const ELEMENT_ARRAYS = { i32: Int32Array, u32: Uint32Array, f16: Uint16Array };
const HALF_FEATURE = "shader-f16";
const HALVES_ENABLED = "enable f16";
// said where the GPU lacks it, as natively (lib/host/native_host/gpu.warp NO_HALVES)
const NO_HALVES = "this GPU offers no f16 (WebGPU's shader-f16), which the shader's enable f16 needs";
const WORD_BYTES = 4; // WebGPU writes, copies and binds buffers in whole 4-byte words
// appended to gpu_render's shader (after it, so its line numbers stay the user's): one triangle covering the image
const FULL_IMAGE_VERTICES = `
@vertex fn warp_full_image(@builtin(vertex_index) corner: u32) -> @builtin(position) vec4f {
	return vec4f(f32(corner / 2u) * 4.0 - 1.0, f32(corner % 2u) * 4.0 - 1.0, 0.0, 1.0);
}`;
const VERTEX_ENTRY_POINT = "warp_full_image";
const TRIANGLE_CORNERS = 3;
// a shader with a vertex stage of its own (card gpu-instanced-blend) draws `values.instances` quads, each its 4 corners
// (vertex_index 0 to 3) as a triangle strip, blended over what the quads before drew (instance_index 0 first), as
// lib/host/native_host/gpu.warp draws them
const OWN_VERTEX_STAGE = /@vertex\s+fn\s+(\w+)/;
const QUAD_CORNERS = 4;
const INSTANCES_NAME = "instances";
// values.indices is a uint32 index buffer, not a storage array. values.depth attaches depth32float (lib/host/native_host/gpu.warp)
const INDICES_NAME = "indices";
const DEPTH_NAME = "depth";
const INDIRECT_NAME = "indirect";
const COMPUTE_NAME = "compute";
const WORKGROUPS_NAME = "workgroups";
// values.compute's passes: each `@compute … fn name`, dispatched in source order before the draw
const COMPUTE_ENTRY = /@compute\b[^{]*?\bfn\s+(\w+)/g;
// a field (wiki graphics.md "Fields and passes"): a compute shader's fixed-length read_write array that no value names,
// `var<storage, read_write> particles: array<vec4f, 4096>`, zeroed on the first frame and kept across the program's
// frames; the draw reads it as values.particles
const FIELD_ARRAY = /var<\s*storage\s*,\s*read_write\s*>\s+(\w+)\s*:\s*(array<\s*(.+?)\s*,\s*(\d+)\s*>)\s*;/g;
// an array element's stride in bytes, by its WGSL type
const ELEMENT_STRIDES = [[/^(f32|i32|u32|atomic<[iu]32>)$/, 4], [/^vec2(f|i|u|<[fiu]32>)$/, 8], [/^vec[34](f|i|u|<[fiu]32>)$/, 16]];
const fields = new Map(); // the task Worker's: program canvas (or shader) + field declaration → its buffer
const MOST_STORAGE_ARRAYS = 12;
const MOST_BOUND_ARRAYS = 8;
const DEPTH_FORMAT = "depth32float";
// a quad's color over the ones drawn before it: source alpha, as `vec4f(color, alpha)` says
const OVER = { color: { srcFactor: "src-alpha", dstFactor: "one-minus-src-alpha" }, alpha: { srcFactor: "one", dstFactor: "one-minus-src-alpha" } };
const PIXEL_BYTES = 4;
const ROW_ALIGNMENT = 256; // a copied row is padded to 256 bytes
const OPAQUE = 0xFF000000;
// what a painting shows before a shader colored it: the paper of lib/draw.warp (host-media.js PAINT_PAPER, in dark mode
// PAINT_INK), not black
const paperColor = () => { const shade = (darkTheme() ? PAINT_INK : PAINT_PAPER) / 255; return { r: shade, g: shade, b: shade, a: 1 }; };
const VALUES_NAME = "values"; // gpu_render's values as the shader names them
const VALUES_BINDING = 0;
// `$previous(at)` (card shader-read): the color the shader left at pixel `at` in its previous frame of this size, black
// before the first, kept on the GPU (src/graphics/gpu.rs PREVIOUS_FRAME)
const PREVIOUS_HOLE = "previous";
const PREVIOUS_FUNCTION = "warp_previous";
const PREVIOUS_FRAME_BINDING = 1;
// paint's lists of more than 4 numbers or of vectors: each a read-only storage array of its own from this binding on, as
// long as it is (card paint-values-storage), `values.mesh` in the shader the array warp_values_mesh
// (lib/host/native_host/gpu.warp STORAGE_BINDING)
const STORAGE_BINDING = 2;
const STORAGE_PREFIX = "warp_values_";
const PREVIOUS_FRAME = `
@group(0) @binding(${PREVIOUS_FRAME_BINDING}) var warp_previous_frame: texture_2d<f32>;
fn ${PREVIOUS_FUNCTION}(at: vec2f) -> vec4f {
	let last = vec2i(textureDimensions(warp_previous_frame)) - vec2i(1);
	return textureLoad(warp_previous_frame, clamp(vec2i(at), vec2i(0), last), 0);
}`;
const PREVIOUS_WORDS = ["last", "feedback", "before"]; // a value missing under these hints at $previous (src/graphics/shader_holes.rs)
const previousFrames = new Map(); // the task Worker's: shader → the texture it drew last
let keptDepth; // the depth target of the last values.depth paint, cleared each frame that asks for one
const UNIFORM_ALIGNMENT = 16; // a uniform struct is a whole number of 16-byte rows
// the WGSL type of a value of 1…4 floats and its alignment in bytes
const VALUE_TYPES = [, ["f32", 4], ["vec2f", 8], ["vec3f", 16], ["vec4f", 16]];
const VECTOR_FLOATS = 4; // the floats of one vector in an array value, `values.<name>[i]` (src/graphics/gpu.rs)
let gpuDevice;
// a shader painted every frame compiles once, not each frame (card playground-tour: the tour's page crashed in a
// shader animation on a Mac): the task Worker's last MOST_COMPILED modules and pipelines, by their WGSL
const MOST_COMPILED = 16;
const compiledModules = new Map();
const compiledPipelines = new Map();
const WORDS_STATE = 2; // the shared buffer's state when it holds raw 32-bit words, floats or pixels (writeShared's JSON is state 1)
const CANVAS_MISSING = "no canvas of this paint";
const CANVAS_FAILED = "paint: the page's canvas failed, so shaders paint through pixels"; // said once a run, with why
let canvasFailure;
// card uncaught-exception-undefined: Firefox's WebGPU may fail with no error at all (a rejection with undefined), which
// left the program waiting and the console saying "uncaught exception: undefined"; it is said as this instead
const SILENT_FAILURE = "the browser's WebGPU failed without saying why";
const NO_CANVAS_CONTEXT = "this browser gives a Worker's canvas no WebGPU context";
// the task Worker's side: Firefox's WebGPU at times never answers for its adapter or device (card logs-uncaught: the
// tour's shader example then waited until the run's time was up, and the Worker's replacement logged a bare "uncaught
// exception: undefined"): a failure that says which, well within the run's time
const GPU_DEVICE_WAIT_MS = 5000;
// a step doing real work, compiling a shader or reading back a heavy compute shader's result (a slow or software GPU):
// silent this long it fails naming itself, still before the program's side stops waiting (GPU_JOB_WAIT_MS)
const GPU_WORK_WAIT_MS = 25000;
const GPU_JOB_WAIT_MS = 30000; // the program's side: a task Worker that gives no answer in this time is said to
const LOSS_WAIT_MS = 100; // the program's side: why painting into the page's canvas failed, after which pixels are painted
const paintCanvases = new Map(); // the task Worker's: canvas id → {context, format}
let paintCanvasCount = 0; // the program's side: the canvases asked of the page, which names them by this count
let gpuWorker; // the program's side: the task Worker that ran the last GPU job, and keeps its buffers

// the task Worker's side: the numbers the shader left in its storage array, `numbers` before
async function gpuComputedFloats({ shader, numbers, workgroups }) {
	const device = await theGpuDevice();
	const Elements = gpuElements(shader);
	const input = wordPadded(numbers instanceof Elements ? numbers : new Elements(numbers));
	const storage = storageBuffer(device, input.byteLength);
	const readback = readbackBuffer(device, input.byteLength);
	if (input.length) device.queue.writeBuffer(storage, 0, input);
	const encoder = await encodedComputePass(device, shader, workgroups, [{ binding: GPU_BINDING, buffer: storage }]);
	encoder.copyBufferToBuffer(storage, 0, readback, 0, input.byteLength);
	const numbersLeft = new Elements(await gpuReadBack(device, encoder, readback), 0, numbers.length);
	storage.destroy();
	return numbersLeft;
}

// a typed array's bytes in whole words, the last one filled with zeros (an odd count of f16 cells)
const wordBytes = byteLength => Math.ceil(byteLength / WORD_BYTES) * WORD_BYTES;
function wordPadded(array) {
	const bytes = new Uint8Array(wordBytes(array.byteLength));
	bytes.set(new Uint8Array(array.buffer, array.byteOffset, array.byteLength));
	return bytes;
}

const storageBuffer = (device, size) => device.createBuffer({ size, usage: GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_SRC | GPUBufferUsage.COPY_DST });
const readbackBuffer = (device, size) => device.createBuffer({ size, usage: GPUBufferUsage.MAP_READ | GPUBufferUsage.COPY_DST });

// one pass of the compute shader over the buffers bound as [{binding, buffer}], after `before(encoder)`'s copies;
// the encoder, for the copies that read the results back
async function encodedComputePass(device, shader, workgroups, bound, before = () => {}) {
	if (shader.includes(HALVES_ENABLED) && !device.features.has(HALF_FEATURE)) throw new Error(NO_HALVES);
	const module = await gpuModule(device, shader);
	device.pushErrorScope("validation");
	const pipeline = compiledOnce(compiledPipelines, shader, () => device.createComputePipeline({ layout: "auto", compute: { module, entryPoint: GPU_ENTRY_POINT } }));
	const entries = bound.map(({ binding, buffer }) => ({ binding, resource: { buffer } }));
	const bindings = device.createBindGroup({ layout: pipeline.getBindGroupLayout(0), entries });
	const encoder = device.createCommandEncoder();
	before(encoder);
	const pass = encoder.beginComputePass();
	pass.setPipeline(pipeline);
	pass.setBindGroup(0, bindings);
	pass.dispatchWorkgroups(workgroups);
	pass.end();
	return encoder;
}

// the holes every shader has (src/graphics/shader_holes.rs BUILTIN_HOLES), filled here when the shader reads them and the
// program gave none: the image's size, seconds since the first render, the renders before, the pointer and key over
// the canvas (canvas.js, self.pagePointer)
const BUILTIN_HOLES = ["width", "height", "size", "time", "frame", "mouse", "mouse_down", "mouse_wheel", "key"];
let firstRender, renders = 0;
function builtinValues(shader, width, height) {
	const shared = name => {
		const at = self.pagePointer?.names.indexOf(name) ?? -1;
		return at >= 0 ? Atomics.load(self.pagePointer.values, at) : 0;
	};
	firstRender ??= performance.now();
	const value = { width, height, size: [width, height], time: (performance.now() - firstRender) / 1000, frame: renders++,
		mouse: [shared("mouse_x"), shared("mouse_y")], mouse_down: shared("mouse_down"), mouse_wheel: shared("mouse_wheel"), key: shared("key") };
	return Object.fromEntries(BUILTIN_HOLES.filter(name => new RegExp(`\\b${VALUES_NAME}\\.${name}\\b`).test(shader)).map(name => [name, value[name]]));
}

// the task Worker's page canvases on their way, by id: the page posts each straight here (task-worker.js), maybe before
// or after the program's gpu_canvas job for it
const arrivingCanvases = new Map();
const arrivingCanvas = id => {
	if (!arrivingCanvases.has(id)) {
		let arrived;
		arrivingCanvases.set(id, { canvas: new Promise(resolve => arrived = resolve), arrived });
	}
	return arrivingCanvases.get(id);
};
const paintCanvasArrived = ({ id, canvas }) => arrivingCanvas(id).arrived(canvas);

// the task Worker's side of a page canvas: the OffscreenCanvas the page sent, drawn on through WebGPU
async function gpuCanvas({ id, replaces }) {
	const device = await theGpuDevice(); // without an adapter that error, as gpu_render's
	const canvas = await answeredWithin(arrivingCanvas(id).canvas, "canvas from the page");
	arrivingCanvases.delete(id);
	paintCanvases.delete(replaces);
	// the program's fields go on into the canvas of its new size
	for (const [key, buffer] of [...fields]) if (key.startsWith(`${replaces}\n`)) {
		fields.delete(key);
		fields.set(`${id}${key.slice(String(replaces).length)}`, buffer);
	}
	const context = canvas.getContext("webgpu");
	if (!context) throw new Error(NO_CANVAS_CONTEXT); // the program paints through pixels then (gpuPainted)
	const format = navigator.gpu.getPreferredCanvasFormat();
	// COPY_DST: a shader reading $previous draws into a texture it keeps, copied here
	context.configure({ device, format, alphaMode: "opaque", usage: GPUTextureUsage.RENDER_ATTACHMENT | GPUTextureUsage.COPY_DST });
	paintCanvases.set(id, { context, format });
	// an opaque canvas shows black until its first frame: paper at once, as a page's paintings start
	const encoder = device.createCommandEncoder();
	encoder.beginRenderPass({ colorAttachments: [paperAttachment(context.getCurrentTexture())] }).end();
	device.queue.submit([encoder.finish()]);
	return id;
}

const paperAttachment = image => ({ view: image.createView(), loadOp: "clear", clearValue: paperColor(), storeOp: "store" });

// the pixels the fragment shader colored, row by row; given a `canvas` id, drawn into that page canvas instead
async function gpuRendered({ shader, width, height, values, canvas }) {
	const device = await theGpuDevice();
	const painted = paintCanvases.get(canvas);
	if (canvas !== undefined && !painted) throw new Error(CANVAS_MISSING);
	const indexed = Array.isArray(values?.[INDICES_NAME]) ? values[INDICES_NAME] : null;
	if (indexed && !indexed.length) throw new Error(`${VALUES_NAME}.${INDICES_NAME} is empty`);
	const withDepth = !!Number(values?.[DEPTH_NAME]);
	const indirect = Array.isArray(values?.[INDIRECT_NAME]) ? values[INDIRECT_NAME] : null;
	if (indirect && indirect.length !== 4 && indirect.length !== 5) throw new Error(`${VALUES_NAME}.${INDIRECT_NAME} is four numbers, or five for an indexed draw`);
	if (indirect?.length === 5 && !indexed) throw new Error("an indexed indirect draw is five numbers and needs indices");
	if (indirect?.length === 4 && indexed) throw new Error("an indexed indirect draw is five numbers");
	const compute = values?.[COMPUTE_NAME] == null ? "" : String(values[COMPUTE_NAME]);
	if (compute && !compute.includes("@compute")) throw new Error("the compute shader needs @compute fn main");
	const workgroups = [1, 1, 1];
	const asked = values?.[WORKGROUPS_NAME];
	if (asked != null) {
		const counts = [asked].flat();
		if (counts.length < 1 || counts.length > 3 || counts.some(count => !Number.isInteger(count) || count < 1)) throw new Error(`${VALUES_NAME}.${WORKGROUPS_NAME} is one to three positive ints`);
		counts.forEach((count, at) => { workgroups[at] = count; });
	}
	const { declarations, bytes: valueBytes, arrays } = valuesLayout(values);
	const kept = compute ? keptFields(device, compute, values, canvas ?? shader) : [];
	const [renamed, read] = arraysRenamed(shader, [...arrays, ...kept]);
	if (read.length > MOST_BOUND_ARRAYS) throw new Error("a paint binds at most 8 arrays");
	shader = renamed;
	const storages = read.map((array, at) => storageDeclared(STORAGE_BINDING + at, array)).join("");
	const ownVertexStage = shader.match(OWN_VERTEX_STAGE)?.[1];
	if (indexed && !ownVertexStage) throw new Error("indexed draw needs the shader's own vertex stage");
	const vertices = ownVertexStage ? "" : FULL_IMAGE_VERTICES;
	const readsPrevious = new RegExp(`\\b${VALUES_NAME}\\.${PREVIOUS_HOLE}\\b`).test(shader);
	const code = readsPrevious
		? shader.replaceAll(`${VALUES_NAME}.${PREVIOUS_HOLE}`, PREVIOUS_FUNCTION) + vertices + declarations + storages + PREVIOUS_FRAME
		: shader + vertices + declarations + storages;
	const module = await gpuModule(device, code);
	const format = painted?.format ?? "rgba8unorm";
	const usage = GPUTextureUsage.RENDER_ATTACHMENT | GPUTextureUsage.COPY_SRC | (readsPrevious ? GPUTextureUsage.TEXTURE_BINDING : 0);
	const canvasImage = painted?.context.getCurrentTexture();
	const image = canvasImage && !readsPrevious ? canvasImage : device.createTexture({ size: [width, height], format, usage });
	device.pushErrorScope("validation");
	const blendQuads = !!ownVertexStage && !(indexed && withDepth);
	const { pipeline, groupLayout } = compiledOnce(compiledPipelines, `${format}\n${withDepth ? 1 : 0}\n${indexed ? 1 : 0}\n${code}`, () => {
		const groupLayout = declarations || readsPrevious || read.length ? bindingsGroupLayout(device, !!declarations, readsPrevious, read.length) : undefined;
		const layout = groupLayout ? device.createPipelineLayout({ bindGroupLayouts: [groupLayout] }) : "auto";
		const pipeline = device.createRenderPipeline({
			layout,
			vertex: { module, entryPoint: ownVertexStage ?? VERTEX_ENTRY_POINT },
			fragment: { module, entryPoint: GPU_ENTRY_POINT, targets: [blendQuads ? { format, blend: OVER } : { format }] },
			...ownVertexStage && !indexed ? { primitive: { topology: "triangle-strip" } } : {},
			...indexed ? { primitive: { topology: "triangle-list" } } : {},
			...withDepth ? { depthStencil: { format: DEPTH_FORMAT, depthWriteEnabled: true, depthCompare: "less-equal" } } : {},
		});
		return { pipeline, groupLayout };
	});
	const previous = readsPrevious ? previousFrame(device, `${format}\n${shader}`, image) : undefined;
	const extras = arrays.filter(({ name }) => !read.some(item => item.name === name));
	for (const array of arrays) if (array.file) array.bytes = await atlasBytes(array.file);
	const uniform = groupLayout ? uniformBinding(device, groupLayout, declarations ? valueBytes : undefined, previous, read, extras) : undefined;
	const released = () => {
		uniform?.buffers.forEach(buffer => buffer.destroy());
		indexBuffer?.destroy();
		indirectBuffer?.destroy();
	};
	const depth = withDepth ? depthTarget(device, width, height) : undefined;
	const indexUsage = GPUBufferUsage.INDEX | GPUBufferUsage.COPY_DST | (compute ? GPUBufferUsage.STORAGE : 0);
	const indexBuffer = indexed && device.createBuffer({ size: indexed.length * PIXEL_BYTES, usage: indexUsage });
	if (indexBuffer) device.queue.writeBuffer(indexBuffer, 0, new Uint32Array(indexed));
	const indirectBuffer = indirect && device.createBuffer({ size: indirect.length * PIXEL_BYTES, usage: GPUBufferUsage.INDIRECT | GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_DST });
	if (indirectBuffer) device.queue.writeBuffer(indirectBuffer, 0, new Uint32Array(indirect));
	const encoder = device.createCommandEncoder();
	if (withDepth) pyramidBuffer(device, width, height);
	if (compute) await dispatchCull(device, encoder, compute, workgroups, { ...(uniform?.named ?? {}), ...Object.fromEntries(kept.map(({ name, buffer }) => [name, buffer])), ...indexBuffer ? { [INDICES_NAME]: indexBuffer } : {}, ...indirectBuffer ? { [INDIRECT_NAME]: indirectBuffer } : {}, ...keptPyramid ? { pyramid: keptPyramid.buffer } : {} });
	const pass = encoder.beginRenderPass({
		colorAttachments: [paperAttachment(image)],
		...depth ? { depthStencilAttachment: { view: depth.createView(), depthClearValue: 1, depthLoadOp: "clear", depthStoreOp: "store" } } : {},
	});
	pass.setPipeline(pipeline);
	if (uniform) pass.setBindGroup(0, uniform.bindings);
	const instances = Math.max(0, Math.trunc(values?.[INSTANCES_NAME] ?? 1));
	if (indirectBuffer && indexed) {
		pass.setIndexBuffer(indexBuffer, "uint32");
		pass.drawIndexedIndirect(indirectBuffer, 0);
	} else if (indirectBuffer) pass.drawIndirect(indirectBuffer, 0);
	else if (indexBuffer) {
		pass.setIndexBuffer(indexBuffer, "uint32");
		pass.drawIndexed(indexed.length, instances);
	} else if (ownVertexStage) pass.draw(QUAD_CORNERS, instances);
	else pass.draw(TRIANGLE_CORNERS);
	pass.end();
	if (withDepth) encodePyramid(device, encoder, depth, width, height);
	if (canvasImage && image !== canvasImage)
		encoder.copyTextureToTexture({ texture: image }, { texture: canvasImage }, [Math.min(width, canvasImage.width), Math.min(height, canvasImage.height)]);
	// a painting into the page's canvas answers once the GPU drew it, as the native window's surface waits for the
	// display: a program painting each frame no faster than its shader draws (card example-native-mismatch: #ocean queued
	// frames far ahead of its heavy shader and showed two)
	if (painted) return gpuSubmitted(device, encoder).then(() => answeredWithin(device.queue.onSubmittedWorkDone(), "drawn frame", GPU_WORK_WAIT_MS)).finally(released);
	const rowBytes = Math.ceil(width * PIXEL_BYTES / ROW_ALIGNMENT) * ROW_ALIGNMENT;
	const readback = readbackBuffer(device, rowBytes * height);
	encoder.copyTextureToBuffer({ texture: image }, { buffer: readback, bytesPerRow: rowBytes, rowsPerImage: height }, [width, height]);
	const bytes = new Uint8Array(await gpuReadBack(device, encoder, readback));
	if (!readsPrevious) image.destroy();
	released();
	const pixels = new Uint32Array(width * height);
	for (let row = 0, pixel = 0; row < height; row++)
		for (let at = row * rowBytes; at < row * rowBytes + width * PIXEL_BYTES; at += PIXEL_BYTES)
			pixels[pixel++] = (OPAQUE | bytes[at] << 16 | bytes[at + 1] << 8 | bytes[at + 2]) >>> 0;
	return pixels;
}

// the WGSL declaring `values` (appended, so the shader's line numbers stay the user's) and the uniform's bytes, laid out
// as WGSL aligns its members (lib/host/native_host/gpu.warp values_laid_out); its lists of more than 4 numbers or of
// vectors as storage arrays [{name, numbers}], 4 numbers a vec4f
// EZAT version 1: zlib RGBA8. The value is `file#id#sha256`, the same file the native host reads.
const ATLAS_MAGIC = 0x54415a45;
const ATLAS_VERSION = 1;
const ATLAS_RGBA8 = 1;
const ATLAS_HEAD = 44;
const ATLAS_ENTRY = 28;
let atlasCacheKey = "";
let atlasCacheBytes = null;

async function inflateZlib(payload) {
	if (typeof DecompressionStream === "function") {
		const stream = new Blob([payload]).stream().pipeThrough(new DecompressionStream("deflate"));
		return new Uint8Array(await new Response(stream).arrayBuffer());
	}
	throw new Error("atlas: this page has no zlib inflate");
}

async function atlasBytes(spec) {
	if (spec === atlasCacheKey && atlasCacheBytes) return atlasCacheBytes;
	const parts = spec.split("#");
	if (parts.length !== 3) throw new Error("atlas: a baked atlas is file#id#hash");
	const file = readBytes(parts[0]);
	const view = new DataView(file.buffer, file.byteOffset, file.byteLength);
	if (file.byteLength < ATLAS_HEAD || view.getUint32(0, true) !== ATLAS_MAGIC) throw new Error(`atlas: ${parts[0]} is not an EZAT atlas`);
	if (view.getUint32(4, true) !== ATLAS_VERSION) throw new Error(`atlas: ${parts[0]} is a different version`);
	const digest = [...file.subarray(12, ATLAS_HEAD)].map(byte => byte.toString(16).padStart(2, "0")).join("");
	if (digest !== parts[2]) throw new Error(`atlas: ${parts[0]} does not match its source hash`);
	const entries = view.getUint32(8, true);
	const id = Number(parts[1]);
	let found = null;
	for (let i = 0; i < entries; i++) {
		const base = ATLAS_HEAD + i * ATLAS_ENTRY;
		if (view.getUint32(base, true) !== id || view.getUint32(base + 12, true) !== ATLAS_RGBA8) continue;
		found = {
			raw: view.getUint32(base + 16, true),
			payload: view.getUint32(base + 20, true),
			at: view.getUint32(base + 24, true),
		};
	}
	if (!found || found.at + found.payload > file.byteLength) throw new Error(`atlas: ${parts[0]} has no RGBA8 entry ${parts[1]}`);
	const pixels = await inflateZlib(file.subarray(found.at, found.at + found.payload));
	if (pixels.byteLength !== found.raw) throw new Error(`atlas: ${parts[0]} did not inflate`);
	atlasCacheKey = spec;
	atlasCacheBytes = pixels;
	return pixels;
}

function valuesLayout(values) {
	const members = [], floats = [], arrays = [];
	const padded = (numbers, alignment) => { while (numbers.length % (alignment / 4)) numbers.push(0); };
	for (const [name, value] of Object.entries(values ?? {})) {
		// an index buffer and the depth switch are host draw state, not uniform members
		if (name === INDICES_NAME || name === DEPTH_NAME || name === INDIRECT_NAME || name === COMPUTE_NAME || name === WORKGROUPS_NAME) continue;
		// a file the host reads into the storage buffer (the baked atlas)
		if (typeof value === "string") {
			if (arrays.length === MOST_STORAGE_ARRAYS) throw new Error(`${VALUES_NAME}.${name} is one list too many: paint's values hold at most ${MOST_STORAGE_ARRAYS}`);
			arrays.push({ name, file: value });
			continue;
		}
		// a list of lists: one vec4f per item, its missing coordinates 0
		const vectors = Array.isArray(value) && value.some(Array.isArray);
		if (vectors && value.some(item => [item].flat().length > VECTOR_FLOATS)) throw new Error(`${VALUES_NAME}.${name} is a list of vectors of up to four numbers, got ${JSON.stringify(value)}`);
		const numbers = vectors ? value.flatMap(item => [...[item].flat(), 0, 0, 0, 0].slice(0, VECTOR_FLOATS)) : [value].flat();
		if (!numbers.every(number => typeof number === "number")) throw new Error(`${VALUES_NAME}.${name} is a number or a list of numbers, got ${JSON.stringify(value)}${PREVIOUS_WORDS.includes(name) ? "; the shader's own previous frame is $previous(at)" : ""}`);
		if (vectors || numbers.length > VECTOR_FLOATS) {
			if (arrays.length === MOST_STORAGE_ARRAYS) throw new Error(`${VALUES_NAME}.${name} is one list too many: paint's values hold at most ${MOST_STORAGE_ARRAYS}`);
			padded(numbers, UNIFORM_ALIGNMENT);
			arrays.push({ name, numbers: new Float32Array(numbers.length ? numbers : [0, 0, 0, 0]) });
			continue;
		}
		const [type, alignment] = VALUE_TYPES[numbers.length] ?? [];
		if (!type) throw new Error(`${VALUES_NAME}.${name} is one number or a list of two to four, got ${numbers.length}`);
		padded(floats, alignment);
		floats.push(...numbers);
		members.push(`${name}: ${type}`);
	}
	if (!members.length) return { declarations: "", bytes: new ArrayBuffer(0), arrays };
	padded(floats, UNIFORM_ALIGNMENT);
	const declarations = `\nstruct WarpValues { ${members.join(", ")} }\n@group(0) @binding(${VALUES_BINDING}) var<uniform> ${VALUES_NAME}: WarpValues;`;
	return { declarations, bytes: new Float32Array(floats).buffer, arrays };
}
// the compute shader's fields, each {name, type, buffer}: kept under `owner` (the program's canvas, else its shader)
function keptFields(device, compute, values, owner) {
	return [...compute.matchAll(FIELD_ARRAY)].filter(([, name]) => !(name in (values ?? {}))).map(([, name, type, element, length]) => {
		const stride = ELEMENT_STRIDES.find(([pattern]) => pattern.test(element.replaceAll(/\s/g, "")))?.[1];
		if (!stride) throw new Error(`the field ${name} holds ${element}: a field's elements are numbers or vectors of them`);
		const key = `${owner}\n${name}: ${type}`;
		if (!fields.has(key)) fields.set(key, storageBuffer(device, stride * Number(length)));
		return { name, type, buffer: fields.get(key) };
	});
}

// `values.name` in a shader, whole
const holePattern = name => new RegExp(`\\b${VALUES_NAME}\\.${name}\\b`, "g");
const holeRead = (shader, name) => holePattern(name).test(shader);

// an explicit layout holding the uniform and the previous frame, so a shader that leaves `values` unread still binds them
const SHADER_STAGES = GPUShaderStage.VERTEX | GPUShaderStage.FRAGMENT;
const bindingsGroupLayout = (device, takesValues, readsPrevious, arrayCount) => device.createBindGroupLayout({ entries: [
	...takesValues ? [{ binding: VALUES_BINDING, visibility: SHADER_STAGES, buffer: { type: "uniform" } }] : [],
	...Array.from({ length: arrayCount }, (_, at) => ({ binding: STORAGE_BINDING + at, visibility: SHADER_STAGES, buffer: { type: "read-only-storage" } })),
	...readsPrevious ? [{ binding: PREVIOUS_FRAME_BINDING, visibility: GPUShaderStage.FRAGMENT, texture: { sampleType: "unfilterable-float" } }] : [],
] });

// this frame's values in a uniform buffer (no bytes: none), its storage arrays and the previous frame, bound as the
// layout says; the buffers are destroyed once the frame is read
function uniformBinding(device, groupLayout, bytes, previous, arrays, extras = []) {
	const filled = (data, usage) => {
		const buffer = device.createBuffer({ size: data.byteLength, usage: usage | GPUBufferUsage.COPY_DST });
		device.queue.writeBuffer(buffer, 0, data);
		return buffer;
	};
	const buffer = bytes && filled(bytes, GPUBufferUsage.UNIFORM);
	// a field's buffer is kept, not filled anew
	const storages = arrays.map(array => array.buffer ?? filled(array.bytes ?? array.numbers, GPUBufferUsage.STORAGE));
	const extraBuffers = extras.map(array => filled(array.bytes ?? array.numbers, GPUBufferUsage.STORAGE));
	const entries = [
		...buffer ? [{ binding: VALUES_BINDING, resource: { buffer } }] : [],
		...storages.map((storage, at) => ({ binding: STORAGE_BINDING + at, resource: { buffer: storage } })),
		...previous ? [{ binding: PREVIOUS_FRAME_BINDING, resource: previous.createView() }] : [],
	];
	const named = Object.fromEntries([
		...arrays.map(({ name }, at) => [name, storages[at]]),
		...extras.map(({ name }, at) => [name, extraBuffers[at]]),
	]);
	return { buffers: [...buffer ? [buffer] : [], ...storages.filter((storage, at) => !arrays[at].buffer), ...extraBuffers], bindings: device.createBindGroup({ layout: groupLayout, entries }), named };
}

// the compute shader writes buffers the draw then reads. Its storage names are the value names, plus indices and indirect.
async function dispatchCull(device, encoder, shader, workgroups, named) {
	const declared = storageArrays(shader);
	if (declared.length > MOST_BOUND_ARRAYS) throw new Error("a compute shader binds at most 8 arrays");
	if (!declared.length) throw new Error("the compute shader declares no storage array");
	const bound = declared.map(array => {
		const buffer = named[array.name];
		if (!buffer) throw new Error(`the compute shader's array ${array.name} has no values`);
		const integer = array.element === "u32" || array.element === "i32";
		if (integer && array.name !== INDICES_NAME && array.name !== INDIRECT_NAME) throw new Error(`${array.name} is float values, and the compute shader asks for ${array.element}`);
		return { binding: array.binding, buffer };
	});
	const module = await gpuModule(device, shader);
	// one layout for every pass, so each binds all the arrays, those it leaves unread too
	const groupLayout = device.createBindGroupLayout({ entries: declared.map(({ binding, access }) => ({ binding, visibility: GPUShaderStage.COMPUTE, buffer: { type: access === "read" ? "read-only-storage" : "storage" } })) });
	const layout = device.createPipelineLayout({ bindGroupLayouts: [groupLayout] });
	const bindings = device.createBindGroup({ layout: groupLayout, entries: bound.map(({ binding, buffer }) => ({ binding, resource: { buffer } })) });
	for (const [, entryPoint] of shader.matchAll(COMPUTE_ENTRY)) {
		const pipeline = compiledOnce(compiledPipelines, `${entryPoint}\n${shader}`, () => device.createComputePipeline({ layout, compute: { module, entryPoint } }));
		const pass = encoder.beginComputePass();
		pass.setPipeline(pipeline);
		pass.setBindGroup(0, bindings);
		pass.dispatchWorkgroups(workgroups[0], workgroups[1], workgroups[2]);
		pass.end();
	}
}

// the frame `shader` drew last when it has the size of `image` (a black one otherwise), and `image` kept as the next
function previousFrame(device, shader, image) {
	const kept = previousFrames.get(shader);
	const fits = kept?.width === image.width && kept?.height === image.height;
	const previous = fits ? kept : device.createTexture({ size: [image.width, image.height], format: image.format, usage: GPUTextureUsage.TEXTURE_BINDING });
	if (!fits) kept?.destroy();
	previousFrames.delete(shader);
	compiledOnce(previousFrames, shader, () => image);
	return previous;
}

// the value made for `key` before, or made now; the oldest of more than MOST_COMPILED goes
function compiledOnce(cache, key, make) {
	if (!cache.has(key)) cache.set(key, make());
	if (cache.size > MOST_COMPILED) cache.delete(cache.keys().next().value);
	return cache.get(key);
}

// the task Worker's device, asked for once
const theGpuDevice = async () => gpuDevice ??= await gpuDeviceOrFailure();

async function gpuDeviceOrFailure() {
	const adapter = await answeredWithin(self.navigator.gpu?.requestAdapter(), "adapter");
	if (!adapter) throw new Error("this browser offers no WebGPU adapter");
	// f16 shaders where the adapter has them, and float32-filterable where an rgba32float field needs linear samples
	const requiredFeatures = [HALF_FEATURE, "float32-filterable"].filter(feature => adapter.features.has(feature));
	const device = await answeredWithin(adapter.requestDevice({ requiredFeatures }), "device");
	// a lost device (the GPU process restarted) is asked for anew by the next job, with all made on it
	device.lost.then(() => forgetDevice(device));
	return device;
}

// the browser's answer for `what`, or a failure once it has kept silent for `waitMs` (GPU_DEVICE_WAIT_MS): every awaited step
// (adapter, device, page canvas, compilation, validation, read-back) names itself, within the run's own stop
function answeredWithin(promise, what, waitMs = GPU_DEVICE_WAIT_MS) {
	let timer;
	const silence = new Promise((_, fail) => {
		timer = setTimeout(() => fail(new Error(`the browser's WebGPU gave no ${what} in ${waitMs / 1000} s`)), waitMs);
	});
	return Promise.race([promise, silence]).finally(() => clearTimeout(timer));
}

function forgetDevice(device) {
	if (gpuDevice !== device) return;
	gpuDevice = undefined;
	keptDepth = undefined;
	keptPyramid?.buffer.destroy();
	keptPyramid = undefined;
	[compiledModules, compiledPipelines, paintCanvases, previousFrames].forEach(made => made.clear());
}

// one depth32float target for this size, cleared by the pass that draws into it (gpu.warp depth_target)
function depthTarget(device, width, height) {
	if (keptDepth?.width === width && keptDepth?.height === height) return keptDepth.texture;
	keptDepth?.texture.destroy();
	const texture = device.createTexture({ size: [width, height], format: DEPTH_FORMAT, usage: GPUTextureUsage.RENDER_ATTACHMENT | GPUTextureUsage.TEXTURE_BINDING });
	keptDepth = { texture, width, height };
	return texture;
}

// one max depth per 8 by 8 tile, kept for the next frame. A new size starts at the far plane.
let keptPyramid;
const pyramidTiles = (width, height) => [Math.ceil(width / 8), Math.ceil(height / 8)];
function pyramidBuffer(device, width, height) {
	const [tilesX, tilesY] = pyramidTiles(width, height);
	const n = tilesX * tilesY;
	if (keptPyramid?.n === n && keptPyramid.width === width && keptPyramid.height === height) return keptPyramid.buffer;
	keptPyramid?.buffer.destroy();
	const buffer = device.createBuffer({ size: n * PIXEL_BYTES, usage: GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_DST });
	device.queue.writeBuffer(buffer, 0, new Float32Array(n).fill(1));
	keptPyramid = { buffer, n, width, height };
	return buffer;
}
function encodePyramid(device, encoder, depth, width, height) {
	const [tilesX, tilesY] = pyramidTiles(width, height);
	const shader = `@group(0) @binding(0) var src: texture_depth_2d;
@group(0) @binding(1) var<storage, read_write> pyramid: array<f32>;
@compute @workgroup_size(1) fn main() {
	let tiles_x = ${tilesX}u;
	let tiles_y = ${tilesY}u;
	let width = ${width}u;
	let height = ${height}u;
	for (var ty = 0u; ty < tiles_y; ty++) {
		for (var tx = 0u; tx < tiles_x; tx++) {
			var far = 0.0;
			let x0 = tx * 8u;
			let y0 = ty * 8u;
			for (var dy = 0u; dy < 8u; dy++) {
				for (var dx = 0u; dx < 8u; dx++) {
					let x = x0 + dx;
					let y = y0 + dy;
					if (x < width && y < height) {
						let d = textureLoad(src, vec2i(i32(x), i32(y)), 0);
						if (d > far) { far = d; }
					}
				}
			}
			pyramid[ty * tiles_x + tx] = far;
		}
	}
}`;
	const module = device.createShaderModule({ code: shader });
	const pipeline = device.createComputePipeline({ layout: "auto", compute: { module, entryPoint: GPU_ENTRY_POINT } });
	const view = depth.createView();
	const pass = encoder.beginComputePass();
	pass.setPipeline(pipeline);
	pass.setBindGroup(0, device.createBindGroup({ layout: pipeline.getBindGroupLayout(0), entries: [
		{ binding: 0, resource: view },
		{ binding: 1, resource: { buffer: keptPyramid.buffer } },
	] }));
	pass.dispatchWorkgroups(1, 1, 1);
	pass.end();
}

// after a failed job: a device lost by it is forgotten before the program asks again (its loss is said a moment later)
async function forgetDeviceIfLost() {
	const device = gpuDevice;
	if (!device) return;
	const lost = await Promise.race([device.lost.then(() => true), new Promise(done => setTimeout(done, LOSS_WAIT_MS, false))]);
	if (lost) forgetDevice(device);
}

// the shader's module, checked once: asking a module for its compilation info a second time crashed Chrome's page (Mac)
const gpuModule = (device, shader) => compiledOnce(compiledModules, shader, () => checkedModule(device, shader));

// the shader's module; one that does not compile throws why and where, `the shader: why at line:column`: a shader
// block's place in the program, as the program placed its text (lib/prelude.warp shader_words), as natively
// (lib/host/native_host/gpu.warp shader_failure); why in the browser's words (Chrome's Tint, Firefox's naga)
async function checkedModule(device, shader) {
	const module = device.createShaderModule({ code: shader });
	const problems = (await answeredWithin(module.getCompilationInfo(), "compilation report", GPU_WORK_WAIT_MS)).messages.filter(message => message.type === "error");
	if (problems.length) throw new Error(SHADER_SUBJECT + problems.map(problem => `${essence(problem.message)} at ${problem.lineNum}:${problem.linePos}`).join("; "));
	return module;
}

// a GPU's report in one line, as gpu.rs essence: Firefox's naga report `Shader parsing error: expected "{"` followed by
// its excerpt of the shader gives what follows `error: `, a one-line report (Chrome, Safari) stays as it is
function essence(report) {
	const lines = report.split("\n").map(line => line.trim()).filter(Boolean);
	const cause = lines.map(line => line.split("error: ")).find(parts => parts.length > 1);
	return cause ? cause.slice(1).join("error: ") : lines.at(-1) ?? report;
}

// submit the encoded work (its validation scope pushed before the pipeline) and give the bytes it left in readback
const gpuReadBack = async (device, encoder, readback) => (await gpuReadBacks(device, encoder, [readback]))[0];

// submit the encoded work; the validation error it raised throws
async function gpuSubmitted(device, encoder) {
	device.queue.submit([encoder.finish()]);
	const invalid = await answeredWithin(device.popErrorScope(), "validation report");
	if (invalid) throw new Error(invalid.message);
}

// as gpuReadBack, the bytes of each readback buffer
async function gpuReadBacks(device, encoder, readbacks) {
	await gpuSubmitted(device, encoder);
	return Promise.all(readbacks.map(async readback => {
		await answeredWithin(readback.mapAsync(GPUMapMode.READ), "read-back result", GPU_WORK_WAIT_MS);
		const bytes = readback.getMappedRange().slice(0);
		readback.destroy();
		return bytes;
	}));
}

// the storage arrays the shader declares, each with the typed array of its elements: f32 unless array<i32>, array<u32> or array<f16>
const storageArrays = shader => [...shader.matchAll(STORAGE_ARRAY)].map(([, binding, access, name, element]) => ({ binding: Number(binding), access, name, element, Elements: ELEMENT_ARRAYS[element] ?? Float32Array }));

// the typed array of the numbers a compute shader reads at @binding(0) (natively gpu.rs element_of)
const gpuElements = shader => storageArrays(shader).find(array => array.binding === GPU_BINDING)?.Elements ?? Float32Array;

// gpu_compute over named arrays: each bound where the shader declares the storage array of that name; the arrays it
// left, by name, in the order given (natively gpu.rs compute_named)
async function gpuComputedArrays({ shader, arrays, workgroups }) {
	const declared = storageArrays(shader);
	const names = declared.map(array => array.name).join(", ");
	const missing = declared.find(array => !(array.name in arrays));
	if (missing) throw new Error(`the shader's array ${missing.name} has no numbers: give each of ${names}`);
	const given = Object.entries(arrays).map(([name, numbers]) => {
		const array = declared.find(array => array.name === name);
		if (!array) throw new Error(`the shader declares no storage array ${name}, only ${names}`);
		if (!numbers.length) throw new Error(`${name} is empty: a storage array holds at least one number`);
		return { ...array, numbers: new array.Elements(numbers) };
	});
	const device = await theGpuDevice();
	const storages = given.map(({ numbers }) => {
		const bytes = wordPadded(numbers);
		const storage = storageBuffer(device, bytes.byteLength);
		device.queue.writeBuffer(storage, 0, bytes);
		return storage;
	});
	const readbacks = storages.map(storage => readbackBuffer(device, storage.size));
	const bound = given.map(({ binding }, at) => ({ binding, buffer: storages[at] }));
	const encoder = await encodedComputePass(device, shader, workgroups, bound);
	storages.forEach((storage, at) => encoder.copyBufferToBuffer(storage, 0, readbacks[at], 0, storage.size));
	const left = await gpuReadBacks(device, encoder, readbacks).finally(() => storages.forEach(storage => storage.destroy()));
	return Object.fromEntries(given.map(({ name, Elements, numbers }, at) => [name, new Elements(left[at], 0, numbers.length)]));
}
// the arrays as one run of 32-bit words, each in whole words after the one before
function joinedWords(arrays) {
	const words = new Uint32Array(arrays.reduce((total, array) => total + wordBytes(array.byteLength) / WORD_BYTES, 0));
	arrays.reduce((at, array) => (words.set(new Uint32Array(wordPadded(array).buffer), at), at + wordBytes(array.byteLength) / WORD_BYTES), 0);
	return words;
}

// a GPU list's cells by gpu_list_cell (extensions/gpu_lists/runtime.warp GpuCell): f32, i32, u32, f16 (as raw bits),
// a vector's f32 lanes
const GPU_LIST_CELLS = [Float32Array, Int32Array, Uint32Array, Uint16Array, Uint16Array, Float32Array, Float32Array, Float32Array];
const GPU_CELL_NAMES = ["f32", "i32", "u32", "f16", "bf16", "vec2", "vec3", "vec4"];
const plainArrays = arrays => Object.fromEntries(Object.entries(arrays).map(([name, array]) => [name, Array.from(array)]));
// gpu_compute's {values} as a list; a kernel's floats and an image's pixels as {words}, given back as raw bytes
// (writeSharedWords), not as JSON
// gpu_fields / gpu_pass / gpu_show / gpu_picture (card gpu-multipass): ping-pong targets that stay on the GPU.
// One command encoder holds every pass of a frame; nothing is read back until gpu_picture. field_<name>(uv), uv in 0..1.
const MOST_FIELDS = 8;
const FIELD_NAME = /^[A-Za-z_][A-Za-z0-9_]*$/;
const fieldSets = new Map();
const passPipelines = new Map();
let nextFieldId = 0;
let fieldsDevice = null;
let fieldsChoice = null;
let fieldSampler = null;
let openEncoder = null;
let passFrame = 0;
let passFrameOpen = false;
const pendingBuffers = [];
const releasePending = () => { pendingBuffers.splice(0).forEach(buffer => buffer.destroy()); };
const encoderOf = device => {
	if (!openEncoder) {
		device.pushErrorScope("validation");
		openEncoder = device.createCommandEncoder();
	}
	return openEncoder;
};
const takeEncoder = device => {
	const encoder = encoderOf(device);
	openEncoder = null;
	return encoder;
};
const openPassFrame = () => { if (!passFrameOpen) { passFrame++; passFrameOpen = true; } };
const closePassFrame = () => { passFrameOpen = false; };
function fieldsOn(device) {
	if (fieldsDevice && fieldsDevice !== device) {
		fieldSets.clear();
		passPipelines.clear();
		fieldSampler = null;
		fieldsChoice = null;
		openEncoder = null;
		releasePending();
	}
	fieldsDevice = device;
}
async function textureRenderable(device, format) {
	device.pushErrorScope("validation");
	let texture;
	try {
		texture = device.createTexture({ size: [1, 1], format, usage: GPUTextureUsage.RENDER_ATTACHMENT | GPUTextureUsage.TEXTURE_BINDING });
	} catch (failure) {
		await device.popErrorScope();
		return false;
	}
	const error = await device.popErrorScope();
	texture.destroy();
	return !error;
}
async function fieldsChoiceOf(device) {
	fieldsOn(device);
	if (fieldsChoice) return fieldsChoice;
	const float32 = device.features.has("float32-filterable");
	const candidates = [
		{ name: "rgba16float", filterable: true },
		{ name: "rgba32float", filterable: float32 },
		{ name: "rgba8unorm", filterable: true },
	];
	for (const candidate of candidates) {
		if (await textureRenderable(device, candidate.name)) { fieldsChoice = candidate; return candidate; }
	}
	throw new Error("gpu_fields: this GPU has no render target for a field");
}
const samplerOf = (device, filterable) => fieldSampler ??= device.createSampler({
	addressModeU: "clamp-to-edge", addressModeV: "clamp-to-edge", addressModeW: "clamp-to-edge",
	magFilter: filterable ? "linear" : "nearest", minFilter: filterable ? "linear" : "nearest",
});
function fieldNames(names) {
	if (!Array.isArray(names) || names.length < 1 || names.length > MOST_FIELDS) throw new Error("gpu_fields takes 1 to 8 names");
	const kept = [];
	for (const name of names) {
		if (!FIELD_NAME.test(name)) throw new Error(`gpu_fields: ${name} is not a field name`);
		if (kept.includes(name)) throw new Error(`gpu_fields: ${name} is named twice`);
		kept.push(name);
	}
	return kept;
}
function fieldValues(shader, width, height, given) {
	openPassFrame();
	const shared = name => {
		const at = self.pagePointer?.names.indexOf(name) ?? -1;
		return at >= 0 ? Atomics.load(self.pagePointer.values, at) : 0;
	};
	firstRender ??= performance.now();
	const value = {
		width, height, size: [width, height], time: (performance.now() - firstRender) / 1000, frame: passFrame,
		mouse: [shared("mouse_x"), shared("mouse_y")], mouse_down: shared("mouse_down"), mouse_wheel: shared("mouse_wheel"), key: shared("key"),
	};
	const holes = Object.fromEntries(BUILTIN_HOLES.filter(name => new RegExp(`\\b${VALUES_NAME}\\.${name}\\b`).test(shader)).map(name => [name, value[name]]));
	return { ...holes, ...(given ?? {}) };
}
function fieldSample(name, binding, filterable) {
	const tex = `field_${name}_tex`;
	const declared = `\n@group(0) @binding(${binding}) var ${tex}: texture_2d<f32>;`;
	if (filterable) return `${declared}\nfn field_${name}(uv: vec2f) -> vec4f {\n\tlet placed = clamp(uv, vec2f(0.0), vec2f(1.0));\n\treturn textureSampleLevel(${tex}, field_sampler, placed, 0.0);\n}`;
	return `${declared}\nfn field_${name}(uv: vec2f) -> vec4f {\n\tlet size = vec2f(textureDimensions(${tex}));\n\tlet at = clamp(uv, vec2f(0.0), vec2f(1.0)) * size - vec2f(0.5);\n\tlet low = vec2i(floor(at));\n\tlet f = fract(at);\n\tlet last = vec2i(size) - vec2i(1);\n\tlet s00 = textureLoad(${tex}, clamp(low, vec2i(0), last), 0);\n\tlet s10 = textureLoad(${tex}, clamp(low + vec2i(1, 0), vec2i(0), last), 0);\n\tlet s01 = textureLoad(${tex}, clamp(low + vec2i(0, 1), vec2i(0), last), 0);\n\tlet s11 = textureLoad(${tex}, clamp(low + vec2i(1, 1), vec2i(0), last), 0);\n\treturn mix(mix(s00, s10, f.x), mix(s01, s11, f.x), f.y);\n}`;
}
// each storage array of these the shader reads, `values.mesh` its own array warp_values_mesh: [the shader so renamed,
// those it reads]
function arraysRenamed(shader, arrays) {
	const read = arrays.filter(({ name }) => holeRead(shader, name));
	return [read.reduce((code, { name }) => code.replace(holePattern(name), STORAGE_PREFIX + name), shader), read];
}
// a storage array's declaration: a field read by the draw its own type, an atomic read as its plain number
const storageDeclared = (binding, { name, type = "array<vec4f>" }) => `\n@group(0) @binding(${binding}) var<storage, read> ${STORAGE_PREFIX}${name}: ${type.replaceAll(/atomic<(\w+)>/g, "$1")};`;
function passShader(shader, declarations, reads, filterable) {
	let code = shader;
	if (!OWN_VERTEX_STAGE.test(shader)) code += FULL_IMAGE_VERTICES;
	code += declarations;
	let binding = 1;
	if (filterable && reads.length) {
		code += "\n@group(0) @binding(1) var field_sampler: sampler;";
		binding = 2;
	}
	for (const name of reads) code += fieldSample(name, binding++, filterable);
	return code;
}
async function passPipeline(device, code, format, ownVertex) {
	const key = `${format}\n${ownVertex ? 1 : 0}\n${code}`;
	if (passPipelines.has(key)) return passPipelines.get(key);
	const module = await gpuModule(device, code);
	device.pushErrorScope("validation");
	const pipeline = device.createRenderPipeline({
		layout: "auto",
		vertex: { module, entryPoint: ownVertex ?? VERTEX_ENTRY_POINT },
		fragment: { module, entryPoint: GPU_ENTRY_POINT, targets: [{ format }] },
		...ownVertex ? { primitive: { topology: "triangle-strip" } } : {},
	});
	const error = await device.popErrorScope();
	if (error) throw new Error(`${SHADER_SUBJECT}${error.message}`);
	passPipelines.set(key, pipeline);
	return pipeline;
}
function setOf(device, id) {
	fieldsOn(device);
	const set = fieldSets.get(id);
	if (!set) throw new Error(`no fields ${id}`);
	return set;
}
async function drawInto(device, shader, values, set, target, format) {
	const choice = await fieldsChoiceOf(device);
	const laid = valuesLayout(values);
	// (a list it reads is a storage array, bound after the fields as paint binds its own)
	const [renamed, arrays] = arraysRenamed(shader, laid.arrays);
	const reads = set.order.filter(name => new RegExp(`\\bfield_${name}\\b`).test(shader));
	const firstArray = (choice.filterable && reads.length ? 2 : 1) + reads.length;
	const code = passShader(renamed, laid.declarations, reads, choice.filterable) + arrays.map((array, at) => storageDeclared(firstArray + at, array)).join("");
	const ownVertex = shader.match(OWN_VERTEX_STAGE)?.[1];
	const pipeline = await passPipeline(device, code, format, ownVertex);
	const entries = [];
	const written = (data, usage) => {
		const buffer = device.createBuffer({ size: data.byteLength, usage: usage | GPUBufferUsage.COPY_DST });
		device.queue.writeBuffer(buffer, 0, data);
		pendingBuffers.push(buffer);
		return { buffer };
	};
	if (laid.declarations) entries.push({ binding: VALUES_BINDING, resource: written(laid.bytes, GPUBufferUsage.UNIFORM) });
	let binding = 1;
	if (choice.filterable && reads.length) {
		entries.push({ binding: 1, resource: samplerOf(device, true) });
		binding = 2;
	}
	for (const name of reads) entries.push({ binding: binding++, resource: set.fields[name].front.createView() });
	for (const array of arrays) entries.push({ binding: binding++, resource: written(array.file ? await atlasBytes(array.file) : array.numbers, GPUBufferUsage.STORAGE) });
	const group = entries.length ? device.createBindGroup({ layout: pipeline.getBindGroupLayout(0), entries }) : undefined;
	const pass = encoderOf(device).beginRenderPass({ colorAttachments: [{ view: target.createView(), loadOp: "clear", storeOp: "store", clearValue: { r: 0, g: 0, b: 0, a: 0 } }] });
	pass.setPipeline(pipeline);
	if (group) pass.setBindGroup(0, group);
	const instances = Math.max(1, Math.trunc(values?.[INSTANCES_NAME] ?? 1));
	pass.draw(ownVertex ? QUAD_CORNERS : TRIANGLE_CORNERS, ownVertex ? instances : 1);
	pass.end();
}
async function gpuFields(job) {
	const device = await theGpuDevice();
	const names = fieldNames(job.names);
	const width = job.width, height = job.height;
	if (width < 1 || height < 1) throw new Error("gpu_fields needs a positive width and height");
	const choice = await fieldsChoiceOf(device);
	const label = names.join(",");
	for (const set of fieldSets.values()) {
		if (set.width === width && set.height === height && set.label === label) return { id: set.id, width, height, format: choice.name };
	}
	const fields = {};
	const usage = GPUTextureUsage.RENDER_ATTACHMENT | GPUTextureUsage.TEXTURE_BINDING;
	for (const name of names) {
		fields[name] = {
			front: device.createTexture({ size: [width, height], format: choice.name, usage }),
			back: device.createTexture({ size: [width, height], format: choice.name, usage }),
		};
	}
	const encoder = device.createCommandEncoder();
	for (const field of Object.values(fields)) {
		for (const texture of [field.front, field.back]) {
			const pass = encoder.beginRenderPass({ colorAttachments: [{ view: texture.createView(), loadOp: "clear", storeOp: "store", clearValue: { r: 0, g: 0, b: 0, a: 0 } }] });
			pass.end();
		}
	}
	device.queue.submit([encoder.finish()]);
	const set = { id: ++nextFieldId, width, height, label, order: names, fields };
	fieldSets.set(set.id, set);
	return { id: set.id, width, height, format: choice.name };
}
async function gpuPass(job) {
	const device = await theGpuDevice();
	const set = setOf(device, job.id);
	const field = set.fields[job.into];
	if (!field) return -1;
	const choice = await fieldsChoiceOf(device);
	const values = fieldValues(job.shader, set.width, set.height, job.values);
	await drawInto(device, job.shader, values, set, field.back, choice.name);
	const kept = field.front;
	field.front = field.back;
	field.back = kept;
	return 0;
}
async function finishPasses(device) {
	if (!openEncoder) return;
	const encoder = openEncoder;
	openEncoder = null;
	await gpuSubmitted(device, encoder);
	releasePending();
}
function packedPixels(bytes, width, height, rowBytes) {
	const pixels = new Uint32Array(width * height);
	for (let row = 0, pixel = 0; row < height; row++)
		for (let at = row * rowBytes; at < row * rowBytes + width * PIXEL_BYTES; at += PIXEL_BYTES)
			pixels[pixel++] = (OPAQUE | bytes[at] << 16 | bytes[at + 1] << 8 | bytes[at + 2]) >>> 0;
	return pixels;
}
async function gpuShow(job) {
	const device = await theGpuDevice();
	const set = setOf(device, job.id);
	if (job.canvas === undefined) {
		await finishPasses(device);
		closePassFrame();
		return 0;
	}
	const painted = paintCanvases.get(job.canvas);
	if (!painted) throw new Error(CANVAS_MISSING);
	const values = fieldValues(job.shader, job.width ?? set.width, job.height ?? set.height, job.values);
	const canvasImage = painted.context.getCurrentTexture();
	await drawInto(device, job.shader, values, set, canvasImage, painted.format);
	const encoder = takeEncoder(device);
	await gpuSubmitted(device, encoder);
	await answeredWithin(device.queue.onSubmittedWorkDone(), "drawn frame", GPU_WORK_WAIT_MS);
	closePassFrame();
	releasePending();
	return 1;
}
async function gpuPicture(job) {
	const device = await theGpuDevice();
	const set = setOf(device, job.id);
	const width = job.width ?? set.width, height = job.height ?? set.height;
	const values = fieldValues(job.shader, width, height, job.values);
	const image = device.createTexture({ size: [width, height], format: "rgba8unorm", usage: GPUTextureUsage.RENDER_ATTACHMENT | GPUTextureUsage.COPY_SRC });
	await drawInto(device, job.shader, values, set, image, "rgba8unorm");
	const encoder = takeEncoder(device);
	const rowBytes = Math.ceil(width * PIXEL_BYTES / ROW_ALIGNMENT) * ROW_ALIGNMENT;
	const readback = readbackBuffer(device, rowBytes * height);
	encoder.copyTextureToBuffer({ texture: image }, { buffer: readback, bytesPerRow: rowBytes, rowsPerImage: height }, [width, height]);
	const bytes = new Uint8Array(await gpuReadBack(device, encoder, readback));
	image.destroy();
	closePassFrame();
	releasePending();
	return packedPixels(bytes, width, height, rowBytes);
}
const GPU_JOBS = {
	gpu_compute: async job => ({ values: job.arrays ? plainArrays(await gpuComputedArrays(job)) : Array.from(await gpuComputedFloats(job)) }),
	// gpu_compute over GPU lists (changed in place): the arrays left as one run of words, in the order given
	gpu_arrays: async job => ({ words: joinedWords(Object.values(await gpuComputedArrays(job))) }),
	gpu_kernel: async job => ({ words: await gpuComputedFloats(job) }),
	gpu_render: async job => ({ words: await gpuRendered(job) }),
	gpu_fields: async job => ({ values: await gpuFields(job) }),
	gpu_pass: async job => ({ values: await gpuPass(job) }),
	gpu_show: async job => ({ values: await gpuShow(job) }),
	gpu_picture: async job => ({ words: await gpuPicture(job) }),
	gpu_fields_format: async () => ({ values: (await fieldsChoiceOf(await theGpuDevice())).name }),
	gpu_canvas: async job => ({ values: await gpuCanvas(job) }),
	gpu_paint: async job => ({ values: (await gpuRendered(job), job.canvas) }),
	gpu_paints: async () => ({ values: [await theGpuDevice().then(() => 1, failure => failureText(failure))] }),
};

// the system value "gpu_paints" (crates/warp-runtime host_words.rs GPU_PAINTS): whether a shader can paint, asked of the
// GPU's task Worker: a yes is kept for the page, a no only for the run (a slow first answer, a timed-out adapter, is asked
// again by the next run, and not on every frame of this one); without, paint(f, …) of a warp function paints f on the
// CPU, as the run's warning says once
const GPU_PAINTS = "gpu_paints"; // lib/prelude.warp gpu_paints()
const GPU_DRAWING = "gpu_drawing"; // lib/draw.warp: use draw on the GPU is native only (draw_shape below), so never here
const PAINTS_ON_CPU = "paint: a function paints on the CPU"; // src/graphics/paint.rs
let gpuPaintsHere = false;
function gpuPaints(holder) {
	if (gpuPaintsHere) return true;
	if (!holder.warnings.some(warning => warning.startsWith(PAINTS_ON_CPU))) {
		const [answer] = hasTaskWorkers() ? gpuJob("gpu_paints", {}) : ["only a cross-origin isolated page has task Workers"];
		gpuPaintsHere = answer === 1;
		if (!gpuPaintsHere) holder.warnings.push(`${PAINTS_ON_CPU}: ${answer}`);
	}
	return gpuPaintsHere;
}

const failureText = failure => failure === undefined || failure === null ? SILENT_FAILURE : String(failure.message ?? failure);

// a task Worker's job (task-worker.js): the shader's answer, or why not, into the shared buffer the program waits on
async function gpuJobInto({ gpu: { word, job }, shared }) {
	let answer;
	try {
		answer = await GPU_JOBS[word](job);
	} catch (failure) {
		answer = { error: failureText(failure) };
		await forgetDeviceIfLost().catch(() => {});
	}
	// a failure writing the answer is the job's, said to the program, which would otherwise wait out GPU_JOB_WAIT_MS
	try {
		if (answer.words) writeSharedWords(shared, answer.words);
		else writeShared(shared, answer);
	} catch (failure) {
		writeShared(shared, { error: failureText(failure) });
	}
}

// a typed array's items into the shared buffer as they are, their count after the state: a million floats as JSON
// took longer than the GPU
function writeSharedWords(shared, words) {
	if (TASK_HEADER + words.byteLength > shared.byteLength) shared.grow(TASK_HEADER + words.byteLength);
	new Uint8Array(shared, TASK_HEADER, words.byteLength).set(new Uint8Array(words.buffer, words.byteOffset, words.byteLength));
	const header = new Int32Array(shared, 0, 2);
	header[1] = words.length;
	Atomics.store(header, 0, WORDS_STATE);
	Atomics.notify(header, 0);
}

// the program's side: hand the job to a task Worker (by default the GPU's, gpuTaskWorker) and wait for the values it
// gives, raw words as `Words` (a copy: the shared buffer takes the next answer)
function gpuJob(word, job, transfer = [], Words = Float32Array, worker = hasTaskWorkers() && gpuTaskWorker()) {
	if (!hasTaskWorkers()) throw new Error(`${word} needs task Workers, which only a cross-origin isolated page has (the playground)`);
	if (!worker) throw new Error(`${word}: every task Worker is busy`);
	const shared = takeAnswerBuffer();
	worker.postMessage({ gpu: { word, job }, shared }, transfer);
	const header = new Int32Array(shared, 0, 2);
	// the buffer and the Worker stay out of their pools: the answer may still come
	if (Atomics.wait(header, 0, 0, GPU_JOB_WAIT_MS) === "timed-out") throw new Error(`${word}: the GPU's task Worker gave no answer in ${GPU_JOB_WAIT_MS / 1000} s`);
	const { values, error } = header[0] === WORDS_STATE ? { values: new Words(shared, TASK_HEADER, header[1]).slice() } : readShared(shared);
	taskPool.push(worker);
	returnAnswerBuffer(shared);
	// a failure naming its subject (the shader's) is said as it is, any other one names the job
	if (error) throw new Error(error.startsWith(SHADER_SUBJECT) ? error : `${word}: ${error}`);
	return values;
}

// the task Worker of the last GPU job, which keeps its buffers; else (busy with a task) another, which does from now on
function gpuTaskWorker() {
	const index = taskPool.indexOf(gpuWorker);
	const worker = index < 0 ? taskPool.pop() : taskPool.splice(index, 1)[0];
	if (worker) gpuWorker = worker;
	return worker;
}

addHostPart({
	systemValue: (value, holder) => value === GPU_PAINTS ? gpuPaints(holder) : value === GPU_DRAWING ? false : undefined,
	words: (holder, hooks, { program }) => {
		const plain = node => guestPlain(program(), node);
		// floats, also the whole ones (newGuestValue would make 3 an Int); ints of an array<i32> or array<u32>
		const numberList = (values, Elements) => Elements === Float32Array ? newGuestFloats(program(), values) : newGuestInts(program(), values);
		// as src/diagnostics/diagnostic.rs report_runtime_warning_once: the notice once a run, its detail after a colon
		const warnOnce = (notice, detail) => {
			if (!holder.warnings.some(warning => warning.startsWith(notice))) holder.warnings.push(`${notice}: ${detail}`);
		};
		const renderValues = (shader, width, height, values) =>
			({ ...builtinValues(shader, width, height), ...(values == null ? {} : plain(values)) });
		const renderJob = (shader, width, height, values) =>
			({ shader, width: Number(width), height: Number(height), values: renderValues(shader, Number(width), Number(height), values) });
		// the pixels as a Uint32Array, for gpu_render and for paint given a shader (host.js, P234)
		holder.gpuRendered = (shader, width, height, values) => gpuJob("gpu_render", renderJob(shader, width, height, values), [], Uint32Array);
		// paint of a shader in the playground: drawn into the page canvas the GPU's task Worker holds, a new one when it
		// holds none of this size (the first frame, or another task Worker took the GPU); the canvas's id. Where that
		// fails but the pixels render (the CI's SwiftShader drops its instance), the pixels, from then on, said once a run
		let paintCanvas;
		holder.gpuPainted = hooks.gpuCanvas && ((shader, width, height, values) => {
			const job = renderJob(shader, width, height, values);
			if (canvasFailure) {
				warnOnce(CANVAS_FAILED, canvasFailure);
				return holder.gpuRendered(shader, width, height, values);
			}
			try {
				return paintedInCanvas(job);
			} catch (failure) {
				const pixels = holder.gpuRendered(shader, width, height, values); // the shader's own error throws here
				canvasFailure = failure.message;
				warnOnce(CANVAS_FAILED, canvasFailure);
				return pixels;
			}
		});
		const paintedInCanvas = job => {
			for (const retry of [false, true]) {
				if (paintCanvas?.width !== job.width || paintCanvas?.height !== job.height) paintCanvas = newPaintCanvas(job.width, job.height, paintCanvas?.id);
				try {
					return gpuJob("gpu_paint", { ...job, canvas: paintCanvas.id });
				} catch (failure) {
					paintCanvas = undefined;
					if (retry || !failure.message.endsWith(CANVAS_MISSING)) throw failure;
				}
			}
		};
		// the page makes the canvas and sends it to the GPU's task Worker, named by the page's id for it, which configures it
		const newPaintCanvas = (width, height, replaces) => {
			const id = ++paintCanvasCount, worker = gpuTaskWorker();
			if (!worker) throw new Error("gpu_canvas: every task Worker is busy");
			hooks.gpuCanvas({ id, width, height, taskWorkerId: worker.taskWorkerId, replaces });
			gpuJob("gpu_canvas", { id, replaces }, [], Float32Array, worker);
			return { id, width, height };
		};
		// a GPU list [f32] of the program's (extensions/gpu_lists/runtime.warp): its cells where they are in the module's memory,
		// undefined for any other value
		const gpuCells = value => {
			const place = program().gpu_list_place ? program().gpu_list_place(value) : -1n;
			if (place < 0n) return undefined;
			const Cells = GPU_LIST_CELLS[Number(program().gpu_list_cell(value))];
			return new Cells(program().memory.buffer, Number(place >> 32n), Number(program().gpu_list_bytes(value)) / Cells.BYTES_PER_ELEMENT);
		};
		// gpu_compute of a map holding GPU lists: those changed where they are and given back themselves, the other
		// arrays as new lists, all bound and read back in one job without a text of them
		const computedInPlace = (code, entries, workgroups) => {
			const elements = Object.fromEntries(storageArrays(code).map(array => [array.name, array.Elements]));
			// a GPU list's cells as its type holds them (the shader's array type converts them, as natively), any other as declared
			const arrays = Object.fromEntries(entries.map(([name, value]) => [name, gpuCells(value)?.slice() ?? (elements[name] ?? Float32Array).from([plain(value)].flat().map(Number))]));
			const buffers = Object.values(arrays).map(array => array.buffer);
			// counted before the job: handing the arrays to the task Worker empties them here
			const lengths = Object.fromEntries(Object.entries(arrays).map(([name, array]) => [name, array.length]));
			const words = gpuJob("gpu_arrays", { shader: code, arrays, workgroups: Number(workgroups) }, buffers, Uint32Array);
			let at = 0;
			return newGuestMap(program(), entries.map(([name, value]) => {
				const length = lengths[name], Elements = elements[name] ?? Float32Array;
				const left = new Elements(words.buffer, words.byteOffset + at * WORD_BYTES, length);
				at += wordBytes(length * Elements.BYTES_PER_ELEMENT) / WORD_BYTES;
				const cells = gpuCells(value); // read anew: the module's memory may have grown meanwhile
				if (cells) cells.set(left);
				return [name, cells ? value : numberList(left, Elements)];
			}));
		};
		// an array<f16> takes only a GPU list [f16], whose cells it binds as they are, and a GPU list [f16] only one
		// (natively gpu.warp halves_unmatched)
		// a GPU list's cell type by name, undefined for any other value
		const gpuCellName = value => gpuCells(value) && GPU_CELL_NAMES[Number(program().gpu_list_cell(value))];
		// why an array cannot bind what it is given (natively gpu.warp cells_unfit): an array<f16> takes only a GPU list
		// [f16], whose cells it binds as they are, and a GPU list [f16] only one; a GPU list [bf16] has no WGSL array
		// type yet (card gpu-lists)
		const cellsChecked = (code, entries) => {
			const declared = storageArrays(code);
			for (const [name, value] of entries) {
				const array = declared.find(array => array.name === name), cell = gpuCellName(value);
				if (!array) continue;
				if (cell === "bf16") throw new Error(`the GPU list [bf16] ${name} has no WGSL array type to bind as yet`);
				if (cell?.startsWith("vec")) throw new Error(`the GPU list [${cell}] ${name} binds in paint's values; gpu_compute's arrays take lists of numbers`);
				if ((array.element === "f16") === (cell === "f16")) continue;
				throw new Error(array.element === "f16" ? `the shader's array<f16> ${name} takes a GPU list [f16]: make it as ${name}: [f16] = …` : `the GPU list [f16] ${name} needs the shader's array<f16>, not array<${array.element}>`);
			}
		};
		const computed = (shader, numbers, workgroups) => {
			const entries = guestEntries(program(), numbers);
			const code = plain(shader);
			cellsChecked(code, entries ?? [[storageArrays(code).find(array => array.binding === GPU_BINDING)?.name, numbers]]);
			if (entries?.some(([, value]) => gpuCells(value))) return computedInPlace(code, entries, workgroups);
			const cells = gpuCells(numbers);
			if (cells) {
				// changed in place, as natively (lib/host/native_host/gpu.warp array_left): the list itself is the value
				const copy = cells.slice();
				const left = gpuJob("gpu_kernel", { shader: code, numbers: copy, workgroups: Number(workgroups) }, [copy.buffer], gpuElements(code));
				gpuCells(numbers).set(left.subarray(0, cells.length)); // the module's memory may have grown meanwhile
				return numbers;
			}
			const given = plain(numbers);
			if (given !== null && typeof given === "object" && isPlainObject(given)) {
				const arrays = Object.fromEntries(Object.entries(given).map(([name, array]) => [name, [array].flat().map(Number)]));
				const left = gpuJob("gpu_compute", { shader: code, arrays, workgroups: Number(workgroups) });
				const elements = Object.fromEntries(storageArrays(code).map(array => [array.name, array.Elements]));
				return newGuestMap(program(), Object.entries(left).map(([name, values]) => [name, numberList(values, elements[name])]));
			}
			const values = gpuJob("gpu_compute", { shader: code, numbers: given.map(Number), workgroups: Number(workgroups) });
			return numberList(values, gpuElements(code));
		};
		const rendered = (shader, width, height, values) => {
			const pixels = holder.gpuRendered(plain(shader), width, height, values);
			return newGuestInts(program(), pixels);
		};
		// the field sets this program made, so a show knows the canvas size before the worker draws
		const fieldMeta = new Map();
		const metaOf = id => {
			const meta = fieldMeta.get(Number(id));
			if (!meta) throw new Error(`no fields ${id}`);
			return meta;
		};
		const plainValues = values => values == null ? {} : plain(values);
		const fieldsMade = (width, height, names) => {
			const made = gpuJob("gpu_fields", { width: Number(width), height: Number(height), names: [...plain(names)] });
			fieldMeta.set(made.id, made);
			return BigInt(made.id);
		};
		const passRecorded = (shader, fields, into, values) => {
			metaOf(fields);
			const recorded = gpuJob("gpu_pass", { shader: String(plain(shader)), id: Number(fields), into: String(plain(into)), values: plainValues(values) });
			return BigInt(recorded);
		};
		const fieldJob = (shader, fields, values) => {
			const meta = metaOf(fields);
			return { shader: String(plain(shader)), id: Number(fields), values: plainValues(values), width: meta.width, height: meta.height };
		};
		// the page canvas, as paint of a shader draws into it (paintedInCanvas): one canvas of this size
		const fieldShown = job => {
			for (const retry of [false, true]) {
				if (paintCanvas?.width !== job.width || paintCanvas?.height !== job.height) paintCanvas = newPaintCanvas(job.width, job.height, paintCanvas?.id);
				try {
					return gpuJob("gpu_show", { ...job, canvas: paintCanvas.id });
				} catch (failure) {
					paintCanvas = undefined;
					if (retry || !failure.message.endsWith(CANVAS_MISSING)) throw failure;
				}
			}
		};
		const shownAt = (job) => {
			if (hooks.gpuCanvas && !canvasFailure) {
				try { return BigInt(fieldShown(job)); }
				catch (failure) {
					canvasFailure = failure.message;
					warnOnce(CANVAS_FAILED, canvasFailure);
				}
			}
			return BigInt(gpuJob("gpu_show", job));
		};
		const shown = (shader, fields, values) => shownAt(fieldJob(shader, fields, values));
		const shownSized = (shader, fields, width, height, values) => {
			const job = fieldJob(shader, fields, values);
			job.width = Number(width);
			job.height = Number(height);
			return shownAt(job);
		};
		const pictured = (shader, fields, values) => newGuestInts(program(), gpuJob("gpu_picture", fieldJob(shader, fields, values), [], Uint32Array));
		const picturedSized = (shader, fields, width, height, values) => {
			const job = fieldJob(shader, fields, values);
			job.width = Number(width);
			job.height = Number(height);
			return newGuestInts(program(), gpuJob("gpu_picture", job, [], Uint32Array));
		};
		const fieldsFormat = () => newGuestValue(program(), gpuJob("gpu_fields_format", {}));
		// a WGSL error inside a `try` is the program's to catch, as every host word's failure (host.js raisedInProgram)
		return {
			gpu_computed: computed,
			gpu_render: rendered,
			gpu_fields: fieldsMade,
			gpu_pass_recorded: passRecorded,
			gpu_show: shown,
			gpu_picture: pictured,
			gpu_show_sized: shownSized,
			gpu_picture_sized: picturedSized,
			gpu_fields_format: fieldsFormat,
			// use draw on the GPU (src/graphics/gpu_canvas.rs) is native only: lib/draw.warp asks gpu_drawing first
			draw_shape: () => { throw new Error(NO_GPU_DRAWING); },
			draw_show: () => { throw new Error(NO_GPU_DRAWING); },
			draw_shader: () => { throw new Error(NO_GPU_DRAWING); },
		};
	},
});
