<script lang="ts">
	import { resolve } from '$app/paths';
	import GuideArticle from '../GuideArticle.svelte';
	import GuideFigure from '../GuideFigure.svelte';
	import allColours from '$lib/assets/guides/wplace-all-none.png';
	import freeColours from '$lib/assets/guides/wplace-free-none.png';
</script>

<GuideArticle slug="wplace-colours">
	<p>
		Wplace paints with a fixed palette: Transparent, 31 free colours and 32 more that you unlock in
		the game. A photo holds thousands of colours, so each of its pixels has to become a colour you
		can paint. Ditherette matches every pixel to the nearest colour you allow and shows the result
		beside the source as you change settings.
	</p>

	<GuideFigure
		images={[
			{
				src: allColours,
				alt: 'The sunset matched to all 63 Wplace colours without dithering.',
				label: 'All 63 colours',
				width: 96,
				height: 60,
				pixelated: true
			},
			{
				src: freeColours,
				alt: 'The sunset matched to the 31 free Wplace colours without dithering.',
				label: '31 free colours',
				width: 96,
				height: 60,
				pixelated: true
			}
		]}
		caption="96 × 60, no dithering, shown at 4×."
	/>

	<h2>Use only the colours you can paint</h2>
	<p>
		The palette table tags each colour Free or Premium. Choose a tag to select every colour that has
		it, then Toggle visibility hides or shows them all at once. The eye button does the same for one
		colour. Hidden colours never appear in the output. If you've unlocked a few premium colours,
		hide every Premium colour that way, then show the ones you own with their eye buttons.
	</p>
	<p>
		The Pixels column counts each colour in the output, and the export bar totals the pixels to
		paint, leaving out Transparent. That total is the number of pixels you'll place.
	</p>

	<h2>Fit the image to the palette</h2>
	<p>
		Nearest-colour matching can flatten a photo. Skies, skin and shadows often fall between Wplace
		colours and collapse into one. Image &gt; Adjustments &gt; Palette fit analyses the image
		together with the colours you've left visible, then adjusts its tones and colours toward what
		those colours can show, before matching. Choose Natural, Fitted or Vivid. Advanced shows the
		curves it wrote, which you can edit.
	</p>

	<h2>Keep transparent areas transparent</h2>
	<p>
		Transparent pixels in a template are pixels you leave alone. Keep the Transparent swatch visible
		to keep them. Its settings choose how partly transparent pixels are handled: a threshold decides
		which become transparent, and Matte blends soft edges onto a colour instead.
	</p>

	<h2>When the nearest colour looks wrong</h2>
	<p>
		Which colour counts as nearest depends on how colour difference is measured. Colour space sets
		that measure, and OKLab is the default. If greens or skin tones land on the wrong colour, try
		another one.
		<a href={resolve('/guides/colour-quantisation')}>Reduce an image to a fixed palette</a>
		explains the options.
	</p>
</GuideArticle>
