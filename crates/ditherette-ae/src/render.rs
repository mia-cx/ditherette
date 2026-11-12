//! Renders one After Effects frame: resize to the pixel grid, dither and match
//! the palette, then scale each cell back up to the layer.
//!
//! The GPU runs the package's `wgpu` stages. Any GPU failure renders the same
//! recipe on the CPU instead. GPU error diffusion uses the banded schedule:
//! deterministic and visually equivalent to the CPU scan, not index-identical.

use std::sync::{Mutex, PoisonError};

use ditherette_wasm::{
    image::contracts::{IndexedImage, PaletteEntry},
    prod::{
        contract::{
            failure::Failure,
            request::{
                self, AlphaPolicy, DitherPolicy, DitherQuantizeRequest, Output, RecipeV1,
                ResizeRequest, Source, DEFAULT_MEMORY_LIMIT_BYTES, RECIPE_VERSION,
            },
        },
        gpu::{Backend, Fallback, GpuTiling},
        pipeline::{
            process::ProcessRequest,
            processor::{InputBoundary, Processor},
            quantize::{IndexedMetadataRef, QuantizeBoundary, QuantizeRequest},
        },
    },
};

use crate::look::Look;

/// Bytes per 8-bit ARGB pixel.
const PIXEL: usize = 4;

/// The editor's default: only fully transparent pixels stay transparent.
const ALPHA: AlphaPolicy = AlphaPolicy::Preserve { threshold: 0.0 };

/// A borrowed 8-bit After Effects frame. Pixels are ARGB; rows lie `stride`
/// bytes apart, and a negative stride stores them bottom-up from the slice start.
#[derive(Clone, Copy)]
pub struct Image<'a> {
    pub width: usize,
    pub height: usize,
    pub stride: isize,
    pub argb: &'a [u8],
}

/// The writable counterpart of [`Image`].
pub struct ImageMut<'a> {
    pub width: usize,
    pub height: usize,
    pub stride: isize,
    pub argb: &'a mut [u8],
}

fn row_start(y: usize, height: usize, stride: isize) -> usize {
    let row = if stride < 0 { height - 1 - y } else { y };
    row * stride.unsigned_abs()
}

impl Image<'_> {
    /// Row `y`, counted from the top: `width` ARGB pixels.
    pub fn row(&self, y: usize) -> &[u8] {
        let start = row_start(y, self.height, self.stride);
        &self.argb[start..start + self.width * PIXEL]
    }
}

impl ImageMut<'_> {
    fn row(&mut self, y: usize) -> &mut [u8] {
        let start = row_start(y, self.height, self.stride);
        &mut self.argb[start..start + self.width * PIXEL]
    }
}

/// Which backend rendered a frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ran {
    Gpu,
    Cpu(Fallback),
}

/// Shared by every render thread. The GPU is probed on first use and serves one frame at a time.
#[derive(Default)]
pub struct Engine {
    gpu: Mutex<Option<Backend>>,
}

/// One frame's validated work, shared by both backends.
struct Job<'a> {
    rgba: Vec<u8>,
    width: u32,
    height: u32,
    output: Output,
    palette: &'a [PaletteEntry],
    look: &'a Look,
}

impl Job<'_> {
    fn resizes(&self) -> bool {
        (self.output.width, self.output.height) != (self.width, self.height)
    }

    fn quantize<'s>(&'s self, source: Source<'s>) -> request::QuantizeRequest<'s> {
        request::QuantizeRequest {
            version: RECIPE_VERSION,
            source,
            palette: self.palette,
            alpha: ALPHA,
            matching: self.look.space.matching,
        }
    }
}

impl Engine {
    /// An engine that never probes the GPU.
    pub fn cpu() -> Self {
        Self {
            gpu: Mutex::new(Some(Backend::Cpu(Fallback::NoAdapter))),
        }
    }

    /// Renders `input` into `output`, which has the same dimensions.
    pub fn render(
        &self,
        look: &Look,
        palette: &[PaletteEntry],
        input: Image<'_>,
        output: &mut ImageMut<'_>,
    ) -> Result<Ran, Failure> {
        let cell = look.pixel_size.max(1) as usize;
        let job = Job {
            rgba: to_rgba(input),
            width: input.width as u32,
            height: input.height as u32,
            output: Output {
                width: input.width.div_ceil(cell) as u32,
                height: input.height.div_ceil(cell) as u32,
                resize: look.resize,
            },
            palette,
            look,
        };
        let (indexed, ran) = match self.gpu(&job) {
            Ok(indexed) => (indexed, Ran::Gpu),
            Err(reason) => (cpu(&job)?, Ran::Cpu(reason)),
        };
        enlarge(&indexed, cell, output);
        Ok(ran)
    }

    fn gpu(&self, job: &Job<'_>) -> Result<IndexedImage, Fallback> {
        let mut slot = self.gpu.lock().unwrap_or_else(PoisonError::into_inner);
        let backend = slot.get_or_insert_with(|| pollster::block_on(Backend::detect()));
        pollster::block_on(async {
            let resized = if job.resizes() {
                let request = ResizeRequest {
                    version: RECIPE_VERSION,
                    source: Source {
                        width: job.width,
                        height: job.height,
                        data: &job.rgba,
                    },
                    output: job.output,
                };
                // Coverage resize keeps transparent pixels from bleeding colour, as in the editor.
                Some(
                    backend
                        .resize(request, true, GpuTiling::Auto, DEFAULT_MEMORY_LIMIT_BYTES)
                        .await?,
                )
            } else {
                None
            };
            let source = Source {
                width: job.output.width,
                height: job.output.height,
                data: resized.as_deref().unwrap_or(&job.rgba),
            };
            let dither = job.look.dither_policy();
            let request = DitherQuantizeRequest {
                quantize: job.quantize(source),
                dither,
            };
            match dither {
                DitherPolicy::Diffusion { .. } => {
                    backend
                        .diffuse_gpu_banded(request, DEFAULT_MEMORY_LIMIT_BYTES)
                        .await
                }
                _ => {
                    backend
                        .dither_and_quantize(request, DEFAULT_MEMORY_LIMIT_BYTES)
                        .await
                }
            }
        })
    }
}

fn cpu(job: &Job<'_>) -> Result<IndexedImage, Failure> {
    let mut processor = Processor::new(DEFAULT_MEMORY_LIMIT_BYTES, 0)?;
    let mut boundary = Frame(&job.rgba);
    let dither = job.look.dither_policy();
    if !job.resizes() {
        let request = QuantizeRequest {
            source_width: job.width,
            source_height: job.height,
            palette: job.palette,
            alpha: ALPHA,
            matching: job.look.space.matching,
        };
        return processor.dither_and_quantize(request, dither, &mut boundary);
    }
    let request = ProcessRequest {
        source_width: job.width,
        source_height: job.height,
        palette: job.palette,
        recipe: RecipeV1 {
            version: RECIPE_VERSION,
            output: job.output,
            alpha: ALPHA,
            matching: job.look.space.matching,
            dither,
        },
    };
    // An empty effect chain still selects the editor's coverage resize.
    processor.process_effects(request, &[], &mut boundary)
}

/// Packed RGBA8 handed to the CPU processor.
struct Frame<'a>(&'a [u8]);

impl InputBoundary for Frame<'_> {
    fn input_len(&mut self) -> Result<usize, Failure> {
        Ok(self.0.len())
    }
    fn copy_input(&mut self, destination: &mut [u8]) -> Result<(), Failure> {
        destination.copy_from_slice(self.0);
        Ok(())
    }
}

impl QuantizeBoundary for Frame<'_> {
    type Output = IndexedImage;
    fn complete(
        &mut self,
        indices: &[u8],
        dimensions: ditherette_wasm::image::ImageDimensions,
        palette: IndexedMetadataRef<'_>,
    ) -> Result<IndexedImage, Failure> {
        Ok(IndexedImage {
            indices: ditherette_wasm::image::ImageBuf::from_vec_packed(
                indices.to_vec(),
                dimensions,
            )
            .expect("the processor validated these dimensions"),
            palette: palette.palette.clone(),
            warnings: palette.warnings.to_vec(),
        })
    }
}

fn to_rgba(image: Image<'_>) -> Vec<u8> {
    let mut rgba = Vec::with_capacity(image.width * image.height * PIXEL);
    for y in 0..image.height {
        for &[alpha, red, green, blue] in image.row(y).as_chunks::<PIXEL>().0 {
            rgba.extend_from_slice(&[red, green, blue, alpha]);
        }
    }
    rgba
}

/// Paints each palette index as a `cell`-pixel square, clipped at the right and bottom edges.
fn enlarge(indexed: &IndexedImage, cell: usize, output: &mut ImageMut<'_>) {
    let columns = indexed.indices.dimensions().width() as usize;
    let indices = indexed.indices.data();
    let colours = indexed.palette.rgba.as_chunks::<PIXEL>().0;
    for y in 0..output.height {
        let cells = &indices[y / cell * columns..][..columns];
        for (x, pixel) in output
            .row(y)
            .as_chunks_mut::<PIXEL>()
            .0
            .iter_mut()
            .enumerate()
        {
            let [red, green, blue, alpha] = colours[usize::from(cells[x / cell])];
            *pixel = [alpha, red, green, blue];
        }
    }
}

// The plugin shares one engine across After Effects' render threads.
const _: fn() = || {
    fn shared<T: Send + Sync>() {}
    shared::<Engine>();
};
