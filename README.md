# sv

Everything you need to build a Svelte project, powered by [`sv`](https://github.com/sveltejs/cli).

## Creating a project

If you're seeing this, you've probably already done this step. Congrats!

```sh
# create a new project
npx sv create my-app
```

To recreate this project with the same configuration:

```sh
# recreate this project
pnpm dlx sv@0.15.2 create --template minimal --types ts --add prettier playwright tailwindcss="plugins:typography,forms" vitest="usages:unit,component" eslint sveltekit-adapter="adapter:cloudflare+cfTarget:workers" --install pnpm ./
```

## Developing

Once you've created a project and installed dependencies with `npm install` (or `pnpm install` or `yarn`), start a development server:

```sh
npm run dev

# or start the server and open the app in a new browser tab
npm run dev -- --open
```

## Building

To create a production version of your app:

```sh
npm run build
```

You can preview the production build with `npm run preview`.

> To deploy your app, you may need to install an [adapter](https://svelte.dev/docs/kit/adapters) for your target environment.

## Threaded Wasm build prerequisites

`pnpm wasm:build:threads` requires `nightly-2024-08-02`, its `wasm32-unknown-unknown` target, and `rust-src`.
The root `rust-toolchain.toml` configures stable Rust; it does not provision this separate nightly.
Install these prerequisites once before running the threaded build:

```sh
rustup toolchain install nightly-2024-08-02 --profile minimal --component rust-src --target wasm32-unknown-unknown
```

`rust-src` is required by the threaded build's `-Z build-std=panic_abort,std`.
