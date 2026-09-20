use ditherette_wasm::prod::contract::{
    cache::*,
    error::ErrorCode,
    lifecycle::{InitOptions, InstanceModel, Progress, Stage},
};

#[test]
fn preparation_identities_match_frozen_palette_and_resize_keys() {
    use ditherette_wasm::{image::contracts::PaletteEntry, spec::contract::cache as frozen};
    use serde_json::json;
    let color = PaletteEntry::Color { rgb: [1, 2, 3] };
    let transparent = PaletteEntry::Transparent {};
    let palettes = [
        vec![color, transparent],
        vec![transparent, color],
        vec![color, color, transparent],
        vec![color; 256],
        vec![color; 257],
    ];
    for entries in &palettes {
        let palette = palette_identity(entries);
        let expected_palette = frozen::palette_identity(entries);
        assert_eq!(palette.0, expected_palette.0);
        for matching in [
            "srgb-euclidean",
            "srgb-compuphase",
            "srgb-rec601",
            "srgb-rec709",
            "linear-rgb-euclidean",
            "oklab-euclidean",
            "oklch-euclidean",
            "oklch-circular-hue",
            "oklch-hue-arc",
            "cielab-euclidean",
            "cielab-ciede2000",
            "cielch-euclidean",
            "cielch-circular-hue",
            "cielch-hue-arc",
            "ycbcr-euclidean",
        ] {
            for alpha in [
                json!({"mode": "preserve", "threshold": -0.0}),
                json!({"mode": "preserve", "threshold": 0.0}),
                json!({"mode": "preserve", "threshold": 127.9999999}),
                json!({"mode": "preserve", "threshold": 128.0}),
                json!({"mode": "premultiplied"}),
                json!({"mode": "matte", "rgb": [3, 19, 47]}),
            ] {
                assert_eq!(
                    stage_identity(
                        None,
                        1,
                        StageOptions::PreparedPalette {
                            palette,
                            alpha: serde_json::from_value(alpha.clone()).unwrap(),
                            matching: serde_json::from_value(json!(matching)).unwrap(),
                        }
                    )
                    .0,
                    frozen::stage_identity(
                        None,
                        1,
                        frozen::StageOptions::PreparedPalette {
                            palette: expected_palette,
                            alpha: serde_json::from_value(alpha).unwrap(),
                            matching: serde_json::from_value(json!(matching)).unwrap(),
                        }
                    )
                    .0,
                );
            }
        }
    }
    assert_ne!(
        palette_identity(&palettes[0]),
        palette_identity(&palettes[1])
    );
    assert_ne!(
        palette_identity(&palettes[0]),
        palette_identity(&palettes[2])
    );
    assert_ne!(
        palette_identity(&palettes[3]),
        palette_identity(&palettes[4])
    );
    let mut tail = palettes[4].clone();
    tail[256] = transparent;
    tail.push(transparent);
    assert_eq!(palette_identity(&tail), palette_identity(&palettes[4]));

    for algorithm in [
        "nearest",
        "area",
        "bilinear",
        "bicubic",
        "lanczos2",
        "lanczos3",
        "trilinear",
    ] {
        for anchor in [
            "top-left",
            "top",
            "top-right",
            "left",
            "center",
            "right",
            "bottom-left",
            "bottom",
            "bottom-right",
        ] {
            for support in ["fixed", "scale-aware"] {
                let mut resize = json!({"algorithm": algorithm});
                if algorithm != "area" {
                    resize["anchor"] = json!(anchor);
                }
                if ["bicubic", "lanczos2", "lanczos3"].contains(&algorithm) {
                    resize["support"] = json!(support);
                }
                let output = json!({"width": 65, "height": 33, "resize": resize});
                assert_eq!(
                    stage_identity(
                        None,
                        1,
                        StageOptions::ResizePlan {
                            source_width: 129,
                            source_height: 97,
                            output: serde_json::from_value(output.clone()).unwrap(),
                        }
                    )
                    .0,
                    frozen::stage_identity(
                        None,
                        1,
                        frozen::StageOptions::ResizePlan {
                            source_width: 129,
                            source_height: 97,
                            output: serde_json::from_value(output).unwrap(),
                        }
                    )
                    .0,
                );
            }
        }
    }
}

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
