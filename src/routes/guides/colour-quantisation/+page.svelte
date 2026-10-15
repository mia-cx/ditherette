<script lang="ts">
	import GuideArticle from '../GuideArticle.svelte';
	import GuideFigure from '../GuideFigure.svelte';
	import noDither from '$lib/assets/guides/custom-none.png';
	import atkinson from '$lib/assets/guides/custom-atkinson.png';
</script>

<GuideArticle slug="colour-quantisation">
	<p>
		Colour quantisation (quantization in US spelling) reduces an image to a limited set of colours.
		Many tools choose that set from the image. Ditherette works the other way round: you bring the
		palette, and every pixel is matched to one of its colours. That suits palettes fixed in advance,
		such as a game's colours, a set of inks or a Wplace canvas.
	</p>

	<GuideFigure
		images={[
			{
				src: noDither,
				alt: 'The sunset matched to four greens without dithering.',
				label: 'No dithering',
				width: 96,
				height: 60,
				pixelated: true
			},
			{
				src: atkinson,
				alt: 'The sunset matched to four greens with Atkinson dithering.',
				label: 'Atkinson',
				width: 96,
				height: 60,
				pixelated: true
			}
		]}
		caption="A four-colour palette: #0F380F, #306230, #8BAC0F and #9BBC0F."
	/>

	<h2>Make your own palette</h2>
	<p>
		Open the palette menu and duplicate a palette, then add colours with New, edit them or delete
		them. Import and Export move custom palettes as JSON files, so you can keep one in a file or
		share it. Custom palettes are saved in your browser.
	</p>

	<h2>Choose how colour difference is measured</h2>
	<p>Colour space decides which palette colour is nearest to each pixel.</p>
	<ul>
		<li>OKLab, the default, measures difference roughly as people see it.</li>
		<li>
			OKLCH and CIELCh compare lightness, chroma and hue, with a circular or Euclidean hue distance.
		</li>
		<li>CIELAB ΔE76 and ΔE2000 are the standard CIE colour-difference formulas.</li>
		<li>sRGB, Linear RGB, the weighted RGB options and YCbCr compare channel values directly.</li>
	</ul>
	<p>There's no single best choice. Try two or three on your image and compare.</p>

	<h2>Resize, then match</h2>
	<p>
		Ditherette runs a fixed order: effects on the source, then resizing, then dithering and matching
		at the output size. Shrinking first averages detail into fewer pixels, so matching sees the
		colours you'll actually keep. Nearest resampling skips the averaging and keeps every edge hard.
	</p>

	<h2>Adjust colours before matching</h2>
	<p>
		Effects run on the full source before resizing, top to bottom in the order you list them. The
		choices are Levels, Curves, Brightness and contrast, Exposure, White balance, Hue and
		saturation, and Palette fit. Palette fit is the one that reads the palette: it adjusts the image
		toward what the visible colours can show. Scopes show a histogram, waveform or vectorscope of
		the source, the image after effects, or the output, so you can see what each effect did.
	</p>

	<h2>Export</h2>
	<p>
		File &gt; Export PNG writes an indexed PNG. Each pixel stores the number of its palette colour,
		and the file's palette holds the colours themselves, so nothing drifts. Transparent pixels stay
		transparent.
	</p>
</GuideArticle>
