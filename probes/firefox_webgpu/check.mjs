(async () => {
	const describe = async () => {
		const out = { gpu: !!self.navigator.gpu, steps: [] };
		const thrown = value => value === undefined ? "undefined" : value === null ? "null" : `${value?.constructor?.name}: ${value?.message ?? value}`;
		const step = async (name, run) => {
			try {
				const value = await run();
				out.steps.push(`${name}: ok ${typeof value === "object" ? (value === null ? "null" : value.constructor?.name) : String(value)}`);
				return value;
			} catch (failure) {
				out.steps.push(`${name}: THREW ${thrown(failure)} ${failure?.stack ?? ""}`);
			}
		};
		const adapter = await step("requestAdapter", () => self.navigator.gpu?.requestAdapter());
		if (!adapter) return out;
		await step("info", () => `${adapter.info?.vendor} ${adapter.info?.architecture} ${adapter.info?.description}`);
		const device = await step("requestDevice", () => adapter.requestDevice());
		if (!device) return out;
		await step("format", () => navigator.gpu.getPreferredCanvasFormat());
		const good = "@vertex fn v(@builtin(vertex_index) i: u32) -> @builtin(position) vec4f { return vec4f(0.0); }\n@fragment fn main() -> @location(0) vec4f { return vec4f(1.0); }";
		const module = await step("createShaderModule", () => device.createShaderModule({ code: good }));
		await step("getCompilationInfo good", async () => (await module.getCompilationInfo()).messages.length);
		const bad = device.createShaderModule({ code: "fn broken( {" });
		await step("getCompilationInfo bad", async () => (await bad.getCompilationInfo()).messages.map(m => `${m.type} ${m.lineNum}:${m.linePos} ${m.message}`).join(" | "));
		await step("pushErrorScope", () => device.pushErrorScope("validation"));
		const pipeline = await step("createRenderPipeline", () => device.createRenderPipeline({ layout: "auto", vertex: { module, entryPoint: "v" }, fragment: { module, entryPoint: "main", targets: [{ format: "rgba8unorm" }] } }));
		await step("popErrorScope", async () => thrown(await device.popErrorScope()));
		const texture = await step("createTexture", () => device.createTexture({ size: [4, 4], format: "rgba8unorm", usage: GPUTextureUsage.RENDER_ATTACHMENT | GPUTextureUsage.COPY_SRC }));
		const readback = await step("createBuffer", () => device.createBuffer({ size: 256 * 4, usage: GPUBufferUsage.MAP_READ | GPUBufferUsage.COPY_DST }));
		await step("render+readback", async () => {
			const encoder = device.createCommandEncoder();
			const pass = encoder.beginRenderPass({ colorAttachments: [{ view: texture.createView(), loadOp: "clear", storeOp: "store" }] });
			pass.setPipeline(pipeline);
			pass.draw(3);
			pass.end();
			encoder.copyTextureToBuffer({ texture }, { buffer: readback, bytesPerRow: 256, rowsPerImage: 4 }, [4, 4]);
			device.queue.submit([encoder.finish()]);
			await readback.mapAsync(GPUMapMode.READ);
			return new Uint8Array(readback.getMappedRange().slice(0)).length;
		});
		if (self.OffscreenCanvas) {
			const context = await step("offscreen getContext", () => new OffscreenCanvas(8, 8).getContext("webgpu"));
			if (context) await step("configure", () => context.configure({ device, format: navigator.gpu.getPreferredCanvasFormat(), alphaMode: "opaque", usage: GPUTextureUsage.RENDER_ATTACHMENT | GPUTextureUsage.COPY_DST }));
		}
		return out;
	};
	const page = await describe();
	const source = `const describe = ${describe}; describe().then(r => postMessage(r), e => postMessage({ workerError: String(e) }));`;
	const worker = new Worker(URL.createObjectURL(new Blob([source], { type: "text/javascript" })));
	const inWorker = await new Promise(done => { worker.onmessage = m => done(m.data); worker.onerror = e => done({ onerror: e.message }); setTimeout(() => done({ timeout: true }), 20000); });
	return JSON.stringify({ userAgent: navigator.userAgent, isolated: self.crossOriginIsolated, page, worker: inWorker }, null, 1);
})()
