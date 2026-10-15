<script lang="ts">
	import { resolve } from '$app/paths';
	import GuideArticle from '../GuideArticle.svelte';
	import GuideFigure from '../GuideFigure.svelte';
	import none from '$lib/assets/guides/wplace-free-none.png';
	import bayer4 from '$lib/assets/guides/wplace-free-bayer4.png';
	import floyd from '$lib/assets/guides/wplace-free-floyd.png';
	import blueNoise from '$lib/assets/guides/wplace-free-bluenoise.png';
</script>

<GuideArticle slug="dithering">
	<p>
		Dithering mixes palette colours in a fine pattern so that, from a distance, they read as a
		colour the palette doesn't have. It turns the bands of a gradient back into a smooth ramp. The
		cost is texture: neighbouring pixels change colour more often, which also makes a template
		slower to paint by hand.
	</p>

	<GuideFigure
		images={[
			{
				src: none,
				alt: 'The sunset in the free Wplace colours with no dithering.',
				label: 'None',
				width: 96,
				height: 60,
				pixelated: true
			},
			{
				src: bayer4,
				alt: 'The sunset in the free Wplace colours with Bayer 4×4 dithering.',
				label: 'Bayer 4×4',
				width: 96,
				height: 60,
				pixelated: true
			},
			{
				src: floyd,
				alt: 'The sunset in the free Wplace colours with Floyd–Steinberg dithering.',
				label: 'Floyd–Steinberg',
				width: 96,
				height: 60,
				pixelated: true
			},
			{
				src: blueNoise,
				alt: 'The sunset in the free Wplace colours with blue-noise dithering.',
				label: 'Blue noise',
				width: 96,
				height: 60,
				pixelated: true
			}
		]}
		caption="96 × 60, the 31 free Wplace colours, shown at 4×."
	/>

	<h2>The methods</h2>
	<p>Ditherette's Dither panel groups its methods into four families.</p>
	<ul>
		<li>
			<strong>None.</strong> Each pixel takes its nearest palette colour. Flat areas stay flat and gradients
			turn into bands. Use it for logos, text and flat artwork.
		</li>
		<li>
			<strong>Ordered.</strong> Bayer matrices from 2×2 to 16×16 and rectangular tiles repeat one threshold
			pattern across the image, giving an even cross-hatch that's easy to follow pixel by pixel. Yliluoma's
			method is an ordered dither built for arbitrary palettes: slower, with more accurate colour mixes.
		</li>
		<li>
			<strong>Error diffusion.</strong> Floyd–Steinberg, Atkinson, Sierra and the other kernels pass each
			pixel's rounding error on to the neighbours not yet processed. They keep the most detail and the
			smoothest gradients, with a busier, irregular texture. Atkinson passes on only part of the error,
			so it keeps more contrast and cleaner flat areas.
		</li>
		<li>
			<strong>Noise.</strong> Random and Blue noise use a noise threshold instead of a repeating pattern.
			Blue noise spreads its dots evenly, without clumps or visible repetition.
		</li>
	</ul>

	<h2>Strength and placement</h2>
	<p>
		Strength sets how far the dither pushes each pixel; lower it to keep flat areas mostly flat.
		Placement can apply the dither everywhere, or adaptively: Adaptive dithers only smooth gradients
		at least Min gradient pixels long, and leaves flat fills and hard edges clean.
	</p>

	<h2>For a Wplace template</h2>
	<p>
		Start with None and look for bands. If a gradient breaks badly, try Bayer 4×4 or Blue noise, and
		lower Strength until the flat areas clean up. Ordered patterns are the easiest to paint by hand;
		error diffusion looks best at a distance but scatters single pixels everywhere. The image size
		and the pixels to paint stay the same whichever you choose.
		<a href={resolve('/guides/wplace-template')}>Make a Wplace template</a>
		covers the rest of the workflow.
	</p>
</GuideArticle>
