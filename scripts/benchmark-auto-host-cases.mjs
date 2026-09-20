const half = { width: 769, height: 513 };
const medium = { width: 65, height: 49 };
const small = { width: 33, height: 25 };
const palette = (count) =>
	Array.from({ length: count }, (_, n) => ({
		kind: 'color',
		rgb: [n, (n * 73) % 256, (n * 151) % 256]
	}));
const quantize = (count, matching = 'srgb-euclidean') => ({
	palette: [...palette(count - 1), { kind: 'transparent' }],
	alpha: { mode: 'preserve', threshold: 127.5 },
	matching
});
const random = {
	field: { algorithm: 'random', seed: 0x12345678 },
	space: 'srgb',
	strength: 0.7,
	placement: { mode: 'everywhere' }
};
const oklab = {
	field: { algorithm: 'blue-noise' },
	space: 'oklab',
	strength: 0.7,
	placement: { mode: 'adaptive', radius: 2, threshold: 0.05, softness: 0.025 }
};
const mixing = {
	family: 'yliluoma',
	size: '4',
	placement: { mode: 'adaptive', radius: 2, threshold: 5, softness: 10 }
};

/** Fixed measured recipe classes that enter the ordinary S35/S36/S37 selectors. */
export const automaticCases = [
	...['area', 'bilinear', 'lanczos3'].map((filter) => ({
		name: filter,
		source: filter === 'lanczos3' ? { width: 2048, height: 1536 } : { width: 1537, height: 1025 },
		output: filter === 'lanczos3' ? { width: 512, height: 384 } : half,
		operation: {
			operation: `resize-${filter}`,
			...(filter === 'area' ? {} : { anchor: 'center' }),
			...(filter === 'lanczos3' ? { support: 'scale-aware' } : {})
		},
		stages: ['resize']
	})),
	{
		name: 'srgb-quantize',
		source: half,
		output: half,
		operation: { operation: 'quantize', settings: quantize(16) },
		stages: ['quantize']
	},
	{
		name: 'srgb-random-perturb',
		source: half,
		output: half,
		operation: { operation: 'perturb', settings: random },
		stages: ['perturb']
	},
	{
		name: 'srgb-random-separable',
		source: half,
		output: half,
		operation: { operation: 'separable', settings: { quantize: quantize(16), perturb: random } },
		stages: ['perturb', 'quantize']
	},
	{
		name: 'oklab-adaptive-composition',
		source: medium,
		output: medium,
		operation: {
			operation: 'separable',
			settings: { quantize: quantize(64, 'oklab-euclidean'), perturb: oklab }
		},
		stages: ['perturb', 'quantize'],
		composition: true
	},
	{
		name: 'yliluoma-medium',
		source: medium,
		output: medium,
		operation: {
			operation: 'yliluoma',
			settings: {
				quantize: {
					palette: palette(8),
					alpha: { mode: 'premultiplied' },
					matching: 'srgb-euclidean'
				},
				size: mixing.size,
				placement: mixing.placement
			}
		},
		stages: ['dither-and-quantize']
	},
	{
		name: 'nearest-oklch-process',
		source: medium,
		output: small,
		operation: {
			operation: 'process',
			settings: {
				palette: palette(4),
				recipe: {
					version: 1,
					output: { ...small, resize: { algorithm: 'nearest', anchor: 'center' } },
					alpha: { mode: 'premultiplied' },
					match: 'oklch-circular-hue',
					dither: mixing
				}
			}
		},
		stages: ['resize', 'dither-and-quantize']
	}
];

/** Build one role without developer scheduling controls or measurement collection. */
export function automaticTrial(assets, fixture, role) {
	const rgba = Array.from({ length: fixture.source.width * fixture.source.height * 4 }, (_, n) => {
		const channel = n % 4;
		if (channel === 3) return 255;
		const pixel = Math.floor(n / 4);
		const x = pixel % fixture.source.width;
		const y = Math.floor(pixel / fixture.source.width);
		return (
			(channel === 0 ? x * 17 + y * 31 : channel === 1 ? x * 43 + y * 7 : x * 11 + y * 53) % 256
		);
	});
	return {
		role,
		pair: 0,
		browser: { assets },
		composition: fixture.composition === true,
		case: {
			name: `untimed-auto-${fixture.name}`,
			source: fixture.source,
			rgba,
			identity: { input: Array(32).fill(1), settings: Array(32).fill(2), output: fixture.output },
			measurement: {
				scope: 'complete-call',
				mode: 'single-call',
				application_cache: 'not-applicable',
				samples: 1
			},
			browser: {
				execution: 'host-worker',
				preparation: 'primed-instance',
				cache: 'none',
				accepted: 'package',
				candidate: 'package',
				threads: { accepted: 'disabled', candidate: 'required' },
				operation: fixture.operation
			}
		}
	};
}
