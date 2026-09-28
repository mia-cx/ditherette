import { sourceImageData } from '$lib/stores/app';
import { LUT_SIZE, sourceEffectsLut } from './source-effects';

const VERTEX = `#version 300 es
in vec2 corner;
out vec2 uv;
void main() {
	// Row 0 of the uploaded image sits at the top of the canvas.
	uv = vec2(corner.x, 1.0 - corner.y);
	gl_Position = vec4(corner * 2.0 - 1.0, 0.0, 1.0);
}`;

/** Sample the lattice at its texel centres, so each lattice colour maps to exactly its own entry. */
const FRAGMENT = `#version 300 es
precision highp float;
precision highp sampler3D;
uniform sampler2D source;
uniform sampler3D lut;
in vec2 uv;
out vec4 color;
const float SIZE = ${LUT_SIZE}.0;
void main() {
	vec4 pixel = texture(source, uv);
	vec3 coordinate = pixel.rgb * ((SIZE - 1.0) / SIZE) + 0.5 / SIZE;
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

/**
 * Svelte attachment: draw the source through the effects lookup table at full source resolution.
 * The source uploads once per image and each edit uploads only the table, so redraws take
 * milliseconds at any image size. Display only: nothing is read back from this canvas.
 */
export function lutView(canvas: HTMLCanvasElement) {
	const gl = canvas.getContext('webgl2', { premultipliedAlpha: false, antialias: false });
	if (!gl) {
		console.error('WebGL2 is unavailable, so effects cannot be drawn on the source.');
		return;
	}
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
	const lutTexture = texture(gl, gl.TEXTURE_3D, gl.LINEAR);
	let uploaded: ImageData | undefined;

	function draw() {
		const source = sourceImageData.get();
		const lut = sourceEffectsLut.get();
		if (!gl || !source || !lut) return;
		if (source !== uploaded) {
			const limit: number = gl.getParameter(gl.MAX_TEXTURE_SIZE);
			if (Math.max(source.width, source.height) > limit) {
				console.error(`This GPU draws effects on images up to ${limit} pixels wide.`);
				return;
			}
			canvas.width = source.width;
			canvas.height = source.height;
			gl.activeTexture(gl.TEXTURE0);
			gl.bindTexture(gl.TEXTURE_2D, sourceTexture);
			gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA8, gl.RGBA, gl.UNSIGNED_BYTE, source);
			uploaded = source;
		}
		gl.activeTexture(gl.TEXTURE1);
		gl.bindTexture(gl.TEXTURE_3D, lutTexture);
		const size = LUT_SIZE;
		gl.texImage3D(gl.TEXTURE_3D, 0, gl.RGBA8, size, size, size, 0, gl.RGBA, gl.UNSIGNED_BYTE, lut);
		gl.viewport(0, 0, canvas.width, canvas.height);
		gl.drawArrays(gl.TRIANGLE_STRIP, 0, 4);
	}

	const unsubscribers = [sourceImageData.listen(draw), sourceEffectsLut.listen(draw)];
	draw();
	return () => {
		for (const unsubscribe of unsubscribers) unsubscribe();
		gl.getExtension('WEBGL_lose_context')?.loseContext();
	};
}
