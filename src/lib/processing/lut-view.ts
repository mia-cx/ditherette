import { atom, computed } from 'nanostores';
import { effectsTable, sourceImageData } from '$lib/stores/app';
import { activeEffectSteps } from '$lib/stores/effects';
import { TABLE_SIDE } from './effects-table';

/** The effects table to draw: the pipeline's latest, while any effect is on. */
export const shownEffectsTable = computed([effectsTable, activeEffectSteps], (table, steps) =>
	steps.length ? table : undefined
);

const VERTEX = `#version 300 es
in vec2 corner;
out vec2 uv;
void main() {
	// Row 0 of the uploaded image sits at the top of the canvas.
	uv = vec2(corner.x, 1.0 - corner.y);
	gl_Position = vec4(corner * 2.0 - 1.0, 0.0, 1.0);
}`;

/** Read each source colour's own texel, at its centre and unfiltered, so the drawing is exact. */
const FRAGMENT = `#version 300 es
precision highp float;
precision highp sampler3D;
uniform sampler2D source;
uniform sampler3D lut;
in vec2 uv;
out vec4 color;
const float SIZE = ${TABLE_SIDE}.0;
void main() {
	vec4 pixel = texture(source, uv);
	vec3 coordinate = (round(pixel.rgb * 255.0) + 0.5) / SIZE;
	color = vec4(texture(lut, coordinate).rgb, pixel.a);
}`;

function compile(gl: WebGL2RenderingContext, type: number, text: string) {
	const shader = gl.createShader(type)!;
	gl.shaderSource(shader, text);
	gl.compileShader(shader);
	if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS))
		throw new Error(gl.getShaderInfoLog(shader) ?? 'Shader failed to compile.');
	return shader;
}

function texture(gl: WebGL2RenderingContext, target: number, filter: number) {
	const created = gl.createTexture()!;
	gl.bindTexture(target, created);
	for (const parameter of [gl.TEXTURE_MIN_FILTER, gl.TEXTURE_MAG_FILTER])
		gl.texParameteri(target, parameter, filter);
	for (const parameter of [gl.TEXTURE_WRAP_S, gl.TEXTURE_WRAP_T, gl.TEXTURE_WRAP_R])
		gl.texParameteri(target, parameter, gl.CLAMP_TO_EDGE);
	return created;
}

/** Whether this browser can draw effects on the source at all. Checked once. */
export const supportsWebGL2 = (() => {
	let supported: boolean | undefined;
	return () => (supported ??= document.createElement('canvas').getContext('webgl2') !== null);
})();

/** Whether the Source pane currently shows the source through the table. */
export const lutDrawn = atom(false);

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
	gl.uniform1i(gl.getUniformLocation(program, 'source'), 0);
	gl.uniform1i(gl.getUniformLocation(program, 'lut'), 1);
	gl.pixelStorei(gl.UNPACK_COLORSPACE_CONVERSION_WEBGL, gl.NONE);
	gl.activeTexture(gl.TEXTURE0);
	const sourceTexture = texture(gl, gl.TEXTURE_2D, gl.NEAREST);
	gl.activeTexture(gl.TEXTURE1);
	const lutTexture = texture(gl, gl.TEXTURE_3D, gl.NEAREST);
	return { sourceTexture, lutTexture };
}

/** The largest image side this context can both hold as a texture and draw in one viewport. */
function drawableSide(gl: WebGL2RenderingContext) {
	const [viewportWidth, viewportHeight] = gl.getParameter(gl.MAX_VIEWPORT_DIMS) as Int32Array;
	return Math.min(gl.getParameter(gl.MAX_TEXTURE_SIZE) as number, viewportWidth!, viewportHeight!);
}

/**
 * Svelte attachment: draw the source through the pipeline's exact effects table at full source
 * resolution. The source uploads once per image and the table once per new table, so redraws take
 * milliseconds at any image size. A hidden canvas draws nothing until it shows again. `lutDrawn`
 * says whether the drawing is showing; after a GPU reset, resources are rebuilt and the drawing
 * returns. Nothing is read back from this canvas.
 */
export function lutView(canvas: HTMLCanvasElement) {
	const gl = canvas.getContext('webgl2', { premultipliedAlpha: false, antialias: false });
	if (!gl) {
		console.error('WebGL2 is unavailable, so effects cannot be drawn on the source.');
		return;
	}
	let resources = setUp(gl);
	let uploaded: ImageData | undefined;
	let uploadedTable: Uint32Array | undefined;
	let lost = false;

	function fail(message: string) {
		console.error(message);
		lutDrawn.set(false);
	}

	function draw() {
		const source = sourceImageData.get();
		const table = shownEffectsTable.get();
		// A canvas with no boxes is hidden; the resize observer draws it once it shows.
		if (!gl || lost || !source || !table || !canvas.getClientRects().length) return;
		if (source !== uploaded) {
			const limit = drawableSide(gl);
			if (Math.max(source.width, source.height) > limit)
				return fail(`This GPU draws effects on images up to ${limit} pixels on a side.`);
			canvas.width = source.width;
			canvas.height = source.height;
			// Browsers may shrink a drawing buffer they can't allocate instead of failing.
			if (gl.drawingBufferWidth !== source.width || gl.drawingBufferHeight !== source.height)
				return fail('The GPU could not make room to draw effects on this image.');
			gl.activeTexture(gl.TEXTURE0);
			gl.bindTexture(gl.TEXTURE_2D, resources.sourceTexture);
			gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA8, gl.RGBA, gl.UNSIGNED_BYTE, source);
			uploaded = source;
		}
		if (table !== uploadedTable) {
			gl.activeTexture(gl.TEXTURE1);
			gl.bindTexture(gl.TEXTURE_3D, resources.lutTexture);
			const [side, bytes] = [TABLE_SIDE, new Uint8Array(table.buffer)];
			gl.texImage3D(
				gl.TEXTURE_3D,
				0,
				gl.RGBA8,
				side,
				side,
				side,
				0,
				gl.RGBA,
				gl.UNSIGNED_BYTE,
				bytes
			);
			uploadedTable = table;
		}
		gl.viewport(0, 0, canvas.width, canvas.height);
		gl.drawArrays(gl.TRIANGLE_STRIP, 0, 4);
		// WebGL skips a command it can't honour, such as an upload that runs out of memory, and only
		// records the error.
		if (gl.getError() !== gl.NO_ERROR) {
			uploaded = undefined;
			uploadedTable = undefined;
			return fail('The GPU could not draw effects on this image.');
		}
		lutDrawn.set(true);
	}

	const onLost = (event: Event) => {
		// Asking to be restored; the old program and textures are gone either way.
		event.preventDefault();
		lost = true;
		lutDrawn.set(false);
	};
	const onRestored = () => {
		lost = false;
		resources = setUp(gl);
		uploaded = undefined;
		uploadedTable = undefined;
		draw();
	};
	canvas.addEventListener('webglcontextlost', onLost);
	canvas.addEventListener('webglcontextrestored', onRestored);
	const shown = new ResizeObserver(draw);
	shown.observe(canvas);
	const unsubscribers = [sourceImageData.listen(draw), shownEffectsTable.listen(draw)];
	draw();
	return () => {
		shown.disconnect();
		for (const unsubscribe of unsubscribers) unsubscribe();
		canvas.removeEventListener('webglcontextlost', onLost);
		canvas.removeEventListener('webglcontextrestored', onRestored);
		lutDrawn.set(false);
		gl.getExtension('WEBGL_lose_context')?.loseContext();
	};
}
