//! Fallback and parity tests. GPU tests accept software adapters and skip when there is none.

use std::sync::{Mutex, MutexGuard, PoisonError};

use pollster::block_on;

use super::*;

/// Mesa's Vulkan loader crashes when tests open instances on several threads at once.
static SERIAL: Mutex<()> = Mutex::new(());

/// The adapter-backed backend, or `None` on a host without one. Hold the guard for the whole test.
fn any_gpu() -> Option<(MutexGuard<'static, ()>, Backend)> {
    let serial = SERIAL.lock().unwrap_or_else(PoisonError::into_inner);
    match block_on(Backend::detect_on(&Gpu::instance(), Adapters::Any)) {
        Backend::Cpu(Fallback::NoAdapter) => {
            eprintln!("skipped: no adapter");
            None
        }
        backend => Some((serial, backend)),
    }
}

fn cpu_result(stage: &impl Stage, rgba: &[u8]) -> Vec<u8> {
    let mut expected = rgba.to_vec();
    stage.cpu(&mut expected);
    expected
}

/// Adds one to red on the GPU only.
struct Skewed;

impl Stage for Skewed {
    const WGSL: &'static str = r"
@group(0) @binding(2) var<storage, read> unused: array<u32>;

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    if !inside(id.xy) {
        return;
    }
    store(id.xy, load(id.xy) + vec4<u32>(unused[0], 0u, 0u, 0u));
}
";

    fn params(&self) -> Vec<u8> {
        1u32.to_le_bytes().to_vec()
    }

    fn cpu(&self, _rgba: &mut [u8]) {}
}

#[test]
fn no_adapter_runs_the_cpu_path() {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::empty(),
        ..wgpu::InstanceDescriptor::new_without_display_handle()
    });
    let mut backend = block_on(Backend::detect_on(&instance, Adapters::Any));
    assert!(matches!(backend, Backend::Cpu(Fallback::NoAdapter)));

    let (stage, dimensions, mut rgba) = ChannelLookup::known_answer();
    let expected = cpu_result(&stage, &rgba);
    let ran = block_on(backend.run(&stage, dimensions, &mut rgba));
    assert_eq!(ran, Some(Fallback::NoAdapter));
    assert_eq!(rgba, expected);
}

fn effects(json: &str) -> Vec<crate::prod::effects::EffectStep> {
    crate::prod::effects::decode_effects(json).unwrap()
}

fn real_chain() -> Vec<crate::prod::effects::EffectStep> {
    effects(
        r#"[
        {"effect":"brightness-contrast","enabled":true,"brightness":-0.11,"contrast":0.32},
        {"effect":"levels","enabled":true,"channel":"red","input":{"black":0.1,"white":0.9},"gamma":1.7,"output":{"black":0.8,"white":0.2}},
        {"effect":"exposure","enabled":true,"stops":0.6},
        {"effect":"white-balance","enabled":true,"temperature":-0.2,"tint":0.4}
    ]"#,
    )
}

#[test]
fn lookup_selects_only_the_whole_enabled_unmasked_channel_chain() {
    let chain = real_chain();
    let stage = ChannelLookup::from_effects(&chain).unwrap().unwrap();
    assert_eq!(
        stage.0,
        crate::prod::effects::table::ChannelTables::new(chain.iter().map(|step| &step.effect))
            .bytes()
    );

    let mixed =
        r#"{"effect":"hue-saturation","enabled":true,"hue":30,"saturation":0.2,"lightness":0}"#;
    let mut chain = real_chain();
    chain.extend(effects(&format!("[{mixed}]")));
    assert!(ChannelLookup::from_effects(&chain).unwrap().is_none());
    chain.last_mut().unwrap().enabled = false;
    assert!(ChannelLookup::from_effects(&chain).unwrap().is_some());
    chain[0].mask =
        serde_json::from_str(r#"[{"x":{"model":"srgb","channel":"red"},"points":[[0,0],[1,1]]}]"#)
            .unwrap();
    assert!(ChannelLookup::from_effects(&chain).unwrap().is_none());
    chain[0].enabled = false;
    assert!(ChannelLookup::from_effects(&chain).unwrap().is_some());
}

#[test]
fn lookup_memory_includes_padded_upload_and_all_owned_pixel_buffers() {
    let dimensions = ImageDimensions::new(37, 19).unwrap();
    let plan = LookupMemory::new(dimensions);
    assert_eq!(plan.rgba_bytes, 37 * 19 * 4);
    assert_eq!(plan.aligned_texture_bytes, 256 * 19);
    assert_eq!(
        plan.required_bytes,
        LookupMemory::FIXED_BYTES + 2 * 256 * 19 + 6 * 37 * 19 * 4
    );
    assert_eq!(
        plan.check(plan.required_bytes - 1),
        Err(Fallback::MemoryLimit)
    );
    assert_eq!(plan.check(plan.required_bytes), Ok(()));
    let huge = LookupMemory::new(ImageDimensions::new(32768, 32768).unwrap());
    assert!(huge.required_bytes > u64::from(u32::MAX));
}

#[test]
fn real_effects_lookup_is_exact_and_keeps_owned_outputs_and_alpha() {
    let Some((_serial, mut backend)) = any_gpu() else {
        return;
    };
    let Backend::Gpu(gpu) = &backend else {
        panic!("hardware rejected");
    };
    eprintln!("real effects adapter: {:?}", gpu.info());
    let chain = real_chain();
    let stage = ChannelLookup::from_effects(&chain).unwrap().unwrap();
    let (_, dimensions, input) = ChannelLookup::known_answer();
    let expected = crate::prod::effects::apply_effects(crate::prod::effects::EffectsRequest {
        version: 1,
        source: crate::prod::contract::request::Source {
            width: dimensions.width(),
            height: dimensions.height(),
            data: &input,
        },
        effects: &chain,
        context: Default::default(),
    })
    .unwrap();
    let limit = LookupMemory::new(dimensions).required_bytes;
    assert_eq!(
        block_on(backend.lookup(&stage, dimensions, &input, limit - 1)),
        Err(Fallback::MemoryLimit)
    );
    assert!(matches!(backend, Backend::Gpu(_)));
    let output = block_on(backend.lookup(&stage, dimensions, &input, limit)).unwrap();
    assert_eq!(output, expected.data());
    assert!(output
        .chunks_exact(4)
        .zip(input.chunks_exact(4))
        .all(|(out, source)| out[3] == source[3]));
    let later = block_on(backend.lookup(&stage, dimensions, &vec![0; input.len()], limit)).unwrap();
    assert_ne!(output, later);
    drop(backend);
    assert_eq!(output, expected.data());
}

#[test]
fn real_lookup_device_loss_returns_fallback_without_partial_output() {
    let Some((_serial, Backend::Gpu(gpu))) = any_gpu() else {
        return;
    };
    gpu.device.destroy();
    let mut backend = Backend::Gpu(gpu);
    let (stage, dimensions, input) = ChannelLookup::known_answer();
    assert!(block_on(backend.lookup(&stage, dimensions, &input, u64::MAX)).is_err());
    assert!(matches!(backend, Backend::Cpu(_)));
    assert_eq!(input, ChannelLookup::known_answer().2);
}

#[test]
fn parity_reports_largest_difference_and_differing_pixels() {
    let cpu = [0, 0, 0, 255, 10, 20, 30, 40, 7, 7, 7, 7];
    let gpu = [0, 0, 0, 255, 13, 20, 29, 40, 7, 7, 7, 8];
    let parity = Parity::between(&cpu, &gpu);
    assert_eq!(
        parity,
        Parity {
            max_byte_difference: 3,
            differing_pixels: 2,
            pixels: 3
        }
    );
    assert!(!parity.exact());
}

#[test]
fn known_answer_round_trips_exactly() {
    let Some((_serial, mut backend)) = any_gpu() else {
        return;
    };
    match &backend {
        Backend::Gpu(gpu) => eprintln!("adapter: {:?}", gpu.info()),
        Backend::Cpu(reason) => panic!("adapter found but GPU rejected: {reason:?}"),
    }
    let (stage, dimensions, rgba) = ChannelLookup::known_answer();
    let mut ran = rgba.clone();
    assert_eq!(block_on(backend.run(&stage, dimensions, &mut ran)), None);
    assert_eq!(ran, cpu_result(&stage, &rgba));
}

#[test]
fn parity_catches_a_kernel_that_disagrees() {
    let Some((_serial, Backend::Gpu(gpu))) = any_gpu() else {
        return;
    };
    let (_, dimensions, rgba) = ChannelLookup::known_answer();
    let parity = block_on(gpu.parity(&Skewed, dimensions, &rgba)).expect("kernel runs");
    assert_eq!(parity.max_byte_difference, 1);
    // Red 255 saturates, so exactly one pixel of the known input still agrees.
    assert_eq!(parity.differing_pixels, parity.pixels - 1);
}

#[test]
fn lost_device_falls_back_and_stays_on_the_cpu() {
    let Some((_serial, Backend::Gpu(gpu))) = any_gpu() else {
        return;
    };
    gpu.device.destroy();
    let mut backend = Backend::Gpu(gpu);

    let (stage, dimensions, mut rgba) = ChannelLookup::known_answer();
    let expected = cpu_result(&stage, &rgba);
    let ran = block_on(backend.run(&stage, dimensions, &mut rgba));
    assert!(
        matches!(ran, Some(Fallback::Lost(_) | Fallback::Failed(_))),
        "{ran:?}"
    );
    assert_eq!(rgba, expected);
    assert!(matches!(backend, Backend::Cpu(_)));
}

/// The known-answer lookup, with a hook that fires when `run` reads its parameters.
struct Sabotaged<F> {
    lookup: ChannelLookup,
    hook: F,
}

impl<F: Fn() + 'static> Stage for Sabotaged<F> {
    const WGSL: &'static str = ChannelLookup::WGSL;

    fn params(&self) -> Vec<u8> {
        (self.hook)();
        self.lookup.params()
    }

    fn cpu(&self, rgba: &mut [u8]) {
        self.lookup.cpu(rgba);
    }
}

/// Runs the sabotaged lookup through `Backend::run`: the CPU must redo it and take over.
fn assert_falls_back(make_hook: impl FnOnce(&Gpu) -> Box<dyn Fn()>) {
    let Some((_serial, Backend::Gpu(gpu))) = any_gpu() else {
        return;
    };
    let (lookup, dimensions, mut rgba) = ChannelLookup::known_answer();
    let stage = Sabotaged {
        hook: make_hook(&gpu),
        lookup,
    };
    let expected = cpu_result(&stage.lookup, &rgba);
    let mut backend = Backend::Gpu(gpu);
    let ran = block_on(backend.run(&stage, dimensions, &mut rgba));
    assert!(
        matches!(ran, Some(Fallback::Lost(_) | Fallback::Failed(_))),
        "{ran:?}"
    );
    assert_eq!(rgba, expected);
    assert!(matches!(backend, Backend::Cpu(_)));
}

#[test]
fn device_lost_during_setup_falls_back() {
    assert_falls_back(|gpu| {
        let device = gpu.device.clone();
        Box::new(move || device.destroy())
    });
}

#[test]
fn device_lost_in_flight_falls_back() {
    assert_falls_back(|gpu| {
        let (device, queue) = (gpu.device.clone(), gpu.queue.clone());
        Box::new(move || {
            let device = device.clone();
            queue.on_submitted_work_done(move || device.destroy());
        })
    });
}

/// llvmpipe never reports a lost device from its fence wait, so this feeds the
/// guard the panic wgpu raises from `Device::poll` when one does.
#[test]
fn a_wgpu_panic_becomes_a_lost_device() {
    let message = "Error in Device::poll: Parent device is lost";
    let caught = device::unwound(|| -> u8 { panic!("{message}") });
    assert_eq!(caught, Err(Fallback::Lost(message.into())));
    assert_eq!(device::unwound(|| 7), Ok(7));
}

#[test]
fn oversized_image_runs_on_the_cpu_and_keeps_the_gpu() {
    let Some((_serial, Backend::Gpu(gpu))) = any_gpu() else {
        return;
    };
    let width = gpu.device.limits().max_texture_dimension_2d + 1;
    let mut backend = Backend::Gpu(gpu);

    let (stage, _, _) = ChannelLookup::known_answer();
    let dimensions = ImageDimensions::new(width, 1).expect("valid dimensions");
    let mut rgba: Vec<u8> = (0..width * 4).map(|i| i as u8).collect();
    let expected = cpu_result(&stage, &rgba);
    let ran = block_on(backend.run(&stage, dimensions, &mut rgba));
    assert_eq!(ran, Some(Fallback::TooLarge));
    assert_eq!(rgba, expected);
    assert!(matches!(backend, Backend::Gpu(_)));
}
