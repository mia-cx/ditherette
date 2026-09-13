use ditherette_wasm::{
    image::{contracts::PaletteEntry, ImageBuf, Rgba8},
    spec::{
        contract::{
            cache::*,
            error::ErrorCode,
            lifecycle::{InitOptions, InstanceModel, Progress, Stage},
            request::*,
        },
        palette::PreparedPalette,
    },
};

fn cache(limit: u64) -> CacheModel {
    CacheModel::new(InitOptions {
        memory_limit_bytes: limit,
        ..InitOptions::default()
    })
    .unwrap()
}

fn key(byte: u8) -> Identity {
    Identity([byte; 32])
}

fn value(byte: u8) -> CachedValue {
    CachedValue {
        bytes: vec![byte],
        rgba_content: None,
    }
}

fn publish(
    cache: &mut CacheModel,
    instance: &mut InstanceModel,
    identity: Identity,
    capacity: u64,
    scratch: u64,
) {
    instance.begin(false).unwrap();
    cache
        .begin(
            MemoryPlan {
                work: capacity,
                scratch,
                boundary_copies: 0,
            },
            false,
        )
        .unwrap();
    assert!(cache
        .stage(identity, value(identity.0[0]), capacity)
        .unwrap());
    instance.output_ready().unwrap();
    cache.finish(instance).unwrap();
}

fn indexed(palette: Identity, alpha: AlphaPolicy, dither: DitherPolicy) -> StageOptions {
    StageOptions::Indexed {
        palette,
        alpha,
        matching: MatchPolicy::SrgbEuclidean,
        dither,
    }
}

fn image_source(image: &ImageBuf<Rgba8>) -> Source<'_> {
    Source {
        width: image.dimensions().width(),
        height: image.dimensions().height(),
        data: image.data(),
    }
}

#[test]
fn all_five_request_plans_share_keys_at_actual_naive_rgba_boundaries() {
    let bytes = [10, 40, 60, 128, 200, 140, 90, 255];
    let source = Source {
        width: 2,
        height: 1,
        data: &bytes,
    };
    let output = Output {
        width: 1,
        height: 2,
        resize: ResizePolicy::Area {},
    };
    let palette = [
        PaletteEntry::Color { rgb: [0, 0, 0] },
        PaletteEntry::Color {
            rgb: [255, 255, 255],
        },
        PaletteEntry::Transparent {},
    ];
    let resize_request = ResizeRequest {
        version: 1,
        source,
        output,
    };
    let standalone_resize = request_identity_plan(Request::Resize(resize_request))
        .unwrap()
        .resolve(&[])
        .unwrap();
    let resized = ditherette_wasm::spec::resize::resize(resize_request).unwrap();
    let resized_source = image_source(&resized);
    let resized_content = source_identity(resized_source);
    assert_ne!(standalone_resize.final_identity, resized_content);
    let perturb = PerturbPolicy {
        field: Field::Bayer {
            size: BayerSize::Two,
        },
        space: WorkingSpace::Srgb,
        strength: 1.0,
        placement: Placement::Everywhere {},
    };
    for dither in [
        DitherPolicy::None {},
        DitherPolicy::Separable { perturb },
        DitherPolicy::Diffusion {
            kernel: Diffusion::FloydSteinberg,
            strength: 1.0,
            placement: Placement::Everywhere {},
            serpentine: false,
            feedback: DiffusionFeedback::SrgbBytes,
        },
        DitherPolicy::Yliluoma {
            size: BayerSize::Two,
            placement: Placement::Everywhere {},
        },
    ] {
        let alpha = AlphaPolicy::Preserve {
            threshold: 127.9999999,
        };
        let process = request_identity_plan(Request::Process(ProcessRequest {
            source,
            palette: &palette,
            recipe: RecipeV1 {
                version: 1,
                output,
                alpha,
                matching: MatchPolicy::SrgbEuclidean,
                dither,
            },
        }))
        .unwrap();
        let quantize = QuantizeRequest {
            version: 1,
            source: resized_source,
            palette: &palette,
            alpha,
            matching: MatchPolicy::SrgbEuclidean,
        };
        let fused = request_identity_plan(Request::DitherAndQuantize(DitherQuantizeRequest {
            quantize,
            dither,
        }))
        .unwrap();
        assert_eq!(
            process.key_at(0, &[]).unwrap(),
            standalone_resize.final_identity
        );
        assert!(process.resolve(&[]).is_err());
        let mut process_outputs = vec![(0, resized_content)];
        let mut fused_outputs = Vec::new();
        if let DitherPolicy::Separable { perturb } = dither {
            let standalone = request_identity_plan(Request::Perturb(PerturbRequest {
                version: 1,
                source: resized_source,
                perturb,
            }))
            .unwrap()
            .resolve(&[])
            .unwrap();
            let perturbed =
                ditherette_wasm::spec::dither::perturb::perturb(resized.as_view(), perturb)
                    .unwrap();
            let perturbed_source = image_source(&perturbed);
            let perturbed_content = source_identity(perturbed_source);
            // Resize is process operation 0; raw Color/Perturb are its following operations.
            process_outputs.push((2, perturbed_content));
            fused_outputs.push((1, perturbed_content));
            assert_eq!(fused.key_at(1, &[]).unwrap(), standalone.final_identity);
            assert_eq!(fused.key_at(0, &[]).unwrap(), standalone.operations[0]);
            let direct_quantize = request_identity_plan(Request::Quantize(QuantizeRequest {
                source: perturbed_source,
                ..quantize
            }))
            .unwrap()
            .resolve(&[])
            .unwrap();
            let resolved_fused = fused.resolve(&fused_outputs).unwrap();
            assert_eq!(resolved_fused.operations[2..], direct_quantize.operations);
            assert_eq!(
                resolved_fused.final_identity,
                direct_quantize.final_identity
            );
        }
        let resolved_process = process.resolve(&process_outputs).unwrap();
        let resolved_fused = fused.resolve(&fused_outputs).unwrap();
        assert_eq!(resolved_process.operations[1..], resolved_fused.operations);
        assert_eq!(
            resolved_process.preparation[1..],
            resolved_fused.preparation
        );
        assert_eq!(
            resolved_process.final_identity,
            resolved_fused.final_identity
        );
        if matches!(dither, DitherPolicy::None {}) {
            let direct = request_identity_plan(Request::Quantize(quantize))
                .unwrap()
                .resolve(&[])
                .unwrap();
            assert_eq!(resolved_fused, direct);
        }
    }
}

#[test]
fn alpha_stage_separates_raw_color_and_different_alpha_preparation() {
    let bytes = [200, 40, 80, 128];
    let source = Source {
        width: 1,
        height: 1,
        data: &bytes,
    };
    let palette = [PaletteEntry::Color { rgb: [0, 0, 0] }];
    let request = QuantizeRequest {
        version: 1,
        source,
        palette: &palette,
        alpha: AlphaPolicy::Preserve {
            threshold: 127.9999999,
        },
        matching: MatchPolicy::SrgbEuclidean,
    };
    let preserve = request_identity_plan(Request::Quantize(request))
        .unwrap()
        .resolve(&[])
        .unwrap();
    let premultiplied = request_identity_plan(Request::Quantize(QuantizeRequest {
        alpha: AlphaPolicy::Premultiplied {},
        ..request
    }))
    .unwrap()
    .resolve(&[])
    .unwrap();
    let raw = stage_identity(
        Some(source_identity(source)),
        1,
        StageOptions::Color {
            space: WorkingSpace::Srgb,
        },
    );
    assert_ne!(preserve.operations[0], premultiplied.operations[0]);
    assert_ne!(preserve.operations[1], premultiplied.operations[1]);
    assert_ne!(raw, preserve.operations[1]);
    assert_ne!(raw, premultiplied.operations[1]);
    assert_eq!(
        request_identity_plan(Request::Quantize(QuantizeRequest {
            version: 2,
            ..request
        }))
        .unwrap_err()
        .code,
        ErrorCode::InvalidRequest
    );
}

#[test]
fn rgba_cache_hits_reuse_recorded_content_identity_for_downstream_keys() {
    let bytes = [10, 40, 60, 128, 200, 140, 90, 255];
    let source = Source {
        width: 2,
        height: 1,
        data: &bytes,
    };
    let output = Output {
        width: 1,
        height: 1,
        resize: ResizePolicy::Area {},
    };
    let palette = [PaletteEntry::Color { rgb: [0, 0, 0] }];
    let plan = request_identity_plan(Request::Process(ProcessRequest {
        source,
        palette: &palette,
        recipe: RecipeV1 {
            version: 1,
            output,
            alpha: AlphaPolicy::Premultiplied {},
            matching: MatchPolicy::SrgbEuclidean,
            dither: DitherPolicy::None {},
        },
    }))
    .unwrap();
    let resized = ditherette_wasm::spec::resize::resize(ResizeRequest {
        version: 1,
        source,
        output,
    })
    .unwrap();
    let content = source_identity(image_source(&resized));
    let cold = plan.resolve(&[(0, content)]).unwrap();
    let operation = plan.key_at(0, &[]).unwrap();
    let mut cache = cache(512);
    let mut instance = InstanceModel::default();
    instance.begin(false).unwrap();
    cache
        .begin(
            MemoryPlan {
                work: 64,
                ..MemoryPlan::default()
            },
            false,
        )
        .unwrap();
    cache
        .stage(
            operation,
            CachedValue {
                bytes: resized.data().to_vec(),
                rgba_content: Some(content),
            },
            64,
        )
        .unwrap();
    instance.output_ready().unwrap();
    cache.finish(&mut instance).unwrap();
    let hit = cache.lookup(operation).unwrap().unwrap();
    assert_eq!(hit.bytes, resized.data());
    assert_eq!(
        plan.resolve(&[(0, hit.rgba_content.unwrap())]).unwrap(),
        cold
    );
}

#[test]
fn source_identity_matches_independent_sha256_and_reads_every_current_byte() {
    let mut bytes = [1, 2, 3, 4];
    let identity = source_identity(Source {
        width: 1,
        height: 1,
        data: &bytes,
    });
    // Independently computed with Node crypto's SHA-256, including prefix and LE dimensions.
    assert_eq!(
        identity
            .0
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>(),
        "9f7bdf6cb06b4e1bcc24fa6ac1a6a74a26afa6038a84c8c9a6137b2f5ff28b78"
    );
    for channel in 0..4 {
        bytes[channel] ^= 1;
        assert_ne!(
            identity,
            source_identity(Source {
                width: 1,
                height: 1,
                data: &bytes
            })
        );
        bytes[channel] ^= 1;
    }
    let bytes = [0; 8];
    assert_ne!(
        source_identity(Source {
            width: 1,
            height: 2,
            data: &bytes
        }),
        source_identity(Source {
            width: 2,
            height: 1,
            data: &bytes
        })
    );
}

#[test]
fn palette_identity_preserves_order_duplicates_and_truncation_warnings() {
    let a = PaletteEntry::Color { rgb: [1, 2, 3] };
    let b = PaletteEntry::Transparent {};
    assert_ne!(palette_identity(&[a, b]), palette_identity(&[b, a]));
    assert_ne!(palette_identity(&[a, b]), palette_identity(&[a, a, b]));
    let short = [a; 256];
    let mut long = vec![a; 257];
    assert_ne!(palette_identity(&short), palette_identity(&long));
    let alpha = AlphaPolicy::Premultiplied {};
    let prepared_short = PreparedPalette::new(&short, alpha);
    let prepared_long = PreparedPalette::new(&long, alpha);
    assert_eq!(prepared_short.palette, prepared_long.palette);
    assert_ne!(prepared_short.warnings, prepared_long.warnings);
    let truncated = palette_identity(&long);
    long[256] = b;
    long.push(b);
    assert_eq!(truncated, palette_identity(&long));
}

#[test]
fn stage_keys_normalize_zero_but_preserve_threshold_precision_feedback_and_parent() {
    let palette = palette_identity(&[PaletteEntry::Transparent {}]);
    let zero = indexed(
        palette,
        AlphaPolicy::Preserve { threshold: 0.0 },
        DitherPolicy::None {},
    );
    let negative_zero = indexed(
        palette,
        AlphaPolicy::Preserve { threshold: -0.0 },
        DitherPolicy::None {},
    );
    assert_eq!(
        stage_identity(Some(key(1)), 1, zero),
        stage_identity(Some(key(1)), 1, negative_zero)
    );
    assert_ne!(
        stage_identity(Some(key(1)), 1, zero),
        stage_identity(Some(key(2)), 1, zero)
    );
    assert_ne!(
        stage_identity(Some(key(1)), 1, zero),
        stage_identity(Some(key(1)), 2, zero)
    );
    let below = indexed(
        palette,
        AlphaPolicy::Preserve {
            threshold: 127.9999999,
        },
        DitherPolicy::None {},
    );
    let at = indexed(
        palette,
        AlphaPolicy::Preserve { threshold: 128.0 },
        DitherPolicy::None {},
    );
    assert_ne!(
        stage_identity(Some(key(1)), 1, below),
        stage_identity(Some(key(1)), 1, at)
    );
    let policy = |feedback| {
        indexed(
            palette,
            AlphaPolicy::Premultiplied {},
            DitherPolicy::Diffusion {
                kernel: Diffusion::FloydSteinberg,
                strength: 1.0,
                placement: Placement::Everywhere {},
                serpentine: false,
                feedback,
            },
        )
    };
    assert_ne!(
        stage_identity(Some(key(1)), 1, policy(DiffusionFeedback::SrgbBytes)),
        stage_identity(Some(key(1)), 1, policy(DiffusionFeedback::Matching))
    );
    let field = |strength| StageOptions::Perturb {
        perturb: PerturbPolicy {
            field: Field::Random { seed: 42 },
            space: WorkingSpace::Oklab,
            strength,
            placement: Placement::Adaptive {
                radius: 1,
                threshold: strength,
                softness: strength,
            },
        },
    };
    assert_eq!(
        stage_identity(Some(key(1)), 1, field(0.0)),
        stage_identity(Some(key(1)), 1, field(-0.0))
    );
}

#[test]
fn stage_kinds_and_source_independent_preparation_do_not_alias() {
    let output = Output {
        width: 2,
        height: 2,
        resize: ResizePolicy::Area {},
    };
    let resize = StageOptions::Resize { output };
    let resize_key = stage_identity(Some(key(7)), 1, resize);
    let plan = StageOptions::ResizePlan {
        source_width: 4,
        source_height: 4,
        output,
    };
    assert_ne!(stage_identity(None, 1, plan), resize_key);
    let prepared = StageOptions::PreparedPalette {
        palette: palette_identity(&[PaletteEntry::Transparent {}]),
        alpha: AlphaPolicy::Preserve {
            threshold: 127.9999999,
        },
        matching: MatchPolicy::OklchHueArc,
    };
    assert_ne!(
        stage_identity(None, 1, prepared),
        stage_identity(None, 1, plan)
    );
    assert_ne!(
        stage_identity(
            Some(key(7)),
            1,
            StageOptions::Color {
                space: WorkingSpace::Srgb
            }
        ),
        stage_identity(
            Some(key(7)),
            1,
            StageOptions::Color {
                space: WorkingSpace::LinearRgb
            }
        )
    );
}

#[test]
fn capacity_preflight_counts_capacity_boundary_copies_and_overflow_before_processing() {
    let mut cache = cache(100);
    assert_eq!(
        cache
            .begin(
                MemoryPlan {
                    work: 80,
                    scratch: 10,
                    boundary_copies: 11
                },
                false
            )
            .unwrap_err()
            .code,
        ErrorCode::MemoryLimit
    );
    assert_eq!(cache.capacity().total(), 0);
    assert_eq!(
        cache
            .begin(
                MemoryPlan {
                    work: u64::MAX,
                    scratch: 1,
                    boundary_copies: 0
                },
                false
            )
            .unwrap_err()
            .code,
        ErrorCode::MemoryLimit
    );
    cache
        .begin(
            MemoryPlan {
                work: 80,
                scratch: 10,
                boundary_copies: 10,
            },
            false,
        )
        .unwrap();
    assert_eq!(cache.capacity().total(), 100);
    // One byte of tiny model data represents twenty bytes of allocated stage capacity.
    assert!(cache.stage(key(1), value(1), 20).unwrap());
    assert_eq!(cache.capacity().work, 60);
    assert_eq!(cache.capacity().pending, 20);
    assert_eq!(cache.capacity().total(), 100);
    cache.fail().unwrap();
    assert_eq!(cache.capacity().total(), 0);
}

#[test]
fn pressure_drops_idle_scratch_before_oldest_entries() {
    let mut cache = cache(100);
    let mut instance = InstanceModel::default();
    publish(&mut cache, &mut instance, key(1), 10, 60);
    publish(&mut cache, &mut instance, key(2), 10, 0);
    assert_eq!(cache.capacity().total(), 80);
    cache
        .begin(
            MemoryPlan {
                work: 30,
                ..MemoryPlan::default()
            },
            false,
        )
        .unwrap();
    assert_eq!(cache.capacity().idle_scratch, 0);
    assert_eq!(cache.retained_identities(), [key(1), key(2)]);
    cache.fail().unwrap();
    cache
        .begin(
            MemoryPlan {
                work: 90,
                ..MemoryPlan::default()
            },
            false,
        )
        .unwrap();
    assert_eq!(cache.retained_identities(), [key(2)]);
    assert_eq!(cache.capacity().total(), 100);
    cache.fail().unwrap();
}

#[test]
fn scratch_reuse_moves_full_capacity_without_counting_it_twice() {
    let mut cache = cache(100);
    let mut instance = InstanceModel::default();
    publish(&mut cache, &mut instance, key(1), 10, 60);
    cache
        .begin(
            MemoryPlan {
                work: 10,
                scratch: 1,
                boundary_copies: 0,
            },
            true,
        )
        .unwrap();
    assert_eq!(cache.capacity().idle_scratch, 0);
    assert_eq!(cache.capacity().active_scratch, 60);
    assert_eq!(cache.capacity().total(), 80);
    cache.fail().unwrap();
    assert_eq!(cache.capacity().total(), 10);
    assert_eq!(
        cache
            .begin(
                MemoryPlan {
                    scratch: 1,
                    ..MemoryPlan::default()
                },
                true
            )
            .unwrap_err()
            .code,
        ErrorCode::Runtime
    );
}

#[test]
fn hits_update_lru_and_both_byte_and_entry_limits_evict() {
    let mut cache = cache(40); // Cache budget is ten bytes.
    let mut instance = InstanceModel::default();
    publish(&mut cache, &mut instance, key(1), 4, 0);
    publish(&mut cache, &mut instance, key(2), 4, 0);
    assert_eq!(cache.lookup(key(1)).unwrap(), Some(value(1)));
    publish(&mut cache, &mut instance, key(3), 4, 0);
    assert_eq!(cache.retained_identities(), [key(1), key(3)]);
    assert_eq!(cache.capacity().retained, 8);
    let mut entries = self::cache(2048);
    for index in 0..129 {
        publish(&mut entries, &mut instance, key(index), 1, 0);
    }
    assert_eq!(entries.retained_identities().len(), 128);
    assert_eq!(entries.lookup(key(0)).unwrap(), None);
    assert_eq!(entries.lookup(key(128)).unwrap(), Some(value(128)));
}

#[test]
fn oversized_result_succeeds_without_cache_publication() {
    let mut cache = cache(100);
    let mut instance = InstanceModel::default();
    instance.begin(false).unwrap();
    cache
        .begin(
            MemoryPlan {
                work: 26,
                ..MemoryPlan::default()
            },
            false,
        )
        .unwrap();
    let output = vec![7];
    assert!(!cache
        .stage(
            key(7),
            CachedValue {
                bytes: output.clone(),
                rgba_content: None
            },
            26
        )
        .unwrap());
    assert_eq!(cache.capacity().work, 26);
    assert_eq!(cache.capacity().pending, 0);
    instance.output_ready().unwrap();
    cache.finish(&mut instance).unwrap();
    assert_eq!(output, [7]);
    assert_eq!(cache.lookup(key(7)).unwrap(), None);
    assert_eq!(cache.capacity().total(), 0);
}

#[test]
fn completion_failure_publishes_neither_pending_entry_and_next_call_succeeds() {
    let mut cache = cache(100);
    let mut instance = InstanceModel::default();
    instance.begin(true).unwrap();
    cache
        .begin(
            MemoryPlan {
                work: 20,
                ..MemoryPlan::default()
            },
            false,
        )
        .unwrap();
    cache.stage(key(1), value(1), 5).unwrap();
    cache.stage(key(2), value(2), 7).unwrap();
    assert_eq!(cache.capacity().work, 8);
    assert_eq!(cache.capacity().pending, 12);
    assert_eq!(cache.lookup(key(1)).unwrap(), None);
    assert_eq!(cache.lookup(key(2)).unwrap(), None);
    assert!(cache.finish(&mut instance).is_err());
    instance.output_ready().unwrap();
    assert!(cache.finish(&mut instance).is_err());
    instance
        .report(
            Progress {
                stage: Stage::Complete,
                completed: Some(1),
                total: Some(1),
            },
            0,
        )
        .unwrap();
    assert_eq!(
        cache.begin(MemoryPlan::default(), false).unwrap_err().code,
        ErrorCode::ReentrantCall
    );
    assert_eq!(
        cache.dispose(&mut instance).unwrap_err().code,
        ErrorCode::ReentrantCall
    );
    assert!(cache.finish(&mut instance).is_err());
    assert_eq!(instance.callback_failed().code, ErrorCode::Callback);
    cache.fail().unwrap();
    assert_eq!(cache.lookup(key(1)).unwrap(), None);
    assert_eq!(cache.lookup(key(2)).unwrap(), None);
    assert_eq!(cache.capacity().total(), 0);
    publish(&mut cache, &mut instance, key(3), 4, 0);
    assert_eq!(cache.lookup(key(3)).unwrap(), Some(value(3)));
}

#[test]
fn successful_completion_publishes_only_materialized_entries() {
    let mut cache = cache(100);
    let mut instance = InstanceModel::default();
    instance.begin(true).unwrap();
    cache
        .begin(
            MemoryPlan {
                work: 10,
                ..MemoryPlan::default()
            },
            false,
        )
        .unwrap();
    assert!(cache.stage(key(1), value(1), 4).unwrap());
    assert!(!cache.stage(key(1), value(1), 4).unwrap());
    instance.output_ready().unwrap();
    instance
        .report(
            Progress {
                stage: Stage::Complete,
                completed: None,
                total: None,
            },
            0,
        )
        .unwrap();
    instance.callback_succeeded().unwrap();
    cache.finish(&mut instance).unwrap();
    assert_eq!(cache.retained_identities(), [key(1)]);
    assert_eq!(cache.capacity().retained, 4);
    assert_eq!(cache.capacity().work, 0);
}

#[test]
fn allocation_failure_discards_pending_entries_and_preserves_instance_usability() {
    let mut cache = cache(100);
    let mut instance = InstanceModel::default();
    publish(&mut cache, &mut instance, key(1), 4, 0);
    instance.begin(false).unwrap();
    cache
        .begin(
            MemoryPlan {
                work: 10,
                ..MemoryPlan::default()
            },
            false,
        )
        .unwrap();
    cache.stage(key(2), value(2), 4).unwrap();
    let error = cache.allocation_failed();
    assert_eq!(
        (error.code, error.path.as_str()),
        (ErrorCode::WasmMemoryUnavailable, "memory")
    );
    instance.fail().unwrap();
    assert_eq!(cache.lookup(key(1)).unwrap(), Some(value(1)));
    assert_eq!(cache.lookup(key(2)).unwrap(), None);
    publish(&mut cache, &mut instance, key(3), 4, 0);
}

#[test]
fn results_are_durable_instances_are_isolated_and_disposal_releases_ownership() {
    let mut first = cache(100);
    let mut second = cache(100);
    let mut first_control = InstanceModel::default();
    let mut second_control = InstanceModel::default();
    publish(&mut first, &mut first_control, key(1), 4, 20);
    let mut result = first.lookup(key(1)).unwrap().unwrap();
    result.bytes[0] = 99;
    assert_eq!(first.lookup(key(1)).unwrap(), Some(value(1)));
    assert_eq!(second.lookup(key(1)).unwrap(), None);
    publish(&mut second, &mut second_control, key(2), 4, 0);
    first.dispose(&mut first_control).unwrap();
    first.dispose(&mut first_control).unwrap();
    assert_eq!(first.capacity().total(), 0);
    assert_eq!(result.bytes, [99]);
    assert_eq!(
        first.begin(MemoryPlan::default(), false).unwrap_err().code,
        ErrorCode::Disposed
    );
    assert_eq!(
        first_control.begin(false).unwrap_err().code,
        ErrorCode::Disposed
    );
    assert_eq!(second.lookup(key(2)).unwrap(), Some(value(2)));
}
