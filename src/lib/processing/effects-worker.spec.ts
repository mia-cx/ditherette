import { readFile } from 'node:fs/promises';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import {
	createDitherette,
	createGpuEffects,
	applyCompiledEffects,
	indexColours,
	type Effect,
	type GpuEffects
} from 'ditherette';
import { installGpuTransport } from '../../../packages/ditherette/tests/gpu-transport.mjs';
import type { LiveEffectsRequest, LiveEffectsResponse } from './live-effects';
import { EffectsWorkerPipeline } from './worker-pipeline';

vi.mock('ditherette', async (original) => ({
	...(await original<typeof import('ditherette')>()),
	createGpuEffects: vi.fn(),
	createDitherette: vi.fn()
}));
const actual = await vi.importActual<typeof import('ditherette')>('ditherette');
const wasm = await WebAssembly.compile(
	await readFile(
		new URL(
			'../../../packages/ditherette/dist/wasm/scalar/ditherette_wasm_bg.wasm',
			import.meta.url
		)
	)
);
const gpuWasm = await readFile(
	new URL('../../../packages/ditherette/dist/wasm/gpu/ditherette_wasm_bg.wasm', import.meta.url)
)
	.then((bytes) => WebAssembly.compile(bytes))
	.catch((error: unknown) => {
		if (error instanceof Error && 'code' in error && error.code === 'ENOENT') return undefined;
		throw error;
	});
let facades: GpuEffects[];
let transport: ReturnType<typeof installGpuTransport>;
let pipeline: EffectsWorkerPipeline;
let replies: LiveEffectsResponse[];

const source = () =>
	({
		width: 257,
		height: 17,
		data: Uint8ClampedArray.from({ length: 257 * 17 * 4 }, (_, i) => (i * 61 + (i >> 3)) & 255)
	}) as ImageData;
const tone: Effect[] = [
	{
		effect: 'levels',
		enabled: true,
		channel: 'red',
		input: { black: 0.05, white: 0.9 },
		gamma: 1.3,
		output: { black: 0.02, white: 1 }
	},
	{ effect: 'brightness-contrast', enabled: true, brightness: -0.11, contrast: 0.32 },
	{ effect: 'exposure', enabled: true, stops: 0.6 },
	{ effect: 'white-balance', enabled: true, temperature: -0.2, tint: 0.4 }
];
const mixed: Effect = {
	effect: 'hue-saturation',
	enabled: true,
	hue: 30,
	saturation: 0.2,
	lightness: 0
};
const compile = (
	id = 1,
	effects = tone,
	sourceId = 1
): Extract<LiveEffectsRequest, { type: 'compile' }> => ({
	type: 'compile',
	id,
	sourceId,
	key: String(id),
	effects,
	context: { palette: [], space: 'oklab' }
});

beforeEach(() => {
	vi.stubGlobal('navigator', {});
	for (const key of ['Window', 'window', 'GPUAdapter', 'GPUDeviceLostInfo'])
		vi.stubGlobal(key, undefined);
	transport = installGpuTransport();
	facades = [];
	replies = [];
	vi.mocked(createDitherette).mockImplementation((options) =>
		actual.createDitherette({ ...options, wasm })
	);
	vi.mocked(createGpuEffects).mockImplementation(async (options) => {
		const facade = await actual.createGpuEffects({ ...options, wasm, gpuWasm });
		facades.push(facade);
		return facade;
	});
	pipeline = new EffectsWorkerPipeline((reply, transfer = []) => {
		// Exercise independent, transferred ownership at the real worker boundary.
		replies.push(structuredClone(reply, { transfer }));
	});
});
afterEach(async () => {
	await pipeline.dispose();
	vi.unstubAllGlobals();
	vi.clearAllMocks();
});

// Ordinary CPU package builds intentionally omit GPU assets. `vite build` and
// the package's explicit GPU build enable these real-artifact transport proofs.
describe.skipIf(!gpuWasm)('shared Source and processing effects', () => {
	it('dispatches exact whole-chain tables, keeps original alpha and reuses one bounded device', async () => {
		const image = source();
		await pipeline.handle({ type: 'source', sourceId: 1, source: image });
		const cpu = await actual.createDitherette({ wasm });
		const index = indexColours({ ...image, data: new Uint8Array(image.data.buffer) });
		const curves: Effect = {
			effect: 'curves',
			enabled: true,
			curves: [
				{
					kind: 'remap',
					x: { model: 'linear-rgb', channel: 'green' },
					y: { model: 'linear-rgb', channel: 'green' },
					points: [
						[0, 0.02],
						[0.5, 0.7],
						[1, 0.98]
					]
				}
			]
		};
		try {
			for (let id = 1; id <= 24; id++) {
				const effects = id % 2 ? tone : [...tone, curves, { ...mixed, enabled: false }];
				await pipeline.handle(compile(id, effects));
				const result = replies.at(-1)!;
				expect(result.type).toBe('compiled');
				if (result.type !== 'compiled') throw Error('Compile failed');
				const mapped = applyCompiledEffects(
					{ ...image, data: new Uint8Array(image.data.buffer) },
					index.indices,
					result.results
				);
				expect(mapped.data).toEqual(
					cpu.applyEffects({
						version: 1,
						source: { ...image, data: new Uint8Array(image.data.buffer) },
						effects
					}).data
				);
				expect(facades[0]!.execution?.backend).toBe('gpu');
				expect(transport.resources.size).toBe(0);
			}
		} finally {
			cpu.dispose();
		}
		expect(createGpuEffects).toHaveBeenCalledOnce();
		expect(createDitherette).not.toHaveBeenCalled();
		expect(transport.observations).toMatchObject({
			devices: 1,
			handlers: 1,
			lossHandlers: 1,
			externalUploads: 0
		});
		await pipeline.dispose();
		expect(transport.observations.destroyed).toBe(1);
	});

	it.each(['mixed', 'masked', 'no-webgpu', 'no-adapter', 'device-failure', 'device-loss'])(
		'preserves CPU output after %s and never recreates retired bindings',
		async (route) => {
			const image = source();
			await pipeline.handle({ type: 'source', sourceId: 1, source: image });
			if (route === 'no-webgpu') vi.stubGlobal('navigator', {});
			if (route === 'no-adapter')
				Object.defineProperty(navigator, 'gpu', { value: { requestAdapter: async () => null } });
			if (route === 'device-failure') {
				const adapter = await (
					navigator as Navigator & {
						gpu: { requestAdapter(): Promise<{ requestDevice(): Promise<unknown> }> };
					}
				).gpu.requestAdapter();
				Object.getPrototypeOf(adapter).requestDevice = async () => {
					throw Error('Injected device failure');
				};
			}
			if (route === 'device-loss') {
				await pipeline.handle(compile());
				transport.devices[0]!.destroy();
				await new Promise((resolve) => setTimeout(resolve, 0));
			}
			const effects =
				route === 'mixed'
					? [...tone, mixed]
					: route === 'masked'
						? [
								{
									...tone[0]!,
									mask: [
										{
											x: { model: 'srgb' as const, channel: 'red' as const },
											points: [
												[0, 0],
												[1, 1]
											] as const
										}
									]
								}
							]
						: tone;
			await pipeline.handle(compile(2, effects));
			const result = replies.at(-1)!;
			expect(result.type).toBe('compiled');
			if (result.type !== 'compiled') throw Error('Compile failed');
			const cpu = await actual.createDitherette({ wasm });
			try {
				const input = { ...image, data: new Uint8Array(image.data.buffer) };
				const index = indexColours(input);
				expect(applyCompiledEffects(input, index.indices, result.results).data).toEqual(
					cpu.applyEffects({ version: 1, source: input, effects }).data
				);
			} finally {
				cpu.dispose();
			}
			expect(facades[0]!.execution?.backend).toBe('cpu');
			const deviceCount = transport.observations.devices;
			await pipeline.handle(compile(3));
			expect(facades[0]!.execution?.backend).toBe('cpu');
			expect(transport.observations.devices).toBe(deviceCount);
			expect(transport.resources.size).toBe(0);
		}
	);

	it.each(['cancel', 'source', 'dispose'])(
		'drops delayed readback after %s without overlapping package calls',
		async (action) => {
			let applying = 0;
			let concurrent = 0;
			let release!: () => void;
			const gate = new Promise<void>((resolve) => {
				release = resolve;
			});
			let started!: () => void;
			const waiting = new Promise<void>((resolve) => {
				started = resolve;
			});
			const create = vi.mocked(createGpuEffects).getMockImplementation()!;
			vi.mocked(createGpuEffects).mockImplementation(async (options) => {
				const facade = await create(options);
				const apply = facade.applyEffects.bind(facade);
				facade.applyEffects = async (request) => {
					concurrent = Math.max(concurrent, ++applying);
					try {
						const result = await apply(request);
						started();
						await gate;
						return result;
					} finally {
						applying--;
					}
				};
				return facade;
			});
			await pipeline.handle({ type: 'source', sourceId: 1, source: source() });
			const first = pipeline.handle(compile());
			await waiting;
			if (action === 'cancel') await pipeline.handle({ type: 'cancel', id: 1 });
			if (action === 'source')
				await pipeline.handle({ type: 'source', sourceId: 2, source: source() });
			const finishing =
				action === 'dispose'
					? pipeline.dispose()
					: pipeline.handle(compile(2, tone, action === 'source' ? 2 : 1));
			expect(replies.some((reply) => reply.type === 'compiled')).toBe(false);
			expect(transport.observations.devices).toBe(1);
			release();
			await Promise.all([first, finishing]);
			expect(concurrent).toBe(1);
			expect(replies.flatMap((reply) => (reply.type === 'compiled' ? [reply.id] : []))).toEqual(
				action === 'dispose' ? [] : [2]
			);
			expect(transport.resources.size).toBe(0);
		}
	);
});
