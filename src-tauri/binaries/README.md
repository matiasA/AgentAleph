# llama.cpp Binaries

This folder must contain the precompiled llama.cpp `llama-server` binary and its runtime
libraries (`*.so`) inside `llama-linux-x64/`.

They are **not versioned in git** because they are large (~130 MB). Download them with:

```bash
./scripts/setup-llama.sh
```

That downloads the pinned release (default: `b9754`, `vulkan-x64` flavor) and places it in
`llama-linux-x64/`. For a CPU-only build:

```bash
LLAMA_FLAVOR=x64 ./scripts/setup-llama.sh
```

CI (`.github/workflows/build.yml`) runs the same script before building installers, so these
binaries do not need to be committed.
