import { atom, computed, type ReadableAtom } from 'nanostores';
import { activeEffectSteps } from '$lib/stores/effects';
import { compiledEffects, compiledMask, effectsIndex, shownMaskKey } from './live-effects';

/** Where the source image sits in the canvas, in CSS pixels. */
export type Placement = { left: number; top: number; width: number; height: number };

/** The source centred and scaled to fit a canvas of this CSS size, like `object-fit: contain`. */
export function containPlacement(
	source: { width: number; height: number },
	width: number,
	height: number
): Placement {
	const scale = Math.min(width / source.width, height / source.height);
	const [fitWidth, fitHeight] = [source.width * scale, source.height * scale];
	return {
		left: (width - fitWidth) / 2,
		top: (height - fitHeight) / 2,
		width: fitWidth,
		height: fitHeight
	};
}

/** Results are laid out in rows of this many texels, or fewer where the GPU's textures are narrower. */
const RESULTS_ROW = 4096;

/**
 * What to draw: the source's colour index and the latest compiled results for it, while any effect
 * is on. Results for an older source never pair with a newer index.
 */
export const shownEffects = computed(
	[effectsIndex, compiledEffects, activeEffectSteps],
	(index, compiled, steps) =>
		steps.length && index && compiled?.source === index.source
			? { index, results: compiled.results }
			: undefined
);
type Shown = NonNullable<ReturnType<typeof shownEffects.get>>;

/**
 * What the Source preview draws: the shown mask's greys while a step's mask is shown, otherwise
 * the effects. Greys compiled for another mask never stand in for the shown one.
 */
export const shownSource = computed(
	[shownEffects, effectsIndex, compiledMask, shownMaskKey],
	(effects, index, mask, key): Shown | undefined => {
		if (key === undefined) return effects;
		return index && mask?.source === index.source && mask.key === key
			? { index, results: mask.results }
			: undefined;
	}
);

/** Whether a view is showing the source through the effects. */
export const effectsDrawn = atom(false);

/** Whether this browser can draw effects on the source at all. Checked once. */
export const supportsWebGL2 = (() => {
	let supported: boolean | undefined;
	return () => (supported ??= document.createElement('canvas').getContext('webgl2') !== null);
})();

const VERTEX = `#version 300 es
in vec2 corner;
void main() {
	gl_Position = vec4(corner * 2.0 - 1.0, 0.0, 1.0);
}`;

/**
 * Each canvas pixel covers a footprint of source pixels. Zoomed in, it shows the one under its
 * centre. Zoomed out, it averages them in premultiplied alpha, each weighted by the area it covers.
 * Every source pixel takes its colour's result, so the average is exact while a footprint spans up
 * to 16 pixels per axis. Past that it splits the footprint into at most 17 runs per axis and samples
 * one pixel per run, weighted by the run's area.
 */
const FRAGMENT = `#version 300 es
precision highp float;
precision highp int;
uniform highp sampler2D indexTexture;
uniform highp sampler2D results;
uniform int resultsRow;
uniform vec4 placement;
uniform vec2 sourceSize;
uniform float canvasHeight;
out vec4 color;
// A footprint 16 pixels wide touches up to 17.
const int MAX_TAPS = 17;

vec4 effected(ivec2 texel) {
	vec4 entry = texelFetch(indexTexture, texel, 0);
	ivec3 bytes = ivec3(round(entry.rgb * 255.0));
	int index = bytes.r + bytes.g * 256 + bytes.b * 65536;
	return vec4(texelFetch(results, ivec2(index % resultsRow, index / resultsRow), 0).rgb, entry.a);
}

void main() {
	vec2 device = vec2(gl_FragCoord.x, canvasHeight - gl_FragCoord.y);
	vec2 scale = sourceSize / placement.zw;
	vec2 low = (device - 0.5 - placement.xy) * scale;
	vec2 high = low + scale;
	if (high.x <= 0.0 || high.y <= 0.0 || low.x >= sourceSize.x || low.y >= sourceSize.y) discard;
	if (scale.x <= 1.0 && scale.y <= 1.0) {
		vec2 centre = (device - placement.xy) * scale;
		if (centre.x < 0.0 || centre.y < 0.0 || centre.x >= sourceSize.x || centre.y >= sourceSize.y) discard;
		color = effected(ivec2(centre));
		return;
	}
	vec2 start = max(low, vec2(0.0));
	vec2 end = min(high, sourceSize);
	ivec2 first = ivec2(floor(start));
	ivec2 last = ivec2(ceil(end)) - 1;
	ivec2 stride = max(ivec2(1), (last - first + MAX_TAPS) / MAX_TAPS);
	vec4 sum = vec4(0.0);
	for (int y = first.y; y <= last.y; y += stride.y) {
		float height = min(float(y + stride.y), end.y) - max(float(y), start.y);
		for (int x = first.x; x <= last.x; x += stride.x) {
			float width = min(float(x + stride.x), end.x) - max(float(x), start.x);
			vec4 pixel = effected(ivec2(x, y));
			sum += vec4(pixel.rgb * pixel.a, pixel.a) * width * height;
		}
	}
	// Over the whole footprint, so the image's edges blend out like any scaled image.
	color = sum.a > 0.0 ? vec4(sum.rgb / sum.a, sum.a / (scale.x * scale.y)) : vec4(0.0);
}`;

function compile(gl: WebGL2RenderingContext, type: number, text: string) {
	const shader = gl.createShader(type)!;
	gl.shaderSource(shader, text);
	gl.compileShader(shader);
	if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS))
		throw new Error(gl.getShaderInfoLog(shader) ?? 'Shader failed to compile.');
	return shader;
}

function texture(gl: WebGL2RenderingContext) {
	const created = gl.createTexture()!;
	gl.bindTexture(gl.TEXTURE_2D, created);
	for (const parameter of [gl.TEXTURE_MIN_FILTER, gl.TEXTURE_MAG_FILTER])
		gl.texParameteri(gl.TEXTURE_2D, parameter, gl.NEAREST);
	for (const parameter of [gl.TEXTURE_WRAP_S, gl.TEXTURE_WRAP_T])
		gl.texParameteri(gl.TEXTURE_2D, parameter, gl.CLAMP_TO_EDGE);
	return created;
}

function setUp(gl: WebGL2RenderingContext) {
	const program = gl.createProgram();
	gl.attachShader(program, compile(gl, gl.VERTEX_SHADER, VERTEX));
	gl.attachShader(program, compile(gl, gl.FRAGMENT_SHADER, FRAGMENT));
	gl.linkProgram(program);
	gl.useProgram(program);
	gl.bindBuffer(gl.ARRAY_BUFFER, gl.createBuffer());
	gl.bufferData(gl.ARRAY_BUFFER, new Float32Array([0, 0, 1, 0, 0, 1, 1, 1]), gl.STATIC_DRAW);
	const corner = gl.getAttribLocation(program, 'corner');
	gl.enableVertexAttribArray(corner);
	gl.vertexAttribPointer(corner, 2, gl.FLOAT, false, 0, 0);
	gl.uniform1i(gl.getUniformLocation(program, 'indexTexture'), 0);
	gl.uniform1i(gl.getUniformLocation(program, 'results'), 1);
	const maxTextureSize = gl.getParameter(gl.MAX_TEXTURE_SIZE) as number;
	const resultsRow = Math.min(RESULTS_ROW, maxTextureSize);
	gl.uniform1i(gl.getUniformLocation(program, 'resultsRow'), resultsRow);
	gl.pixelStorei(gl.UNPACK_ALIGNMENT, 1);
	gl.activeTexture(gl.TEXTURE0);
	const indexTexture = texture(gl);
	gl.activeTexture(gl.TEXTURE1);
	const resultsTexture = texture(gl);
	const uniform = (name: string) => gl.getUniformLocation(program, name);
	return {
		maxTextureSize,
		resultsRow,
		indexTexture,
		resultsTexture,
		placement: uniform('placement'),
		sourceSize: uniform('sourceSize'),
		canvasHeight: uniform('canvasHeight')
	};
}

/**
 * Svelte attachment: draw the source through the live effects, exactly, at the canvas's own device
 * resolution and only where the source is visible. `place` says where the source sits in a canvas of
 * the given CSS size; the view redraws whenever it changes. The colour index uploads once per image,
 * and an edit uploads only each colour's result. Nothing is read back from this canvas. `content`
 * picks which results to draw, such as `shownSource` for the Source preview.
 */
export function effectsView(
	place: (width: number, height: number) => Placement | undefined,
	content: ReadableAtom<Shown | undefined> = shownEffects
) {
	return (canvas: HTMLCanvasElement) => {
		const gl = canvas.getContext('webgl2', { premultipliedAlpha: false, antialias: false });
		if (!gl) {
			console.error('WebGL2 is unavailable, so effects cannot be drawn on the source.');
			return;
		}
		let resources = setUp(gl);
		let uploadedIndex: Uint32Array | undefined;
		let uploadedResults: Uint32Array | undefined;
		let lost = false;

		function fail(message: string) {
			console.error(message);
			effectsDrawn.set(false);
		}

		function upload({ index, results }: Shown) {
			if (!gl) return;
			if (index.pixels !== uploadedIndex) {
				const { width, height } = index.source;
				if (Math.max(width, height) > resources.maxTextureSize)
					return fail('This GPU cannot hold the image to draw effects on it.');
				gl.activeTexture(gl.TEXTURE0);
				gl.bindTexture(gl.TEXTURE_2D, resources.indexTexture);
				const bytes = new Uint8Array(
					index.pixels.buffer,
					index.pixels.byteOffset,
					width * height * 4
				);
				gl.texImage2D(
					gl.TEXTURE_2D,
					0,
					gl.RGBA8,
					width,
					height,
					0,
					gl.RGBA,
					gl.UNSIGNED_BYTE,
					bytes
				);
				uploadedIndex = index.pixels;
			}
			if (results !== uploadedResults) {
				const { resultsRow, maxTextureSize } = resources;
				const rows = Math.max(1, Math.ceil(results.length / resultsRow));
				if (rows > maxTextureSize)
					return fail('This GPU cannot hold the colours to draw effects on them.');
				const padded = new Uint32Array(rows * resultsRow);
				padded.set(results);
				gl.activeTexture(gl.TEXTURE1);
				gl.bindTexture(gl.TEXTURE_2D, resources.resultsTexture);
				const bytes = new Uint8Array(padded.buffer);
				gl.texImage2D(
					gl.TEXTURE_2D,
					0,
					gl.RGBA8,
					resultsRow,
					rows,
					0,
					gl.RGBA,
					gl.UNSIGNED_BYTE,
					bytes
				);
				uploadedResults = results;
			}
			return true;
		}

		function draw() {
			const shown = content.get();
			// A canvas with no boxes is hidden; the resize observer draws it once it shows.
			if (!gl || lost || !shown || !canvas.getClientRects().length) return;
			const ratio = window.devicePixelRatio || 1;
			const [cssWidth, cssHeight] = [canvas.clientWidth, canvas.clientHeight];
			const placement = place(cssWidth, cssHeight);
			if (!placement || !placement.width || !placement.height) return;
			const [width, height] = [Math.round(cssWidth * ratio), Math.round(cssHeight * ratio)];
			if (canvas.width !== width || canvas.height !== height)
				[canvas.width, canvas.height] = [width, height];
			if (!upload(shown)) return;
			gl.viewport(0, 0, width, height);
			gl.clearColor(0, 0, 0, 0);
			gl.clear(gl.COLOR_BUFFER_BIT);
			const { left, top } = placement;
			gl.uniform4f(
				resources.placement,
				left * ratio,
				top * ratio,
				placement.width * ratio,
				placement.height * ratio
			);
			gl.uniform2f(resources.sourceSize, shown.index.source.width, shown.index.source.height);
			gl.uniform1f(resources.canvasHeight, height);
			gl.drawArrays(gl.TRIANGLE_STRIP, 0, 4);
			// WebGL skips a command it can't honour, such as an upload that runs out of memory, and
			// only records the error.
			if (gl.getError() !== gl.NO_ERROR) {
				uploadedIndex = uploadedResults = undefined;
				return fail('The GPU could not draw effects on this image.');
			}
			effectsDrawn.set(true);
		}

		const onLost = (event: Event) => {
			// Asking to be restored; the old program and textures are gone either way.
			event.preventDefault();
			lost = true;
			effectsDrawn.set(false);
		};
		const onRestored = () => {
			lost = false;
			resources = setUp(gl);
			uploadedIndex = uploadedResults = undefined;
			draw();
		};
		canvas.addEventListener('webglcontextlost', onLost);
		canvas.addEventListener('webglcontextrestored', onRestored);
		const resized = new ResizeObserver(draw);
		resized.observe(canvas);
		const unsubscribe = content.listen(draw);
		// Pan, zoom, and layout reach `place`; reading it here redraws on every change.
		$effect(() => {
			place(canvas.clientWidth, canvas.clientHeight);
			draw();
		});
		return () => {
			resized.disconnect();
			unsubscribe();
			canvas.removeEventListener('webglcontextlost', onLost);
			canvas.removeEventListener('webglcontextrestored', onRestored);
			effectsDrawn.set(false);
			gl.getExtension('WEBGL_lose_context')?.loseContext();
		};
	};
}
