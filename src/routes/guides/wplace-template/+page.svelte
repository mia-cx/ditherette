<script lang="ts">
	import { resolve } from '$app/paths';
	import GuideArticle from '../GuideArticle.svelte';
	import GuideFigure from '../GuideFigure.svelte';
	import source from '$lib/assets/guides/source.png';
	import template from '$lib/assets/guides/wplace-free-none.png';
</script>

<GuideArticle slug="wplace-template">
	<p>
		Wplace's overlay tool places a reference image on the map so you can paint over it. Ditherette
		prepares that image. You choose its size in pixels and which Wplace colours it may use, and
		Ditherette exports a PNG where every pixel is already one of those colours.
	</p>
	<p>
		Ditherette doesn't connect to Wplace and doesn't place pixels. You upload the PNG to Wplace
		yourself and paint it by hand.
	</p>

	<h2>Pick a size first</h2>
	<p>
		Each pixel in the template is one pixel you paint. A 100 × 100 template is 10,000 pixels.
		Wplace's
		<a href="https://wplace.live/guides/overlays" rel="noopener">overlay guide</a>
		recommends a small, clear design over a detailed photograph, and it's good advice: start smaller than
		you think.
	</p>

	<GuideFigure
		images={[
			{
				src: source,
				alt: 'A sunset illustration: a gradient sky from deep blue to gold, a pale sun and two layers of dark hills.',
				label: 'Source, 384 × 240',
				width: 384,
				height: 240
			},
			{
				src: template,
				alt: 'The sunset as a 96 × 60 template in the free Wplace colours, without dithering.',
				label: 'Template, 96 × 60',
				width: 96,
				height: 60,
				pixelated: true
			}
		]}
		caption="Free Wplace colours, no dithering. Templates are shown enlarged."
	/>

	<h2>Steps</h2>
	<ol>
		<li>
			Open the editor and drop an image on the preview, or choose File &gt; Open image. PNG, JPEG,
			WebP and GIF files work. The image stays in your browser; Ditherette doesn't upload it.
		</li>
		<li>
			Crop to the part you want. Choose Edit &gt; Crop or the crop button on the preview, drag the
			handles, then Apply. Content crops to the non-transparent part of the image.
		</li>
		<li>
			Set the size in Dimensions: type a width, a height or a scale. Nearest keeps hard edges in
			flat artwork. For photos, Area or a Lanczos filter averages the detail it removes.
		</li>
		<li>
			Choose colours in Palette. The built-in Wplace palette tags each colour Free or Premium. Hide
			the colours you haven't unlocked with the eye button, and the output will only use colours you
			can paint.
			<a href={resolve('/guides/wplace-colours')}>Convert an image to Wplace colours</a>
			covers this step in detail.
		</li>
		<li>
			Decide on dithering in Dither. None gives flat areas that are quick to paint. A dither pattern
			keeps gradients and shading, with more colour changes between neighbouring pixels. See
			<a href={resolve('/guides/dithering')}>Choose a dithering method</a>.
		</li>
		<li>
			Compare source and output with Side by side or A/B reveal, then choose File &gt; Export PNG.
			The export bar shows the output size, the number of colours and the pixels to paint.
		</li>
	</ol>

	<h2>What the PNG contains</h2>
	<p>
		The export is an indexed PNG: each pixel stores the number of a palette colour, so colours come
		out exact, with no smoothing or compression shifts. Transparent pixels stay transparent, so the
		template covers only the pixels you mean to paint.
	</p>

	<h2>Put it on Wplace</h2>
	<p>
		In Wplace, open Overlays from the map, upload the PNG and set its position. Wplace anchors an
		overlay at its top-left pixel. Its overlay tool has its own size and colour conversion, so check
		that the converted overlay still matches your PNG before you save it. Then paint over it, pixel
		by pixel. Wplace's
		<a href="https://wplace.live/guides/overlays" rel="noopener">overlay guide</a>
		covers positioning and paint mode.
	</p>
</GuideArticle>
