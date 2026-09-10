//! Scalar SHA-256 for Wasm source and materialized RGBA identities.
//!
//! Compression and core setup adapted from RustCrypto sha2 0.10.9:
//! https://github.com/RustCrypto/hashes/tree/sha2-v0.10.9/sha2/src
//! (`sha256/soft_compact.rs`, `core_api.rs`, and `consts.rs`).
//! Native production retains sha2's hardware dispatch. RustCrypto's digest wrapper
//! supplies buffering and padding; this core uses fixed stack storage only.
//!
//! Copyright (c) 2006-2009 Graydon Hoare
//! Copyright (c) 2009-2013 Mozilla Foundation
//! Copyright (c) 2016 Artyom Pavlov
//!
//! Permission is hereby granted, free of charge, to any person obtaining a copy
//! of this software and associated documentation files (the "Software"), to deal
//! in the Software without restriction, including without limitation the rights
//! to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
//! copies of the Software, and to permit persons to whom the Software is
//! furnished to do so, subject to the following conditions:
//!
//! The above copyright notice and this permission notice shall be included in
//! all copies or substantial portions of the Software.
//!
//! THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
//! IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
//! FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
//! AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
//! LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
//! OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
//! THE SOFTWARE.

use sha2::digest::{
    block_buffer::Eager,
    core_api::{
        Block, BlockSizeUser, Buffer, BufferKindUser, CoreWrapper, FixedOutputCore, OutputSizeUser,
        UpdateCore,
    },
    typenum::{U32, U64},
    HashMarker, Output,
};

pub(super) type Sha256 = CoreWrapper<ScalarCore>;

#[derive(Clone)]
pub(super) struct ScalarCore {
    state: [u32; 8],
    block_len: u64,
}

impl Default for ScalarCore {
    fn default() -> Self {
        Self {
            state: [
                0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
                0x5be0cd19,
            ],
            block_len: 0,
        }
    }
}

impl HashMarker for ScalarCore {}

impl BlockSizeUser for ScalarCore {
    type BlockSize = U64;
}

impl BufferKindUser for ScalarCore {
    type BufferKind = Eager;
}

impl OutputSizeUser for ScalarCore {
    type OutputSize = U32;
}

impl UpdateCore for ScalarCore {
    #[inline]
    fn update_blocks(&mut self, blocks: &[Block<Self>]) {
        self.block_len += blocks.len() as u64;
        for block in blocks {
            compress(&mut self.state, block);
        }
    }
}

impl FixedOutputCore for ScalarCore {
    #[inline]
    fn finalize_fixed_core(&mut self, buffer: &mut Buffer<Self>, out: &mut Output<Self>) {
        let bit_len = 8 * (buffer.get_pos() as u64 + 64 * self.block_len);
        buffer.len64_padding_be(bit_len, |block| compress(&mut self.state, block));
        for (chunk, word) in out.chunks_exact_mut(4).zip(self.state) {
            chunk.copy_from_slice(&word.to_be_bytes());
        }
    }
}

const K32: [u32; 64] = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
];

// Scalar rounds avoid sha2's emulated four-word SHA instruction helpers on Wasm.
fn compress(state: &mut [u32; 8], block: &Block<ScalarCore>) {
    let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h] = *state;
    let mut w = [0u32; 64];
    for (word, chunk) in w[..16].iter_mut().zip(block.chunks_exact(4)) {
        *word = u32::from_be_bytes(chunk.try_into().unwrap());
    }
    for i in 16..64 {
        let w15 = w[i - 15];
        let s0 = w15.rotate_right(7) ^ w15.rotate_right(18) ^ (w15 >> 3);
        let w2 = w[i - 2];
        let s1 = w2.rotate_right(17) ^ w2.rotate_right(19) ^ (w2 >> 10);
        w[i] = w[i - 16]
            .wrapping_add(s0)
            .wrapping_add(w[i - 7])
            .wrapping_add(s1);
    }
    for i in 0..64 {
        let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
        let ch = (e & f) ^ ((!e) & g);
        let t1 = s1
            .wrapping_add(ch)
            .wrapping_add(K32[i])
            .wrapping_add(w[i])
            .wrapping_add(h);
        let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
        let maj = (a & b) ^ (a & c) ^ (b & c);
        let t2 = s0.wrapping_add(maj);

        h = g;
        g = f;
        f = e;
        e = d.wrapping_add(t1);
        d = c;
        c = b;
        b = a;
        a = t1.wrapping_add(t2);
    }
    for (word, value) in state.iter_mut().zip([a, b, c, d, e, f, g, h]) {
        *word = word.wrapping_add(value);
    }
}

#[cfg(test)]
mod tests {
    use super::Sha256;
    use crate::prod::contract::{cache::source_identity, request::Source};
    use sha2::{Digest, Sha256 as Reference};

    fn bytes(len: usize) -> Vec<u8> {
        (0..len)
            .map(|i| (i.wrapping_mul(131) ^ (i >> 7)) as u8)
            .collect()
    }

    #[test]
    fn scalar_matches_sha2_across_padding_and_block_boundaries() {
        let input = bytes(1024);
        for len in 0..=input.len() {
            assert_eq!(
                Sha256::digest(&input[..len]),
                Reference::digest(&input[..len]),
                "length {len}"
            );
        }
    }

    #[test]
    fn scalar_matches_sha2_for_split_and_empty_updates() {
        let input = bytes(257);
        let expected = Reference::digest(&input);
        for split in 0..=input.len() {
            let mut hash = Sha256::new();
            hash.update(&input[..split]);
            hash.update([]);
            hash.update(&input[split..]);
            assert_eq!(hash.finalize(), expected, "split {split}");
        }
        for chunk_size in [1, 3, 31, 55, 56, 63, 64, 65, 127] {
            let mut hash = Sha256::new();
            for chunk in input.chunks(chunk_size) {
                hash.update(chunk);
            }
            assert_eq!(hash.finalize(), expected, "chunk size {chunk_size}");
        }
    }

    #[test]
    fn scalar_matches_sha2_for_large_inputs() {
        let input = bytes(1024 * 1024 + 63);
        let expected = Reference::digest(&input);
        assert_eq!(Sha256::digest(&input), expected);
        let mut hash = Sha256::new();
        for chunk in input.chunks(4093) {
            hash.update(chunk);
        }
        assert_eq!(hash.finalize(), expected);
    }

    #[test]
    fn scalar_preserves_source_domain_dimensions_and_rgba_bytes() {
        for (width, height) in [
            (1u32, 1u32),
            (2, 1),
            (1, 2),
            (6, 1),
            (8, 1),
            (17, 31),
            (257, 263),
        ] {
            let input = bytes(width as usize * height as usize * 4);
            let mut expected = Reference::new();
            let mut scalar = Sha256::new();
            for part in [
                b"ditherette-rgba8-input-v1\0".as_slice(),
                width.to_le_bytes().as_slice(),
                height.to_le_bytes().as_slice(),
                input.as_slice(),
            ] {
                expected.update(part);
                scalar.update(part);
            }
            let expected: [u8; 32] = expected.finalize().into();
            let actual: [u8; 32] = scalar.finalize().into();
            assert_eq!(actual, expected, "dimensions {width} x {height}");
            assert_eq!(
                source_identity(Source {
                    width,
                    height,
                    data: &input
                })
                .0,
                expected
            );
        }
    }
}
