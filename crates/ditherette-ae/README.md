# ditherette-ae

Ditherette as an After Effects effect. It renders each frame with the same Rust engine as the website: on the GPU through `wgpu` (Metal on macOS, DirectX 12 or Vulkan on Windows), and on the CPU when no GPU is available.

## Install

Download the `Ditherette-macOS` or `Ditherette-Windows` artifact from the latest [After Effects plugin workflow run](https://github.com/mia-cx/ditherette/actions/workflows/after-effects.yml), then copy the plugin into the shared plug-ins folder and restart After Effects.

- macOS: copy `Ditherette.plugin` to `/Library/Application Support/Adobe/Common/Plug-ins/7.0/MediaCore/`, then clear the download quarantine with `xattr -dr com.apple.quarantine "/Library/Application Support/Adobe/Common/Plug-ins/7.0/MediaCore/Ditherette.plugin"`.
- Windows: copy `Ditherette.aex` to `C:\Program Files\Adobe\Common\Plug-ins\7.0\MediaCore\`.

The effect appears under **Effect › Ditherette › Ditherette**.

## Controls

The controls match the website editor:

- **Palette layer**: every fully opaque colour in the chosen layer becomes a palette colour, up to 255. Lospec's PNG palette downloads work as palette layers. With no layer, the effect uses Wplace (Default). Transparency is always kept.
- **Pixel size**: the edge of each output pixel, in layer pixels. Reduced preview resolutions scale it to match the full-resolution render.
- **Resize**, **Dither**, **Strength**, **Serpentine scan**, **Colour space**, **Dither in selected space**, **Placement**, **Sample size** and **Contrast tolerance** work as in the editor.

The effect renders 8 bits per channel; After Effects converts frames in 16- and 32-bit projects. GPU error diffusion uses the banded schedule, which looks the same as the CPU scan but places different pixels.

## Build

After Effects plugins build on the platform they run on, because the PiPL resource takes the build host's format. On macOS or Windows, with the Rust toolchain from the repository root:

```sh
node crates/ditherette-ae/scripts/package.mjs
```

This writes `dist/Ditherette.plugin` (universal, ad-hoc signed) or `dist/Ditherette.aex`. macOS needs `rustup target add aarch64-apple-darwin x86_64-apple-darwin` first.

`cargo test` runs the render core on any OS. `src/plugin.rs`, the only code that touches the After Effects SDK, compiles only for macOS and Windows; `cargo check --target x86_64-pc-windows-msvc` type-checks it from Linux.
