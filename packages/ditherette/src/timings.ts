/**
 * Host wall time, including awaited GPU completion; nested spans overlap.
 * Names are diagnostic labels and may change between releases.
 */
export interface ExecutionTiming {
	readonly name: string;
	readonly ms: number;
}

/** Aggregate bounded stage spans without retaining per-dispatch records. */
export function executionTimings() {
	const spans = new Map<string, { ms: number; start?: number }>();
	return {
		mark(name: string, begin: boolean) {
			const now = performance.now();
			const span = spans.get(name) ?? { ms: 0 };
			if (begin) span.start = now;
			else if (span.start !== undefined) {
				span.ms += now - span.start;
				span.start = undefined;
			}
			spans.set(name, span);
		},
		finish(): ExecutionTiming[] {
			const now = performance.now();
			return [...spans].map(([name, span]) => {
				if (span.start !== undefined) span.ms += now - span.start;
				span.start = undefined;
				return { name, ms: span.ms };
			});
		}
	};
}
