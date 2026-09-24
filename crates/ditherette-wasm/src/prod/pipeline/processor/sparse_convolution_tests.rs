use super::{Boundary, InputBoundary, Processor, ResizeRequest};
use crate::{
    image::ImageDimensions,
    prod::contract::{
        error::ErrorCode,
        failure::{ErrorPath, Failure},
        request::{Anchor, Output, ResizePolicy, Support},
    },
};

struct Io {
    pixels: Vec<u8>,
    sparse: bool,
    fail_gather: bool,
    copies: usize,
    gathers: usize,
}

impl InputBoundary for Io {
    fn input_len(&mut self) -> Result<usize, Failure> {
        Ok(self.pixels.len())
    }
    fn copy_input(&mut self, bytes: &mut [u8]) -> Result<(), Failure> {
        self.copies += 1;
        bytes.copy_from_slice(&self.pixels);
        Ok(())
    }
    fn supports_sparse_input(&self) -> bool {
        self.sparse
    }
    fn gather_input(
        &mut self,
        bytes: &mut [u8],
        columns: &[u8],
        rows: &[u8],
        source_len: usize,
    ) -> Result<(), Failure> {
        self.gathers += 1;
        assert_eq!(source_len, self.pixels.len());
        if self.fail_gather {
            bytes.fill(91);
            return Err(Failure::new(
                ErrorCode::WasmMemoryUnavailable,
                ErrorPath::SourceData,
            ));
        }
        let mut pixels = bytes.chunks_exact_mut(4);
        for y in rows.chunks_exact(4) {
            let y = u32::from_le_bytes(y.try_into().unwrap()) as usize;
            for x in columns.chunks_exact(4) {
                let x = u32::from_le_bytes(x.try_into().unwrap()) as usize;
                pixels
                    .next()
                    .unwrap()
                    .copy_from_slice(&self.pixels[y + x..y + x + 4]);
            }
        }
        assert!(pixels.next().is_none());
        Ok(())
    }
}

impl Boundary for Io {
    type Output = Vec<u8>;
    fn complete(&mut self, bytes: &[u8], _: ImageDimensions) -> Result<Vec<u8>, Failure> {
        Ok(bytes.to_vec())
    }
}

fn request(sw: u32, sh: u32, ow: u32, oh: u32, radius: u32, anchor: Anchor) -> ResizeRequest {
    ResizeRequest {
        source_width: sw,
        source_height: sh,
        output: Output {
            width: ow,
            height: oh,
            resize: if radius == 2 {
                ResizePolicy::Lanczos2 {
                    anchor,
                    support: Support::Fixed,
                }
            } else {
                ResizePolicy::Lanczos3 {
                    anchor,
                    support: Support::Fixed,
                }
            },
        },
    }
}

#[test]
fn sparse_fixed_convolution_is_exact_for_edges_alpha_and_reused_dense_plans() {
    for (sw, sh, ow, oh) in [(320, 240, 16, 12), (65, 49, 4, 3), (17, 17, 1, 1)] {
        for radius in [2, 3] {
            for alpha in [0, 127, 255] {
                for anchor in [
                    Anchor::TopLeft,
                    Anchor::Top,
                    Anchor::TopRight,
                    Anchor::Left,
                    Anchor::Center,
                    Anchor::Right,
                    Anchor::BottomLeft,
                    Anchor::Bottom,
                    Anchor::BottomRight,
                ] {
                    let mut io = Io {
                        pixels: (0..sw * sh * 4)
                            .map(|n| {
                                if n % 4 == 3 {
                                    if alpha == 127 {
                                        (n * 41 + 127) as u8
                                    } else {
                                        alpha
                                    }
                                } else {
                                    (n * 73 + n / 251) as u8
                                }
                            })
                            .collect(),
                        sparse: false,
                        fail_gather: false,
                        copies: 0,
                        gathers: 0,
                    };
                    let request = request(sw, sh, ow, oh, radius, anchor);
                    let mut processor = Processor::new(4 << 20, 0).unwrap();
                    let expected = processor.resize(request, &mut io).unwrap();
                    io.sparse = true;
                    for _ in 0..2 {
                        assert_eq!(
                            processor.resize(request, &mut io).unwrap(),
                            expected,
                            "{sw}x{sh}, r{radius}, {anchor:?}, alpha{alpha}"
                        );
                    }
                    assert_eq!(io.copies, 1);
                    assert_eq!(io.gathers, 2);
                    io.sparse = false;
                    assert_eq!(processor.resize(request, &mut io).unwrap(), expected);
                    assert_eq!(
                        io.copies, 2,
                        "sparse input invalidates the full snapshot identity"
                    );
                }
            }
        }
    }
}

#[test]
fn sparse_fixed_convolution_retries_failed_gathers_and_accounts_compact_storage() {
    let request = request(320, 240, 16, 12, 3, Anchor::Center);
    let mut io = Io {
        pixels: vec![255; 320 * 240 * 4],
        sparse: true,
        fail_gather: false,
        copies: 0,
        gathers: 0,
    };
    let mut processor = Processor::new(64 << 10, 0).unwrap();
    let first = processor.resize(request, &mut io).unwrap();
    assert_eq!(first, vec![255; 16 * 12 * 4]);
    assert!(processor.peak_capacity_bytes() <= 64 << 10);
    io.fail_gather = true;
    assert_eq!(
        processor.resize(request, &mut io).unwrap_err().path,
        ErrorPath::SourceData
    );
    io.fail_gather = false;
    io.pixels.fill(0);
    assert_eq!(
        processor.resize(request, &mut io).unwrap(),
        vec![0; 16 * 12 * 4]
    );
    assert_eq!(io.copies, 0);
    assert_eq!(io.gathers, 3);
    assert_eq!(
        Processor::new(Processor::bookkeeping_bytes(0) + 1024, 0)
            .unwrap()
            .resize(request, &mut io)
            .unwrap_err()
            .code,
        ErrorCode::MemoryLimit
    );
    assert_eq!(io.gathers, 3, "preflight rejects before gathering");
}
