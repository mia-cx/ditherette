//! One instance's shared preparation/image LRU and success-only publication.

use std::mem::size_of;

use super::{
    identity,
    processor::Allocator,
    quantize::{preparation_failure, IndexedMetadataRef, QuantizeRequest},
    resize::PreparedResize,
    stages::{ImageStage, IndexedMetadata, Metadata},
};
use crate::{
    image::ImageDimensions,
    prod::{
        contract::{
            cache::{source_identity, Identity},
            error::ErrorCode,
            failure::{ErrorPath, Failure},
            lifecycle::{MAX_CACHE_BYTES, MAX_CACHE_ENTRIES},
            request::{Output, Source},
        },
        quantize::PreparedQuantizer,
        resize::common::allocation::CapacityBudget,
    },
};

enum Value {
    Palette(Vec<PreparedQuantizer>),
    Resize(Vec<PreparedResize>),
    Image(Vec<ImageStage>),
}

struct Entry {
    key: Identity,
    used: u64,
    value: Value,
}

impl Entry {
    fn capacity(&self) -> u64 {
        match &self.value {
            Value::Palette(value) => {
                value[0].capacity_bytes()
                    + (value.capacity() - 1) as u64 * size_of::<PreparedQuantizer>() as u64
            }
            Value::Resize(value) => {
                value[0].capacity_bytes()
                    + value.capacity() as u64 * size_of::<PreparedResize>() as u64
            }
            Value::Image(value) => {
                value[0].capacity_bytes() + value.capacity() as u64 * size_of::<ImageStage>() as u64
            }
        }
    }

    fn scratch(&self) -> u64 {
        match &self.value {
            Value::Resize(value) => value[0].scratch_capacity_bytes(),
            _ => 0,
        }
    }

    fn retained(&self) -> u64 {
        size_of::<Self>() as u64 + self.capacity() - self.scratch()
    }

    fn required(&self) -> Result<u64, Failure> {
        Ok(self.capacity()
            + match &self.value {
                Value::Resize(value) => value[0]
                    .required_scratch_bytes()?
                    .saturating_sub(self.scratch()),
                _ => 0,
            })
    }

    fn drop_scratch(&mut self) {
        if let Value::Resize(value) = &mut self.value {
            value[0].drop_scratch();
        }
    }
}

#[derive(Default)]
pub(super) struct Scratch {
    pub buffers: [Vec<u8>; 4],
    pub diffusion: Vec<[f32; 3]>,
}

impl Scratch {
    fn capacity(&self) -> u64 {
        self.buffers
            .iter()
            .map(|buffer| buffer.capacity() as u64)
            .sum::<u64>()
            + (self.diffusion.capacity() * size_of::<[f32; 3]>()) as u64
    }
}

pub(super) struct Store {
    entries: [Option<Entry>; MAX_CACHE_ENTRIES],
    scratch: Scratch,
    clock: u64,
    hits: u64,
    misses: u64,
    image_hits: u64,
    image_misses: u64,
}

impl Default for Store {
    fn default() -> Self {
        Self {
            entries: std::array::from_fn(|_| None),
            scratch: Scratch::default(),
            clock: 0,
            hits: 0,
            misses: 0,
            image_hits: 0,
            image_misses: 0,
        }
    }
}

impl std::fmt::Debug for Store {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PreparationStore")
            .field("capacity", &self.capacity())
            .finish()
    }
}

impl Store {
    #[cfg(test)]
    pub(super) fn image_stats(&self) -> (usize, u64, u64) {
        (
            self.entries
                .iter()
                .flatten()
                .filter(|entry| matches!(entry.value, Value::Image(_)))
                .count(),
            self.image_hits,
            self.image_misses,
        )
    }

    #[cfg(test)]
    pub(super) fn evict_preparation(&mut self) {
        for entry in &mut self.entries {
            if entry
                .as_ref()
                .is_some_and(|entry| !matches!(entry.value, Value::Image(_)))
            {
                *entry = None;
            }
        }
    }
    #[cfg(test)]
    pub(super) fn stats(&self) -> (usize, u64, u64, u64, u64) {
        (
            self.entries.iter().flatten().count(),
            self.hits,
            self.misses,
            self.entries.iter().flatten().map(Entry::retained).sum(),
            self.scratch.capacity()
                + self
                    .entries
                    .iter()
                    .flatten()
                    .map(Entry::scratch)
                    .sum::<u64>(),
        )
    }

    fn capacity(&self) -> u64 {
        self.scratch.capacity()
            + self
                .entries
                .iter()
                .flatten()
                .map(Entry::capacity)
                .sum::<u64>()
    }

    fn take(&mut self, key: Identity) -> Option<Entry> {
        let index = self
            .entries
            .iter()
            .position(|entry| entry.as_ref().is_some_and(|entry| entry.key == key));
        if let Some(index) = index {
            self.hits += 1;
            self.clock += 1;
            let mut entry = self.entries[index].take().unwrap();
            entry.used = self.clock;
            Some(entry)
        } else {
            self.misses += 1;
            None
        }
    }

    fn evict(&mut self) -> bool {
        let index = self
            .entries
            .iter()
            .enumerate()
            .filter_map(|(index, entry)| entry.as_ref().map(|entry| (index, entry.used)))
            .min_by_key(|(_, used)| *used)
            .map(|(index, _)| index);
        if let Some(index) = index {
            self.entries[index] = None;
            true
        } else {
            false
        }
    }

    fn room(&mut self, needed: u64, limit: u64) -> Result<(), Failure> {
        if needed + self.capacity() <= limit {
            return Ok(());
        }
        self.scratch = Scratch::default();
        for entry in self.entries.iter_mut().flatten() {
            entry.drop_scratch();
        }
        while needed + self.capacity() > limit {
            if !self.evict() {
                return Err(memory_limit());
            }
        }
        Ok(())
    }

    fn publish(&mut self, mut entry: Entry, limit: u64) {
        let cap = MAX_CACHE_BYTES.min(limit / 4);
        if entry.retained() > cap {
            return;
        }
        while self
            .entries
            .iter()
            .flatten()
            .map(Entry::retained)
            .sum::<u64>()
            + entry.retained()
            > cap
            || self.entries.iter().all(Option::is_some)
        {
            self.evict();
        }
        if entry.used == 0 {
            self.clock += 1;
            entry.used = self.clock;
        }
        *self.entries.iter_mut().find(|slot| slot.is_none()).unwrap() = Some(entry);
    }
}

#[derive(Clone, Copy)]
pub(super) struct ResizePreparation {
    pub source: ImageDimensions,
    pub output: Output,
}

pub(super) struct Call<'a> {
    store: &'a mut Store,
    palette: Option<Entry>,
    resize: Option<Entry>,
    palette_hit: bool,
    resize_hit: bool,
    images: [Option<Entry>; 3],
    image_hits: [bool; 3],
    pub scratch: Scratch,
    success: bool,
    limit: u64,
    overhead: u64,
}

impl<'a> Call<'a> {
    pub(super) const fn record_bytes() -> u64 {
        size_of::<Self>() as u64
    }

    /// Validation happens at each existing adapter before any lookup. Hits leave the LRU
    /// during this call, so pressure cannot evict either preparation pinned by Process.
    pub(super) fn new<A: Allocator>(
        store: &'a mut Store,
        palette: Option<QuantizeRequest<'_>>,
        resize: Option<ResizePreparation>,
        lengths: [usize; 4],
        diffusion_len: usize,
        overhead: u64,
        limit: u64,
        peak: &mut u64,
        allocator: &mut A,
    ) -> Result<Self, Failure> {
        let mut call = Self {
            palette_hit: false,
            resize_hit: false,
            palette: None,
            resize: None,
            images: std::array::from_fn(|_| None),
            image_hits: [false; 3],
            scratch: std::mem::take(&mut store.scratch),
            store,
            success: false,
            limit,
            overhead,
        };
        call.prepare(palette, resize, lengths, diffusion_len, peak, allocator)?;
        Ok(call)
    }

    /// Reserve only the owned input snapshot before content-key lookup.
    pub(super) fn snapshot<A: Allocator>(
        store: &'a mut Store,
        source_len: usize,
        overhead: u64,
        limit: u64,
        peak: &mut u64,
        allocator: &mut A,
    ) -> Result<Self, Failure> {
        Self::new(
            store,
            None,
            None,
            [source_len, 0, 0, 0],
            0,
            overhead,
            limit,
            peak,
            allocator,
        )
    }

    /// After source hashing and available image hits, reserve the remaining execution.
    pub(super) fn prepare<A: Allocator>(
        &mut self,
        palette: Option<QuantizeRequest<'_>>,
        resize: Option<ResizePreparation>,
        lengths: [usize; 4],
        diffusion_len: usize,
        peak: &mut u64,
        allocator: &mut A,
    ) -> Result<(), Failure> {
        let call = self;
        let limit = call.limit;
        let overhead = call.overhead
            + call
                .images
                .iter()
                .flatten()
                .map(Entry::capacity)
                .sum::<u64>();
        let palette_key = palette
            .map(|request| identity::palette(request.palette, request.alpha, request.matching))
            .transpose()?;
        let resize_key = resize
            .map(|request| {
                identity::resize(
                    request.source.width(),
                    request.source.height(),
                    request.output,
                )
            })
            .transpose()?;
        call.palette = palette_key.and_then(|key| call.store.take(key));
        call.resize = resize_key.and_then(|key| call.store.take(key));
        call.palette_hit = call.palette.is_some();
        call.resize_hit = call.resize.is_some();
        let palette_required = palette
            .map(|request| {
                PreparedQuantizer::required_capacity_bytes(
                    request.palette,
                    request.alpha,
                    request.matching,
                )
                .map_err(preparation_failure)
            })
            .transpose()?
            .unwrap_or(0);
        let resize_required = resize
            .map(|request| {
                PreparedResize::required_bytes(
                    request.source,
                    ImageDimensions::new(request.output.width, request.output.height)
                        .expect("validated dimensions"),
                    request.output.resize,
                )
                .map(|bytes| bytes + size_of::<PreparedResize>() as u64)
            })
            .transpose()?
            .unwrap_or(0);
        let planned = |call: &Self| -> Result<u64, Failure> {
            Ok(overhead
                + call
                    .palette
                    .as_ref()
                    .map_or(palette_required, Entry::capacity)
                + call
                    .resize
                    .as_ref()
                    .map_or(Ok(resize_required), Entry::required)?
                + lengths
                    .iter()
                    .zip(&call.scratch.buffers)
                    .map(|(&length, buffer)| length.max(buffer.capacity()) as u64)
                    .sum::<u64>()
                + diffusion_len.max(call.scratch.diffusion.capacity()) as u64
                    * size_of::<[f32; 3]>() as u64)
        };
        // These buffers were idle on entry. Release excess capacity before evicting any plan.
        if planned(call)? + call.store.capacity() > limit {
            // The first buffer may already hold the hashed input snapshot.
            for buffer in &mut call.scratch.buffers[1..] {
                *buffer = Vec::new();
            }
            call.scratch.diffusion = Vec::new();
            if call.scratch.buffers[0].is_empty() {
                call.scratch.buffers[0] = Vec::new();
            }
            if let Some(entry) = &mut call.resize {
                entry.drop_scratch();
            }
        }
        call.store.room(planned(call)?, limit)?;
        let buffers_required = lengths
            .iter()
            .zip(&call.scratch.buffers)
            .map(|(&length, buffer)| length.max(buffer.capacity()) as u64)
            .sum::<u64>()
            + diffusion_len.max(call.scratch.diffusion.capacity()) as u64
                * size_of::<[f32; 3]>() as u64;
        if let Some(request) = palette {
            if call.palette.is_none() {
                let budget = limit
                    - overhead
                    - call.store.capacity()
                    - buffers_required
                    - call
                        .resize
                        .as_ref()
                        .map_or(Ok(resize_required), Entry::required)?;
                let mut record = CapacityBudget::new(budget).vector::<PreparedQuantizer>(1)?;
                let extra = (record.capacity() - 1) as u64 * size_of::<PreparedQuantizer>() as u64;
                record.push(
                    PreparedQuantizer::try_new(
                        request.palette,
                        request.alpha,
                        request.matching,
                        budget - extra,
                    )
                    .map_err(preparation_failure)?,
                );
                call.palette = Some(Entry {
                    key: palette_key.unwrap(),
                    used: 0,
                    value: Value::Palette(record),
                });
            }
        }
        if let Some(request) = resize {
            let budget = limit
                - overhead
                - call.store.capacity()
                - buffers_required
                - call.palette.as_ref().map_or(0, Entry::capacity);
            if let Some(entry) = &mut call.resize {
                let Value::Resize(record) = &mut entry.value else {
                    unreachable!()
                };
                let record_bytes = (record.capacity() * size_of::<PreparedResize>()) as u64;
                record[0].restore_scratch(budget - record_bytes)?;
            } else {
                let mut record = CapacityBudget::new(budget).vector::<PreparedResize>(1)?;
                let record_bytes = (record.capacity() * size_of::<PreparedResize>()) as u64;
                record.push(PreparedResize::new(
                    request.source,
                    ImageDimensions::new(request.output.width, request.output.height)
                        .expect("validated dimensions"),
                    request.output.resize,
                    budget - record_bytes,
                )?);
                call.resize = Some(Entry {
                    key: resize_key.unwrap(),
                    used: 0,
                    value: Value::Resize(record),
                });
            }
        }
        let owned = overhead
            + call.store.capacity()
            + call.palette.as_ref().map_or(0, Entry::capacity)
            + call.resize.as_ref().map_or(0, Entry::capacity);
        let mut remaining = buffers_required;
        let mut actual = owned;
        for (buffer, length) in call.scratch.buffers.iter_mut().zip(lengths) {
            let planned_capacity = length.max(buffer.capacity()) as u64;
            if buffer.capacity() < length {
                *buffer = Vec::new();
                allocator.reserve(buffer, length)?;
            }
            actual += buffer.capacity() as u64;
            remaining -= planned_capacity;
            *peak = (*peak).max(actual);
            if actual + remaining > limit {
                return Err(memory_limit());
            }
            buffer.resize(length, 0);
        }
        if call.scratch.diffusion.capacity() < diffusion_len {
            call.scratch.diffusion = Vec::new();
            call.scratch.diffusion = CapacityBudget::new(limit - actual).vector(diffusion_len)?;
        }
        actual += (call.scratch.diffusion.capacity() * size_of::<[f32; 3]>()) as u64;
        *peak = (*peak).max(actual);
        if actual > limit {
            return Err(memory_limit());
        }
        call.scratch.diffusion.resize(diffusion_len, [0.0; 3]);
        Ok(())
    }

    pub(super) fn parts(
        &mut self,
    ) -> (
        Option<&PreparedQuantizer>,
        Option<&mut PreparedResize>,
        &mut Scratch,
    ) {
        let palette = self.palette.as_ref().map(|entry| match &entry.value {
            Value::Palette(value) => &value[0],
            _ => unreachable!(),
        });
        let resize = self.resize.as_mut().map(|entry| match &mut entry.value {
            Value::Resize(value) => &mut value[0],
            _ => unreachable!(),
        });
        (palette, resize, &mut self.scratch)
    }

    pub(super) fn image_parts(
        &mut self,
    ) -> (
        Option<&PreparedQuantizer>,
        Option<&mut PreparedResize>,
        [Option<&ImageStage>; 3],
        &mut Scratch,
    ) {
        let palette = self.palette.as_ref().map(|entry| match &entry.value {
            Value::Palette(value) => &value[0],
            _ => unreachable!(),
        });
        let resize = self.resize.as_mut().map(|entry| match &mut entry.value {
            Value::Resize(value) => &mut value[0],
            _ => unreachable!(),
        });
        let images = self.images.each_ref().map(|entry| {
            entry.as_ref().map(|entry| match &entry.value {
                Value::Image(value) => &value[0],
                _ => unreachable!(),
            })
        });
        (palette, resize, images, &mut self.scratch)
    }

    pub(super) fn content(
        &self,
        slot: usize,
        buffer: usize,
        dimensions: ImageDimensions,
    ) -> Identity {
        if let Some(image) = self.image(slot) {
            let Metadata::Rgba { content } = image.metadata else {
                unreachable!()
            };
            return content;
        }
        source_identity(Source {
            width: dimensions.width(),
            height: dimensions.height(),
            data: &self.scratch.buffers[buffer],
        })
    }

    pub(super) fn retain_rgba(
        &mut self,
        slot: usize,
        key: Identity,
        buffer: usize,
        dimensions: ImageDimensions,
        content: Identity,
        peak: &mut u64,
    ) {
        if self.image(slot).is_some() {
            return;
        }
        let image = ImageStage {
            dimensions,
            bytes: std::mem::take(&mut self.scratch.buffers[buffer]),
            metadata: Metadata::Rgba { content },
        };
        if let Err(image) = self.stage_image(slot, key, image, peak) {
            self.scratch.buffers[buffer] = image.bytes;
        }
    }

    /// Reserve optional durable metadata before final output construction. A retention
    /// budget/allocation miss keeps the completed work and its prepared metadata usable.
    pub(super) fn retain_indexed(
        &mut self,
        key: Identity,
        buffer: usize,
        dimensions: ImageDimensions,
        peak: &mut u64,
    ) {
        if self.image(2).is_some() {
            return;
        }
        let Value::Palette(prepared) = &self.palette.as_ref().expect("requested palette").value
        else {
            unreachable!()
        };
        let palette = prepared[0].palette();
        let needed = IndexedMetadata::required_bytes(&palette.palette, &palette.warnings);
        let retained = needed
            + self.scratch.buffers[buffer].capacity() as u64
            + size_of::<ImageStage>() as u64
            + size_of::<Entry>() as u64;
        if retained > MAX_CACHE_BYTES.min(self.limit / 4) {
            return;
        }
        let owned = self.active_capacity();
        if self
            .store
            .room(owned + needed + size_of::<ImageStage>() as u64, self.limit)
            .is_err()
        {
            return;
        }
        let Value::Palette(prepared) = &self.palette.as_ref().unwrap().value else {
            unreachable!()
        };
        let palette = prepared[0].palette();
        let budget = self.limit - owned - self.store.capacity() - size_of::<ImageStage>() as u64;
        let Ok(metadata) = IndexedMetadata::try_copy(&palette.palette, &palette.warnings, budget)
        else {
            return;
        };
        *peak = (*peak).max(owned + self.store.capacity() + metadata.capacity_bytes());
        let image = ImageStage {
            dimensions,
            bytes: std::mem::take(&mut self.scratch.buffers[buffer]),
            metadata: Metadata::Indexed(metadata),
        };
        if let Err(image) = self.stage_image(2, key, image, peak) {
            self.scratch.buffers[buffer] = image.bytes;
        }
    }

    pub(super) fn indexed_result(&self, buffer: usize) -> (&[u8], IndexedMetadataRef<'_>) {
        if let Some(image) = self.image(2) {
            let Metadata::Indexed(metadata) = &image.metadata else {
                unreachable!()
            };
            return (
                &image.bytes,
                IndexedMetadataRef {
                    palette: &metadata.palette,
                    warnings: &metadata.warnings,
                },
            );
        }
        let Value::Palette(prepared) = &self.palette.as_ref().expect("requested palette").value
        else {
            unreachable!()
        };
        (&self.scratch.buffers[buffer], prepared[0].palette().into())
    }

    fn active_capacity(&self) -> u64 {
        self.overhead
            + self.scratch.capacity()
            + self.palette.as_ref().map_or(0, Entry::capacity)
            + self.resize.as_ref().map_or(0, Entry::capacity)
            + self
                .images
                .iter()
                .flatten()
                .map(Entry::capacity)
                .sum::<u64>()
    }

    /// Pin a published materialized stage. Pending candidates are never lookup sources.
    pub(super) fn take_image(&mut self, slot: usize, key: Identity) -> bool {
        assert!(self.images[slot].is_none());
        self.images[slot] = self.store.take(key);
        self.image_hits[slot] = self.images[slot].is_some();
        if self.image_hits[slot] {
            self.store.image_hits += 1;
        } else {
            self.store.image_misses += 1;
        }
        self.image_hits[slot]
    }

    pub(super) fn image(&self, slot: usize) -> Option<&ImageStage> {
        self.images[slot].as_ref().map(|entry| match &entry.value {
            Value::Image(value) => &value[0],
            _ => unreachable!("typed image-stage identity"),
        })
    }

    /// Transfer completed work without copying its pixels. Optional retention returns
    /// the original owner unchanged when either cap or the record reservation prevents it.
    pub(super) fn stage_image(
        &mut self,
        slot: usize,
        key: Identity,
        image: ImageStage,
        peak: &mut u64,
    ) -> Result<(), ImageStage> {
        assert!(self.images[slot].is_none());
        if self
            .images
            .iter()
            .chain(self.store.entries.iter())
            .flatten()
            .any(|entry| entry.key == key)
        {
            return Err(image);
        }
        let required = image.capacity_bytes() + size_of::<ImageStage>() as u64;
        let retained = required + size_of::<Entry>() as u64;
        if retained > MAX_CACHE_BYTES.min(self.limit / 4) {
            return Err(image);
        }
        let owned = self.active_capacity() + image.capacity_bytes();
        if self
            .store
            .room(owned + size_of::<ImageStage>() as u64, self.limit)
            .is_err()
        {
            return Err(image);
        }
        let budget = self.limit - owned - self.store.capacity();
        let Ok(mut record) = CapacityBudget::new(budget).vector::<ImageStage>(1) else {
            return Err(image);
        };
        if image.capacity_bytes()
            + record.capacity() as u64 * size_of::<ImageStage>() as u64
            + size_of::<Entry>() as u64
            > MAX_CACHE_BYTES.min(self.limit / 4)
        {
            return Err(image);
        }
        *peak = (*peak).max(
            owned
                + self.store.capacity()
                + record.capacity() as u64 * size_of::<ImageStage>() as u64,
        );
        record.push(image);
        self.images[slot] = Some(Entry {
            key,
            used: 0,
            value: Value::Image(record),
        });
        Ok(())
    }

    pub(super) fn finish<T>(mut self, result: Result<T, Failure>) -> Result<T, Failure> {
        self.success = result.is_ok();
        result
    }
}

impl Drop for Call<'_> {
    fn drop(&mut self) {
        for (entry, hit) in [
            (&mut self.palette, self.palette_hit),
            (&mut self.resize, self.resize_hit),
        ] {
            if let Some(mut entry) = entry.take() {
                if self.success {
                    self.store.publish(entry, self.limit);
                } else if hit {
                    entry.drop_scratch();
                    *self
                        .store
                        .entries
                        .iter_mut()
                        .find(|slot| slot.is_none())
                        .unwrap() = Some(entry);
                }
            }
        }
        for (entry, hit) in self.images.iter_mut().zip(self.image_hits) {
            if let Some(entry) = entry.take() {
                if self.success || hit {
                    self.store.publish(entry, self.limit);
                }
            }
        }
        if self.success {
            for buffer in &mut self.scratch.buffers {
                buffer.clear();
            }
            self.scratch.diffusion.clear();
            self.store.scratch = std::mem::take(&mut self.scratch);
        }
    }
}

fn memory_limit() -> Failure {
    Failure::new(ErrorCode::MemoryLimit, ErrorPath::MemoryLimitBytes)
}

pub(super) fn source_key(bytes: &[u8], dimensions: ImageDimensions) -> Identity {
    source_identity(Source {
        width: dimensions.width(),
        height: dimensions.height(),
        data: bytes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prod::contract::request::{Anchor, ResizePolicy};
    use crate::prod::pipeline::processor::SystemAllocator;
    use crate::prod::pipeline::stages::Metadata;

    fn image_call(store: &mut Store, length: usize, limit: u64) -> Call<'_> {
        Call::new(
            store,
            None,
            None,
            [4, length, length, 0],
            0,
            size_of::<Store>() as u64 + Call::record_bytes(),
            limit,
            &mut 0,
            &mut SystemAllocator,
        )
        .unwrap()
    }

    fn candidate(call: &mut Call<'_>, buffer: usize, byte: u8) -> ImageStage {
        let mut bytes = std::mem::take(&mut call.scratch.buffers[buffer]);
        bytes.fill(byte);
        ImageStage {
            dimensions: ImageDimensions::new((bytes.len() / 4) as u32, 1).unwrap(),
            bytes,
            metadata: Metadata::Rgba {
                content: Identity([byte; 32]),
            },
        }
    }

    #[test]
    fn two_pending_images_publish_only_after_success_and_hits_remain_pinned() {
        let mut store = Store::default();
        let limit = 1 << 20;
        for success in [false, true] {
            let mut call = image_call(&mut store, 16, limit);
            for slot in 0..2 {
                let before = call.active_capacity();
                let image = candidate(&mut call, slot + 1, slot as u8 + 1);
                assert!(call
                    .stage_image(slot, Identity([slot as u8; 32]), image, &mut 0)
                    .is_ok());
                assert_eq!(
                    call.active_capacity(),
                    before + size_of::<ImageStage>() as u64
                );
            }
            assert!(call.store.entries.iter().all(Option::is_none));
            let result = if success { Ok(()) } else { Err(memory_limit()) };
            assert_eq!(call.finish(result).is_ok(), success);
            assert_eq!(store.stats().0, if success { 2 } else { 0 });
        }
        let mut call = image_call(&mut store, 0, limit);
        assert!(call.take_image(0, Identity([0; 32])));
        let image = call.image(0).unwrap();
        assert_eq!(image.bytes, [1; 16]);
        assert_eq!(image.dimensions.width(), 4);
        assert!(matches!(
            image.metadata,
            Metadata::Rgba {
                content: Identity([1, ..])
            }
        ));
        call.store.room(limit, limit).unwrap();
        assert_eq!(call.store.stats().0, 0);
        assert_eq!(call.image(0).unwrap().bytes, [1; 16]);
        assert!(call.finish::<()>(Err(memory_limit())).is_err());
        assert_eq!(store.stats().0, 1);
    }

    #[test]
    fn oversized_image_keeps_its_owner_and_success_does_not_publish_it() {
        let mut store = Store::default();
        let limit = 64 * 1024;
        let mut call = image_call(&mut store, 18 * 1024, limit);
        let image = candidate(&mut call, 1, 7);
        let returned = call
            .stage_image(0, Identity([3; 32]), image, &mut 0)
            .err()
            .unwrap();
        assert_eq!(returned.bytes, vec![7; 18 * 1024]);
        call.finish(Ok(())).unwrap();
        assert_eq!(store.stats().0, 0);
    }

    #[test]
    fn image_and_preparation_entries_share_count_byte_caps_and_lru() {
        let mut store = Store::default();
        let limit = 1 << 20;
        prepare(&mut store, 17, limit);
        let preparation_key = store.entries.iter().flatten().next().unwrap().key;
        for byte in 0..MAX_CACHE_ENTRIES as u8 {
            let mut call = image_call(&mut store, 4, limit);
            let image = candidate(&mut call, 1, byte);
            assert!(call
                .stage_image(0, Identity([byte; 32]), image, &mut 0)
                .is_ok());
            call.finish(Ok(())).unwrap();
            assert!(store.stats().3 <= limit / 4);
        }
        assert_eq!(store.stats().0, MAX_CACHE_ENTRIES);
        assert!(!store
            .entries
            .iter()
            .flatten()
            .any(|entry| entry.key == preparation_key));
        let mut call = image_call(&mut store, 4, limit);
        assert!(call.take_image(0, Identity([0; 32])));
        let image = candidate(&mut call, 1, 200);
        assert!(call
            .stage_image(1, Identity([200; 32]), image, &mut 0)
            .is_ok());
        call.finish(Ok(())).unwrap();
        assert!(store
            .entries
            .iter()
            .flatten()
            .any(|entry| entry.key == Identity([0; 32])));
        assert!(!store
            .entries
            .iter()
            .flatten()
            .any(|entry| entry.key == Identity([1; 32])));
        let cap = store.stats().3 - 1;
        let mut call = image_call(&mut store, 4, cap * 4);
        let image = candidate(&mut call, 1, 201);
        assert!(call
            .stage_image(0, Identity([201; 32]), image, &mut 0)
            .is_ok());
        call.finish(Ok(())).unwrap();
        assert!(store.stats().3 <= cap);
    }

    #[test]
    fn publication_preserves_lookup_recency_across_entry_kinds() {
        for success in [true, false] {
            let mut store = Store::default();
            let limit = 1 << 20;
            prepare(&mut store, 17, limit);
            let preparation_key = store.entries.iter().flatten().next().unwrap().key;
            let image_key = Identity([5; 32]);
            let mut call = image_call(&mut store, 4, limit);
            let image = candidate(&mut call, 1, 5);
            assert!(call.stage_image(0, image_key, image, &mut 0).is_ok());
            call.finish(Ok(())).unwrap();
            let mut call = image_call(&mut store, 0, limit);
            assert!(call.take_image(0, image_key));
            call.resize = call.store.take(preparation_key);
            call.resize_hit = true;
            assert_eq!(
                call.finish(if success { Ok(()) } else { Err(memory_limit()) })
                    .is_ok(),
                success
            );
            store.scratch = Scratch::default();
            for entry in store.entries.iter_mut().flatten() {
                entry.drop_scratch();
            }
            store.room(limit - store.capacity() + 1, limit).unwrap();
            assert!(store
                .entries
                .iter()
                .flatten()
                .any(|entry| entry.key == preparation_key));
            assert!(!store
                .entries
                .iter()
                .flatten()
                .any(|entry| entry.key == image_key));
        }
    }

    fn prepare(store: &mut Store, width: u32, limit: u64) {
        let source = ImageDimensions::new(31, 9).unwrap();
        let output = Output {
            width,
            height: 5,
            resize: ResizePolicy::Bilinear {
                anchor: Anchor::Center,
            },
        };
        Call::new(
            store,
            None,
            Some(ResizePreparation { source, output }),
            [31 * 9 * 4, width as usize * 5 * 4, 0, 0],
            0,
            size_of::<Store>() as u64 + Call::record_bytes(),
            limit,
            &mut 0,
            &mut SystemAllocator,
        )
        .unwrap()
        .finish(Ok(()))
        .unwrap();
    }

    #[test]
    fn total_pressure_drops_all_idle_scratch_before_an_lru_entry() {
        let mut store = Store::default();
        let limit = 1 << 20;
        prepare(&mut store, 17, limit);
        prepare(&mut store, 19, limit);
        let (entries, _, _, retained, scratch) = store.stats();
        assert_eq!(entries, 2);
        assert!(scratch > 0);
        let heap = store.capacity();
        store.room(limit - heap + 1, limit).unwrap();
        assert_eq!(store.stats().0, entries);
        assert_eq!(store.stats().3, retained);
        assert_eq!(store.stats().4, 0);
        let heap = store.capacity();
        store.room(limit - heap + 1, limit).unwrap();
        assert_eq!(store.stats().0, 1);
    }

    #[test]
    fn retained_byte_limit_uses_capacity_and_oversized_plans_finish_uncached() {
        let mut store = Store::default();
        let limit = 32 * 1024;
        for width in 2..40 {
            prepare(&mut store, width, limit);
            assert!(store.stats().3 <= limit / 4);
            assert!(store.stats().0 < MAX_CACHE_ENTRIES);
        }
        assert!(store.stats().0 < 38);
        // A large coordinate map can fit the whole call while exceeding its quarter-size cache cap.
        let mut store = Store::default();
        let source = ImageDimensions::new(1, 1).unwrap();
        let output = Output {
            width: 4096,
            height: 1,
            resize: ResizePolicy::Nearest {
                anchor: Anchor::Center,
            },
        };
        let limit = 80 * 1024;
        Call::new(
            &mut store,
            None,
            Some(ResizePreparation { source, output }),
            [4, 16384, 0, 0],
            0,
            size_of::<Store>() as u64 + Call::record_bytes(),
            limit,
            &mut 0,
            &mut SystemAllocator,
        )
        .unwrap()
        .finish(Ok(()))
        .unwrap();
        assert_eq!(store.stats().0, 0);
        assert!(store.stats().4 > 0);
    }
}
