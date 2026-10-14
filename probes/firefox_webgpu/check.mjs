(async () => {
	const describe = async () => {
		const out = { gpu: !!self.navigator.gpu };
		try {
			const adapter = await self.navigator.gpu?.requestAdapter();
			out.adapter = !!adapter;
			out.info = adapter?.info ? `${adapter.info.vendor} ${adapter.info.architecture} ${adapter.info.description}` : undefined;
			const device = await adapter?.requestDevice();
			out.device = !!device;
			if (device) {
				const module = device.createShaderModule({ code: "@fragment fn main() -> @location(0) vec4f { return vec4f(1.0); } fn broken( {" });
				out.compilationInfo = typeof module.getCompilationInfo;
				try { out.messages = (await module.getCompilationInfo()).messages.map(m => `${m.type} ${m.lineNum}:${m.linePos} ${m.message}`); } catch (e) { out.compilationInfoError = String(e); }
				device.pushErrorScope("validation");
				device.createShaderModule({ code: "fn broken( {" });
				const scope = await device.popErrorScope();
				out.scope = scope ? `${scope.constructor.name}: ${scope.message}` : "none";
				if (self.OffscreenCanvas) {
					const context = new OffscreenCanvas(8, 8).getContext("webgpu");
					out.offscreenContext = !!context;
					try { context?.configure({ device, format: navigator.gpu.getPreferredCanvasFormat(), alphaMode: "opaque", usage: GPUTextureUsage.RENDER_ATTACHMENT | GPUTextureUsage.COPY_DST }); out.configured = true; } catch (e) { out.configureError = String(e); }
				}
			}
		} catch (e) { out.error = `${e?.constructor?.name}: ${e?.message ?? e}`; }
		return out;
	};
	const page = await describe();
	const source = `const describe = ${describe}; describe().then(r => postMessage(r), e => postMessage({ workerError: String(e) }));`;
	const worker = new Worker(URL.createObjectURL(new Blob([source], { type: "text/javascript" })));
	const inWorker = await new Promise(done => { worker.onmessage = m => done(m.data); worker.onerror = e => done({ onerror: e.message }); setTimeout(() => done({ timeout: true }), 20000); });
	return { userAgent: navigator.userAgent, isolated: self.crossOriginIsolated, page, worker: inWorker };
})()
