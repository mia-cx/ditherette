const stages = new Set([
	'prepare',
	'resize',
	'alpha',
	'color',
	'perturb',
	'quantize',
	'dither-and-quantize',
	'complete'
]);
const validCount = (value) => value === undefined || (Number.isInteger(value) && value >= 0);

/** Constant-storage callback work. Reset and verify outside the public-call timer. */
export function progressProbe() {
	let count;
	let complete;
	let invalid;
	let lastStage;
	let lastCompleted;
	let lastTotal;
	const reset = () => {
		count = 0;
		complete = false;
		invalid = false;
		lastStage = undefined;
		lastCompleted = undefined;
		lastTotal = undefined;
	};
	reset();
	return {
		reset,
		onProgress({ stage, completed, total }) {
			count++;
			invalid ||=
				complete ||
				!stages.has(stage) ||
				!validCount(completed) ||
				!validCount(total) ||
				(completed !== undefined && total !== undefined && completed > total);
			complete ||= stage === 'complete';
			lastStage = stage;
			lastCompleted = completed;
			lastTotal = total;
		},
		verify() {
			if (invalid || !complete || lastStage !== 'complete') {
				throw new Error(
					`Invalid progress delivery: count=${count}, invalid=${invalid}, last=${String(lastStage)}, completed=${String(lastCompleted)}, total=${String(lastTotal)}`
				);
			}
			return count;
		}
	};
}
