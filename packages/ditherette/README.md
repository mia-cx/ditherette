# ditherette

This workspace owns the future MIT-licensed browser ESM package and its release version. It remains private during implementation.

The current scaffold exports no processing methods. Importing it performs no initialization, network requests, or worker creation. S19 introduces the first supported package call after the reference freeze.

Run `pnpm package:build` at the repository root to compile both Wasm variants and the wrapper. Generated artifacts stay under this package's ignored `dist/` directory. The crate owns compilation; this package stages the scalar, threaded, and worker files for distribution.

The website remains at the repository root as private workspace `ditherette-web`. Its existing Wasm URLs remain available through the root `wasm:build` commands until package adoption.
