/**
 * Single source for the public guides: titles and descriptions feed the guide
 * pages, the /guides hub, cross-links between guides and the sitemap.
 */
export const GUIDES = [
	{
		slug: 'wplace-template',
		title: 'Make a Wplace template from an image',
		description:
			"Turn a picture into a Wplace-ready PNG: crop it, resize it to the pixels you'll paint, limit it to your Wplace colours, and export it."
	},
	{
		slug: 'wplace-colours',
		title: 'Convert an image to Wplace colours',
		description:
			"Match every pixel to the Wplace palette, keep to the free colours or the ones you've unlocked, and keep transparent areas transparent."
	},
	{
		slug: 'dithering',
		title: 'Choose a dithering method for a limited palette',
		description:
			'How ordered, error-diffusion and noise dithering trade smooth gradients against clean flat areas, with examples on the Wplace free colours.'
	},
	{
		slug: 'colour-quantisation',
		title: 'Reduce an image to a fixed palette',
		description:
			'Colour quantisation with a palette you choose: build or import a palette, pick how colour difference is measured, and export an indexed PNG.'
	}
] as const;

export type Guide = (typeof GUIDES)[number];

/** Absolute path of a guide page, e.g. `/guides/dithering`. */
export const guidePath = (slug: Guide['slug']) => `/guides/${slug}` as const;
