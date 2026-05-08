export type JsonValue =
	| null
	| boolean
	| number
	| string
	| readonly JsonValue[]
	| { readonly [key: string]: JsonValue | undefined };

export function settingsHash(value: JsonValue): string {
	try {
		const serialized = JSON.stringify(value);
		if (serialized === undefined) throw new Error('Settings are not JSON serializable.');
		return serialized;
	} catch (error) {
		throw new Error('Settings could not be hashed for processing.', { cause: error });
	}
}
