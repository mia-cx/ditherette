//! One instance's preparation LRU and success-only scratch publication.

use std::mem::size_of;

use super::{
    identity,
    processor::Allocator,
    quantize::{preparation_failure, QuantizeRequest},
    resize::PreparedResize,
};
use crate::{
    image::ImageDimensions,
    prod::{
        contract::{
            cache::Identity,
            error::ErrorCode,
            failure::{ErrorPath, Failure},
            lifecycle::{MAX_CACHE_BYTES, MAX_CACHE_ENTRIES},
            request::Output,
        },
        quantize::PreparedQuantizer,
        resize::common::allocation::CapacityBudget,
    },
};

enum Value {
    Palette(Vec<PreparedQuantizer>),
    Resize(Vec<PreparedResize>),
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
}

impl Default for Store {
    fn default() -> Self {
        Self {
            entries: std::array::from_fn(|_| None),
            scratch: Scratch::default(),
            clock: 0,
            hits: 0,
            misses: 0,
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
        self.clock += 1;
        entry.used = self.clock;
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
    pub scratch: Scratch,
    success: bool,
    limit: u64,
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
        let palette_entry = palette_key.and_then(|key| store.take(key));
        let resize_entry = resize_key.and_then(|key| store.take(key));
        let mut call = Self {
            palette_hit: palette_entry.is_some(),
            resize_hit: resize_entry.is_some(),
            palette: palette_entry,
            resize: resize_entry,
            scratch: std::mem::take(&mut store.scratch),
            store,
            success: false,
            limit,
        };
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
        if planned(&call)? + call.store.capacity() > limit {
            call.scratch = Scratch::default();
            if let Some(entry) = &mut call.resize {
                entry.drop_scratch();
            }
        }
        call.store.room(planned(&call)?, limit)?;
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
            buffer.clear();
            if buffer.capacity() < length {
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
            call.scratch.diffusion = CapacityBudget::new(limit - actual).vector(diffusion_len)?;
        }
        actual += (call.scratch.diffusion.capacity() * size_of::<[f32; 3]>()) as u64;
        *peak = (*peak).max(actual);
        if actual > limit {
            return Err(memory_limit());
        }
        call.scratch.diffusion.resize(diffusion_len, [0.0; 3]);
        Ok(call)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prod::contract::request::{Anchor, ResizePolicy};
    use crate::prod::pipeline::processor::SystemAllocator;

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
