/**
 * A small WebGL2 view for coloured points, lines, and triangles around an orbiting camera. The
 * curve analysis draws its 3D surface and 3D colour scope with it.
 */

type Mat4 = number[];
export type Vec3 = readonly [number, number, number];

function perspective(fovy: number, aspect: number, near: number, far: number): Mat4 {
	const f = 1 / Math.tan(fovy / 2);
	const nf = 1 / (near - far);
	return [
		f / aspect,
		0,
		0,
		0,
		0,
		f,
		0,
		0,
		0,
		0,
		(far + near) * nf,
		-1,
		0,
		0,
		2 * far * near * nf,
		0
	];
}

function lookAt(eye: Vec3, target: Vec3): Mat4 {
	const normalise = (v: number[]) => {
		const length = Math.hypot(...v);
		return v.map((c) => c / length);
	};
	const z = normalise([eye[0] - target[0], eye[1] - target[1], eye[2] - target[2]]);
	const x = normalise([z[2]!, 0, -z[0]!]);
	const y = [
		z[1]! * x[2]! - z[2]! * x[1]!,
		z[2]! * x[0]! - z[0]! * x[2]!,
		z[0]! * x[1]! - z[1]! * x[0]!
	];
	const dot = (a: number[]) => a[0]! * eye[0] + a[1]! * eye[1] + a[2]! * eye[2];
	return [
		x[0]!,
		y[0]!,
		z[0]!,
		0,
		x[1]!,
		y[1]!,
		z[1]!,
		0,
		x[2]!,
		y[2]!,
		z[2]!,
		0,
		-dot(x),
		-dot(y),
		-dot(z),
		1
	];
}

function multiply(a: Mat4, b: Mat4): Mat4 {
	const out = new Array<number>(16);
	for (let column = 0; column < 4; column++)
		for (let row = 0; row < 4; row++) {
			let sum = 0;
			for (let k = 0; k < 4; k++) sum += a[k * 4 + row]! * b[column * 4 + k]!;
			out[column * 4 + row] = sum;
		}
	return out;
}

const VERTEX = `#version 300 es
in vec3 position;
in vec4 colour;
uniform mat4 transform;
uniform float size;
out vec4 tint;
void main() {
	gl_Position = transform * vec4(position, 1.0);
	gl_PointSize = size;
	tint = colour;
}`;
const FRAGMENT = `#version 300 es
precision mediump float;
in vec4 tint;
uniform bool round;
out vec4 colour;
void main() {
	if (round) {
		vec2 d = gl_PointCoord - 0.5;
		if (dot(d, d) > 0.25) discard;
	}
	colour = tint;
}`;

export type Camera = { yaw: number; pitch: number; distance: number; target: Vec3 };
export type DrawOptions = {
	size?: number;
	round?: boolean;
	depth?: boolean;
	indices?: Uint32Array;
};

/** Pitch stays above the floor and short of straight down. */
const PITCH = { min: -0.2, max: 1.45 } as const;
const DISTANCE = { min: 0.8, max: 6 } as const;

export function orbitView(canvas: HTMLCanvasElement, camera: Camera) {
	const gl = canvas.getContext('webgl2', { antialias: true });
	if (!gl) return undefined;
	const compile = (type: number, text: string) => {
		const shader = gl.createShader(type)!;
		gl.shaderSource(shader, text);
		gl.compileShader(shader);
		if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS))
			throw new Error(gl.getShaderInfoLog(shader) ?? '');
		return shader;
	};
	const program = gl.createProgram();
	gl.attachShader(program, compile(gl.VERTEX_SHADER, VERTEX));
	gl.attachShader(program, compile(gl.FRAGMENT_SHADER, FRAGMENT));
	gl.linkProgram(program);
	const at = {
		position: gl.getAttribLocation(program, 'position'),
		colour: gl.getAttribLocation(program, 'colour'),
		transform: gl.getUniformLocation(program, 'transform'),
		size: gl.getUniformLocation(program, 'size'),
		round: gl.getUniformLocation(program, 'round')
	};
	const dpr = () => window.devicePixelRatio || 1;

	/** Camera transform for the canvas's current size. */
	function transform(): Mat4 {
		const { yaw, pitch, distance, target } = camera;
		const eye: Vec3 = [
			target[0] + distance * Math.cos(pitch) * Math.sin(yaw),
			target[1] + distance * Math.sin(pitch),
			target[2] + distance * Math.cos(pitch) * Math.cos(yaw)
		];
		return multiply(perspective(0.75, canvas.width / canvas.height, 0.05, 20), lookAt(eye, target));
	}

	return {
		/** Resize to the element, clear, and set the camera. Call before drawing a frame. */
		begin() {
			const box = canvas.getBoundingClientRect();
			const [width, height] = [Math.round(box.width * dpr()), Math.round(box.height * dpr())];
			if (canvas.width !== width || canvas.height !== height)
				[canvas.width, canvas.height] = [width, height];
			gl.viewport(0, 0, canvas.width, canvas.height);
			gl.clearColor(0, 0, 0, 0);
			gl.clear(gl.COLOR_BUFFER_BIT | gl.DEPTH_BUFFER_BIT);
			gl.enable(gl.BLEND);
			gl.blendFunc(gl.SRC_ALPHA, gl.ONE_MINUS_SRC_ALPHA);
			gl.useProgram(program);
			gl.uniformMatrix4fv(at.transform, false, transform());
		},
		/** Draw xyz positions with rgba colours from 0 to 1. */
		draw(
			mode: 'points' | 'lines' | 'triangles',
			positions: Float32Array,
			colours: Float32Array,
			options: DrawOptions = {}
		) {
			const { size = 1, round = false, depth = true, indices } = options;
			if (depth) gl.enable(gl.DEPTH_TEST);
			else gl.disable(gl.DEPTH_TEST);
			const vao = gl.createVertexArray();
			gl.bindVertexArray(vao);
			const buffers = [
				[positions, at.position, 3],
				[colours, at.colour, 4]
			].map(([data, location, width]) => {
				const buffer = gl.createBuffer();
				gl.bindBuffer(gl.ARRAY_BUFFER, buffer);
				gl.bufferData(gl.ARRAY_BUFFER, data as Float32Array, gl.STREAM_DRAW);
				gl.enableVertexAttribArray(location as number);
				gl.vertexAttribPointer(location as number, width as number, gl.FLOAT, false, 0, 0);
				return buffer;
			});
			gl.uniform1f(at.size, size * dpr());
			gl.uniform1i(at.round, round ? 1 : 0);
			const primitive = { points: gl.POINTS, lines: gl.LINES, triangles: gl.TRIANGLES }[mode];
			if (indices) {
				const buffer = gl.createBuffer();
				gl.bindBuffer(gl.ELEMENT_ARRAY_BUFFER, buffer);
				gl.bufferData(gl.ELEMENT_ARRAY_BUFFER, indices, gl.STREAM_DRAW);
				gl.drawElements(primitive, indices.length, gl.UNSIGNED_INT, 0);
				gl.deleteBuffer(buffer);
			} else gl.drawArrays(primitive, 0, positions.length / 3);
			for (const buffer of buffers) gl.deleteBuffer(buffer);
			gl.deleteVertexArray(vao);
		},
		/** A world point in canvas CSS pixels, for labels and hit tests. */
		project(point: Vec3): [number, number] {
			const m = transform();
			const [x, y, z] = point;
			const w = m[3]! * x + m[7]! * y + m[11]! * z + m[15]!;
			const px = (m[0]! * x + m[4]! * y + m[8]! * z + m[12]!) / w;
			const py = (m[1]! * x + m[5]! * y + m[9]! * z + m[13]!) / w;
			return [((px + 1) / 2) * (canvas.width / dpr()), ((1 - py) / 2) * (canvas.height / dpr())];
		},
		/** Turn the camera by a pointer drag, in CSS pixels. */
		orbit(dx: number, dy: number) {
			camera.yaw -= dx * 0.008;
			camera.pitch = Math.min(PITCH.max, Math.max(PITCH.min, camera.pitch + dy * 0.006));
		},
		/** Turn the camera around its target, for auto-rotation. */
		turn(radians: number) {
			camera.yaw += radians;
		},
		zoom(deltaY: number) {
			camera.distance = Math.min(
				DISTANCE.max,
				Math.max(DISTANCE.min, camera.distance * (1 + deltaY * 0.001))
			);
		}
	};
}

export type OrbitView = NonNullable<ReturnType<typeof orbitView>>;
