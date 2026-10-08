// WebGPU (a part of host.js, which says how parts work; card web-apis, notes/web_framework.md "web-apis: WebGPU"):
// gpu_compute(shader, numbers, workgroups) runs a WGSL compute shader whose entry point `main` reads and writes the
// numbers as array<f32> at @group(0) @binding(0), dispatched over `workgroups` workgroups, and gives back the numbers it
// left. WebGPU only answers asynchronously, so a task Worker (host-tasks.js) does the GPU work while the program's
// worker waits for its answer in shared memory, as a task's result.

const GPU_ENTRY_POINT = "main";
const GPU_BINDING = 0;
let gpuDevice; // the task Worker's device, asked for once

// the task Worker's side: {values} the shader left, or {error} (no WebGPU, a shader that does not compile, …)
async function gpuComputed(shader, numbers, workgroups) {
	try {
		gpuDevice ??= await gpuDeviceOrFailure();
		const device = gpuDevice;
		const input = new Float32Array(numbers);
		const usage = GPUBufferUsage;
		const storage = device.createBuffer({ size: input.byteLength, usage: usage.STORAGE | usage.COPY_SRC | usage.COPY_DST });
		const readback = device.createBuffer({ size: input.byteLength, usage: usage.MAP_READ | usage.COPY_DST });
		device.queue.writeBuffer(storage, 0, input);
		const module = device.createShaderModule({ code: shader });
		const problems = (await module.getCompilationInfo()).messages.filter(message => message.type === "error");
		if (problems.length) return { error: problems.map(problem => `${problem.lineNum}:${problem.linePos}: ${problem.message}`).join("; ") };
		device.pushErrorScope("validation");
		const pipeline = device.createComputePipeline({ layout: "auto", compute: { module, entryPoint: GPU_ENTRY_POINT } });
		const bindings = device.createBindGroup({ layout: pipeline.getBindGroupLayout(0), entries: [{ binding: GPU_BINDING, resource: { buffer: storage } }] });
		const encoder = device.createCommandEncoder();
		const pass = encoder.beginComputePass();
		pass.setPipeline(pipeline);
		pass.setBindGroup(0, bindings);
		pass.dispatchWorkgroups(workgroups);
		pass.end();
		encoder.copyBufferToBuffer(storage, 0, readback, 0, input.byteLength);
		device.queue.submit([encoder.finish()]);
		const invalid = await device.popErrorScope();
		if (invalid) return { error: invalid.message };
		await readback.mapAsync(GPUMapMode.READ);
		const values = Array.from(new Float32Array(readback.getMappedRange().slice(0)));
		storage.destroy();
		readback.destroy();
		return { values };
	} catch (failure) {
		return { error: String(failure.message ?? failure) };
	}
}

async function gpuDeviceOrFailure() {
	const adapter = await self.navigator.gpu?.requestAdapter();
	if (!adapter) throw new Error("this browser offers no WebGPU adapter");
	return adapter.requestDevice();
}

// a task Worker's job (task-worker.js): the shader's answer into the shared buffer the program waits on
async function gpuComputeInto({ gpu: { shader, numbers, workgroups }, shared }) {
	writeShared(shared, await gpuComputed(shader, numbers, workgroups));
}

// the program's side: hand the job to a task Worker and wait for its answer
function gpuCompute(shader, numbers, workgroups) {
	if (!hasTaskWorkers()) throw new Error("gpu_compute needs task Workers, which only a cross-origin isolated page has (the playground)");
	const worker = taskPool.pop();
	if (!worker) throw new Error("gpu_compute: every task Worker is busy");
	const shared = new SharedArrayBuffer(TASK_HEADER + TASK_RESULT_BYTES, { maxByteLength: TASK_RESULT_LIMIT });
	worker.postMessage({ gpu: { shader, numbers, workgroups }, shared });
	const { values, error } = readShared(shared, true);
	taskPool.push(worker);
	if (error) throw new Error(`gpu_compute: ${error}`);
	return values;
}

// the f64 cells of a linear array: its block is [count: i64][count cells] (src/wasm_emitter/linear_arrays.rs)
function linearCells(memory, block) {
	const count = Number(new DataView(memory.buffer).getBigInt64(Number(block), true));
	return new Float64Array(memory.buffer, Number(block) + Float64Array.BYTES_PER_ELEMENT, count);
}

addHostPart({
	words: (holder, hooks, { program }) => {
		const plain = node => plainOfTree(readNode(program(), node));
		return {
			gpu_compute: (shader, numbers, workgroups) => {
				const values = gpuCompute(plain(shader), plain(numbers).map(Number), Number(workgroups));
				// floats, also the whole ones (treeOfPlain would make 3 an Int)
				return buildValue(program(), { kind: SQUARE_LIST, items: values.map(float => ({ kind: KIND_FLOAT, data: { float }, chain: [] })) });
			},
			// over a `linear xs = float[n]`: its cells read and written in place, no list built (card gpu-vectors)
			gpu_compute_linear: (shader, block, workgroups) => {
				const cells = linearCells(program().memory, block);
				cells.set(gpuCompute(plain(shader), cells.slice(), Number(workgroups))); // a copy: a view would post all of memory
				return block;
			},
		};
	},
});
