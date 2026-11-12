//! Writes the PiPL resource After Effects reads to register the effect.
//!
//! `pipl` formats the resource for the OS running the build, so plugins are
//! built natively on macOS and Windows. Other hosts only build the render core.

use pipl::*;

/// The effect API version these bindings target (After Effects 2023+).
const PF_PLUG_IN_VERSION: u16 = 13;
const PF_PLUG_IN_SUBVERS: u16 = 28;

#[rustfmt::skip]
fn main() {
    let target = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    if target != "macos" && target != "windows" {
        return;
    }
    // `define_effect!` reads these; pipl sets some without declaring them.
    println!("cargo::rustc-check-cfg=cfg(does_dialog, uses_audio, sends_update_params_ui, gpu_render, smart_render, threaded_rendering, with_premiere, catch_panics)");
    // A panic must reach After Effects as an error, never unwind into it.
    println!("cargo::rustc-cfg=catch_panics");
    let version: Vec<u32> = env!("CARGO_PKG_VERSION").split('.').map(|part| part.parse().unwrap()).collect();
    pipl::plugin_build(vec![
        Property::Kind(PIPLType::AEEffect),
        Property::Name("Ditherette"),
        Property::Category("Ditherette"),

        #[cfg(target_os = "windows")]
        Property::CodeWin64X86("EffectMain"),
        #[cfg(target_os = "macos")]
        Property::CodeMacIntel64("EffectMain"),
        #[cfg(target_os = "macos")]
        Property::CodeMacARM64("EffectMain"),

        Property::AE_PiPL_Version { major: 2, minor: 0 },
        Property::AE_Effect_Spec_Version { major: PF_PLUG_IN_VERSION, minor: PF_PLUG_IN_SUBVERS },
        Property::AE_Effect_Version {
            version: version[0],
            subversion: version[1],
            bugversion: version[2],
            stage: Stage::Release,
            build: 0,
        },
        Property::AE_Effect_Info_Flags(0),
        // Not pixel-independent: resize and dithering read neighbouring pixels.
        Property::AE_Effect_Global_OutFlags(OutFlags::empty()),
        Property::AE_Effect_Global_OutFlags_2(
            OutFlags2::SupportsThreadedRendering |
            OutFlags2::SupportsGetFlattenedSequenceData
        ),
        // Saved projects find the effect by this name. Never change it.
        Property::AE_Effect_Match_Name("mia.cx Ditherette"),
        Property::AE_Reserved_Info(0),
        Property::AE_Effect_Support_URL("https://ditherette.mia.cx"),
    ])
}
