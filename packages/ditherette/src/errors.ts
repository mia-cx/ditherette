/** Stable categories shared with the frozen version-one contract. */
export type ErrorCode =
	| 'invalid-request'
	| 'invalid-image'
	| 'invalid-palette'
	| 'invalid-settings'
	| 'unsupported-operation'
	| 'capability'
	| 'initialization'
	| 'memory-limit'
	| 'wasm-memory-unavailable'
	| 'disposed'
	| 'reentrant-call'
	| 'callback'
	| 'runtime';

/** A failed call exposes this diagnostic and no partial image. */
export class DitheretteError extends Error {
	readonly code: ErrorCode;
	readonly path: string;

	constructor(code: ErrorCode, path: string, message: string) {
		super(message);
		this.name = 'DitheretteError';
		this.code = code;
		this.path = path;
	}
}
