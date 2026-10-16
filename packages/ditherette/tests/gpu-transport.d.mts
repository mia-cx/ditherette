/** Observable ownership of the synthetic transport. The package still runs its real Wasm. */
export function installGpuTransport(): {
	resources: Set<unknown>;
	devices: { destroy(): void }[];
	observations: {
		devices: number;
		handlers: number;
		lossHandlers: number;
		destroyed: number;
		externalUploads: number;
	};
};
