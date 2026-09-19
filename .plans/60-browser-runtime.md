# S19 installed-package browser runtime

Validation date: 2026-09-07. Host: Debian GNU/Linux 13.6, x86_64.
Node 24.19.0, pnpm 11.13.0, and package-owned Playwright 1.59.1 drive the fixture.
The package lock pins the driver. Its bundled browser registry pins the revisions below.

The fixture packs the built package, installs that tarball offline into a private temporary consumer,
and serves only that consumer on an ephemeral loopback port. It removes the consumer and closes every browser/server afterward.
It does not use the website, cross-origin isolation headers, a published package, or benchmark timing.

## Verified engines

| Engine                  | Version       | Playwright revision | Executable SHA-256                                                 |
| ----------------------- | ------------- | ------------------- | ------------------------------------------------------------------ |
| Chromium headless shell | 147.0.7727.15 | 1217                | `22d72b589f908111fa29ff4e61ead8a0601e925177f59c70e2e851c22e9b1886` |
| Firefox                 | 148.0.2       | 1511                | `4c6cc6e27feb99dcc6420bbdca0acacf75c6d5e11f9139bd7dc31752b2431391` |
| WebKit WPE MiniBrowser  | 26.4          | 2272                | `d9de960671e7e7b90618b5affb5265bb3ff3bbaf29819fc782599891f17b4c01` |

Chromium was already installed in the Playwright cache. Firefox and WebKit came from the pinned driver's download command:

```sh
pnpm --filter ditherette exec playwright install firefox webkit
```

The reported download URLs were `https://cdn.playwright.dev/dbazure/download/playwright/builds/firefox/1511/firefox-debian-13.zip`
and `https://cdn.playwright.dev/dbazure/download/playwright/builds/webkit/2272/webkit-debian-13.zip`.

## Task-local WebKit libraries

The host lacked GTK4, AVIF16, manette, enchant2, and woff2 libraries.
The coordinator approved downloading and extracting dependencies into `/tmp/ditherette-webkit-libs.2dS6Yu`.
`apt-get --simulate install --no-install-recommends` identified the twelve missing packages below.
`apt-get download` fetched their exact versions from configured Debian `trixie/main` at `http://deb.debian.org/debian`.
`dpkg-deb --extract` unpacked them only inside that private directory. No package installation or maintainer script ran.

| Package                      | Version             | Downloaded .deb SHA-256                                            |
| ---------------------------- | ------------------- | ------------------------------------------------------------------ |
| libaspell15                  | 0.60.8.1-4          | `8be22785198a4e5de1e2c3a157e8cd99ec7d5843c8ac96a174ec701ba4ac28e2` |
| libavif16                    | 1.2.1-1.2           | `ba6c334454d3562560ad583fbe2c998e203b0fc5a0e0e91112ddcd3cff4704bb` |
| libcairo-script-interpreter2 | 1.18.4-1+b1         | `52b45eb6e49c53c0a35ef45555a9457b094482eafef27d27cd777b916a2f06de` |
| libenchant-2-2               | 2.8.2+dfsg1-3       | `83f208d89db1c48e6ede8a23e8ffa2bcddd4f99427bfd1279201247bbf27ca58` |
| libgav1-1                    | 0.19.0-3+b1         | `94010041e59c7a8d2d3d1f5e62afd523f002b7b1caa3189419160b66d71c0195` |
| libgtk-4-1                   | 4.18.6+ds-2         | `51ab1114a57702e703cce745c24caf5bd31ec1d2a900c72fd15fe47c6c030e68` |
| libgtk-4-common              | 4.18.6+ds-2         | `32f0ad042f983b1d53cd2c260d4042eda9041b6debb71beb91937196b1ff3f6b` |
| libhidapi-hidraw0            | 0.14.0-1+b2         | `89c546cb55be403ce63fe668baff2515979100a30fa07e2b97de186b9c07f01e` |
| liblzo2-2                    | 2.10-3+b1           | `f3032201fe2928a87e13f05ce256ff3ac2a7860c7e594dc51ae6d7348a4466ce` |
| libmanette-0.2-0             | 0.2.12-1            | `98841a467692c46fd1870275113235691ebbfe71a3524f6b2a0fa7e9c33c6e56` |
| libwoff1                     | 1.0.2-2+b2          | `628a2a993556027adaa4e93a675fee341e3d47d306db7f3811ff56ea84a42022` |
| libyuv0                      | 0.0.1904.20250204-1 | `1a6827fe94fa5572a032332df7313ed408e9c0ad78f3a1248391bddc6d920ef2` |

The bundled `minibrowser-wpe/MiniBrowser` shell launcher replaces `LD_LIBRARY_PATH`, so an inherited path alone cannot work.
A task-local executable named `webkit` invokes the unchanged `minibrowser-wpe/bin/MiniBrowser` directly.
It sets the bundled `WEBKIT_EXEC_PATH`, `WEBKIT_INJECTED_BUNDLE_PATH`, and `WEBKIT_INSPECTOR_RESOURCES_PATH` locations,
plus `WEBKIT_FORCE_COMPLEX_TEXT=1`, matching the shipped launchers.
Its `LD_LIBRARY_PATH` combines the private `usr/lib/x86_64-linux-gnu` directory with the bundle's `lib` and `sys/lib` directories.
Launcher SHA-256: `1b74ab78fe96fdec76cf1e06ac554d968193ef708b7f81b2a7e8bdb85baea7a8`.
No shared browser binary or launcher changed.

The successful command is:

```sh
DITHERETTE_TEST_WEBKIT_EXECUTABLE=/tmp/ditherette-webkit-libs.2dS6Yu/webkit pnpm --filter ditherette test:browser
```

The override is test-only. Without it, the fixture uses the driver's normal installed WebKit executable.
The temporary library directory remains available for the coordinator's integration rerun; it is not a package dependency or artifact.

## Results and boundary findings

All three browser subtests pass. Node reports four passing tests because it also counts their enclosing tarball fixture.
Each engine verifies nine independently known nearest anchors and eight custom initialization inputs.
It also checks durable output, input preservation, isolated failures, canonical errors, inert imports, and scalar-only network requests.

Two package defects surfaced through the real artifact:

- A copied wasm-pack `.gitignore` containing `*` omitted all generated assets from the tarball. Staging now excludes that file.
- Chromium rejects a DataView passed directly to WebAssembly.instantiate. A same-buffer Uint8Array preserves its offset and length without copying bytes.

Both corrections have retained package-owned fixtures. The node interface suite additionally covers exact capacity boundaries,
getter/copy-boundary reentry, detached input, memory-growth durability, and caught-copy recovery without externref growth.
