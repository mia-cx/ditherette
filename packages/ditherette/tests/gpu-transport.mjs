/** Synthetic WebGPU transport only. The package's planner, tables and Wasm remain real. */
export function installGpuTransport() {
	const resources = new Set();
	const devices = [];
	const observations = {
		devices: 0,
		handlers: 0,
		lossHandlers: 0,
		destroyed: 0,
		externalUploads: 0
	};
	const limits = new Proxy(
		{
			maxTextureDimension2D: 16384,
			maxStorageBufferBindingSize: 128 * 1024 ** 2,
			maxBufferSize: 256 * 1024 ** 2,
			maxComputeWorkgroupsPerDimension: 65535,
			minUniformBufferOffsetAlignment: 256,
			minStorageBufferOffsetAlignment: 256
		},
		{ get: (target, key) => target[key] ?? 256 }
	);
	class GPUDeviceLostInfo {
		reason = 'destroyed';
		message = 'synthetic destroy';
	}
	class GPUAdapter {
		info = { description: 'synthetic Node transport' };
		limits = limits;
		features = new Set();
		async requestDevice() {
			return new Device();
		}
	}
	class Buffer {
		label = '';
		constructor(descriptor) {
			Object.assign(this, descriptor);
			this.data = new ArrayBuffer(descriptor.size);
			resources.add(this);
		}
		async mapAsync() {}
		getMappedRange() {
			return this.data;
		}
		unmap() {}
		destroy() {
			resources.delete(this);
		}
	}
	class Texture {
		label = '';
		constructor(descriptor) {
			Object.assign(this, descriptor.size);
			this.data = new Uint8Array(this.width * this.height * 4);
			resources.add(this);
		}
		createView() {
			return { label: '', texture: this };
		}
		destroy() {
			resources.delete(this);
		}
	}
	class Device {
		label = 'ditherette';
		limits = limits;
		lost = new Promise((resolve) => {
			this.resolveLost = resolve;
		});
		constructor() {
			devices.push(this);
			observations.devices++;
			this.lost.then = (...args) => {
				observations.lossHandlers++;
				return Promise.prototype.then.call(this.lost, ...args);
			};
		}
		set onuncapturederror(value) {
			observations.handlers++;
			this.errorHandler = value;
		}
		pushErrorScope() {}
		async popErrorScope() {
			return null;
		}
		createBuffer(descriptor) {
			return new Buffer(descriptor);
		}
		createTexture(descriptor) {
			return new Texture(descriptor);
		}
		createShaderModule(descriptor) {
			return { ...descriptor, label: descriptor.label ?? '' };
		}
		createComputePipeline() {
			return { label: '', getBindGroupLayout: () => ({ label: '' }) };
		}
		createBindGroup(descriptor) {
			return { ...descriptor, label: descriptor.label ?? '' };
		}
		createCommandEncoder() {
			const operations = [];
			return {
				label: '',
				beginComputePass() {
					let group;
					return {
						label: '',
						setPipeline() {},
						setBindGroup(_index, value) {
							group = value;
						},
						dispatchWorkgroups() {
							operations.push(() => {
								const input = group.entries[0].resource.texture.data;
								const output = new Uint8Array(group.entries[1].resource.buffer.data);
								const table = new Uint32Array(group.entries[2].resource.buffer.data);
								for (let i = 0; i < input.length; i++)
									output[i] = i % 4 === 3 ? input[i] : table[(i % 4) * 256 + input[i]];
							});
						},
						end() {}
					};
				},
				copyBufferToBuffer(source, sourceOffset, target, targetOffset, size) {
					operations.push(() =>
						new Uint8Array(target.data, targetOffset, size).set(
							new Uint8Array(source.data, sourceOffset, size)
						)
					);
				},
				finish() {
					return operations;
				}
			};
		}
		queue = {
			label: '',
			copyExternalImageToTexture() {
				observations.externalUploads++;
				throw new DOMException('The source image is not origin clean.', 'SecurityError');
			},
			writeTexture({ texture }, bytes) {
				texture.data.set(bytes);
			},
			writeBuffer(buffer, offset, bytes, dataOffset, size) {
				const input = ArrayBuffer.isView(bytes)
					? new Uint8Array(bytes.buffer, bytes.byteOffset + dataOffset, size)
					: new Uint8Array(bytes, dataOffset, size);
				new Uint8Array(buffer.data, offset, size).set(input);
			},
			submit(commands) {
				for (const operations of commands) for (const operation of operations) operation();
			}
		};
		destroy() {
			observations.destroyed++;
			const at = devices.indexOf(this);
			if (at >= 0) devices.splice(at, 1);
			this.resolveLost(new GPUDeviceLostInfo());
		}
	}
	Object.assign(globalThis, {
		GPUAdapter,
		GPUDeviceLostInfo,
		Window: class Window {},
		window: globalThis
	});
	Object.defineProperty(globalThis.navigator, 'gpu', {
		value: { requestAdapter: async () => new GPUAdapter() },
		configurable: true
	});

	return { resources, observations, devices };
}
