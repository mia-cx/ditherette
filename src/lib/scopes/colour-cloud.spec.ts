import { describe, expect, it } from 'vitest';
import { colourCloud } from './colour-cloud';

type Pixel = [number, number, number, number];
const image = (...pixels: Pixel[]) =>
	({
		data: new Uint8ClampedArray(pixels.flat()),
		width: pixels.length,
		height: 1
	}) as ImageData;

describe('colourCloud', () => {
	it('omits fully transparent source pixels', () => {
		const cloud = colourCloud(image([255, 0, 0, 255], [0, 255, 0, 0], [0, 0, 255, 255]), undefined);

		expect(cloud.count).toBe(2);
		expect([...cloud.colours]).toEqual([1, 0, 0, 0.55, 0, 0, 1, 0.55].map(Math.fround));
	});

	it('caps large images while sampling both sides of a repeating pattern', () => {
		const [width, height] = [1000, 300];
		const data = new Uint8ClampedArray(width * height * 4);
		for (let pixel = 0; pixel < width * height; pixel++)
			data.set(pixel % 2 ? [0, 0, 255, 255] : [255, 0, 0, 255], pixel * 4);

		const cloud = colourCloud({ data, width, height } as ImageData, undefined);
		let red = 0;
		for (let sample = 0; sample < cloud.count; sample++)
			red += cloud.colours[sample * 4]! > 0.5 ? 1 : 0;
		const redShare = red / cloud.count;

		expect(cloud.count).toBeLessThanOrEqual(150_000);
		expect(redShare).toBeGreaterThan(0.45);
		expect(redShare).toBeLessThan(0.55);
	});
});
