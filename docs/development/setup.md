# Development setup

Koebi builds with stable Rust, a C toolchain and CMake (SDL3 is compiled from source). The
Vulkan SDK provides validation layers and the shader compiler.

| Tool | Needed for | Version |
|---|---|---|
| [rustup](https://rustup.rs) | Rust; the toolchain installs itself from `rust-toolchain.toml` | pinned there |
| C/C++ compiler | SDL3 (built by `sdl3-sys`) | MSVC 2022, GCC/Clang, Xcode |
| [CMake](https://cmake.org) | SDL3 | 3.24 or newer |
| [Vulkan SDK](https://vulkan.lunarg.com/sdk/home) | validation layers, `glslc`; loader and MoltenVK on macOS | `.github/vulkan-sdk-version` |
| A Vulkan driver | running the app and GPU tests | any Vulkan 1.2 GPU, or Mesa lavapipe |

Optional tools, used by `cargo xtask ci` when present (skipped otherwise):

```sh
cargo install cargo-nextest --locked   # faster test runner, same as CI
cargo install cargo-deny --locked      # dependency policy (deny.toml)
cargo install typos-cli --locked       # spell checker
cargo install git-cliff --locked       # changelog generation for releases
```

## Windows

1. Install the Visual Studio 2022 Build Tools with the "Desktop development with C++" workload
   (it includes CMake), or Visual Studio with the same workload.
2. Install the Vulkan SDK with the LunarG installer, using the version in
   `.github/vulkan-sdk-version`. It sets `VULKAN_SDK` and adds `glslc` to `PATH`.
3. Install rustup, then build and run:

```powershell
cargo run -p koebi
```

The Vulkan loader ships with GPU drivers; without the SDK, Koebi runs without validation.

## Linux

Ubuntu 24.04 (adapt package names to your distribution):

```sh
sudo apt-get install -y build-essential cmake ninja-build pkg-config \
  libx11-dev libxext-dev libxrandr-dev libxcursor-dev libxfixes-dev libxi-dev \
  libxss-dev libxtst-dev libxkbcommon-dev \
  libwayland-dev wayland-protocols libdecor-0-dev libegl-dev libdrm-dev libgbm-dev \
  libasound2-dev libpulse-dev libdbus-1-dev libudev-dev \
  mesa-vulkan-drivers vulkan-tools
```

Install the Vulkan SDK tarball (pinned version) and load its environment:

```sh
version="$(cat .github/vulkan-sdk-version)"
mkdir -p ~/vulkan-sdk && cd ~/vulkan-sdk
curl -fLO "https://sdk.lunarg.com/sdk/download/$version/linux/vulkansdk-linux-x86_64-$version.tar.xz"
tar -xJf "vulkansdk-linux-x86_64-$version.tar.xz"
source ~/vulkan-sdk/$version/setup-env.sh   # add this line to your shell profile
```

`mesa-vulkan-drivers` provides drivers for AMD and Intel GPUs and the lavapipe software
driver; NVIDIA users install the proprietary driver. `vulkaninfo --summary` lists the GPUs
Koebi can use.

## macOS

1. Install Xcode (or the Command Line Tools) and CMake (`brew install cmake`).
2. Install the Vulkan SDK (pinned version) with the LunarG installer, then load its
   environment in your shell profile:

   ```sh
   source ~/VulkanSDK/<version>/setup-env.sh
   ```

   Koebi loads `$VULKAN_SDK/lib/libvulkan.1.dylib`, which uses MoltenVK. Release bundles
   carry their own copy of MoltenVK (`cargo xtask package`).

## Android and iOS

See [platforms.md](platforms.md): Android needs the NDK, cargo-ndk and a JDK; iOS needs Xcode
and a static MoltenVK build from the MoltenVK releases.

## First steps

```sh
cargo xtask install-hooks        # versioned git hooks: format, commit message, gitflow
cargo run -p koebi               # open the window
cargo xtask ci                   # the same checks as CI
```

Useful environment variables:

| Variable | Effect |
|---|---|
| `KOEBI_LOG` | `off`, `error`, `warn`, `info` (default), `debug`, `trace` |
| `KOEBI_VALIDATION` | `0`, `1` or `required` (fail if the layers are missing) |
| `KOEBI_ADAPTER` | use the first GPU whose name contains this text |
| `KOEBI_REQUIRE_GPU` | `1`: GPU tests fail instead of skipping (CI behaviour) |
| `KOEBI_VULKAN_LIBRARY` | path of the Vulkan loader to open |
| `KOEBI_BLESS` | `1`: golden tests overwrite their reference images |

Editors: any editor with rust-analyzer works. The repository's `.editorconfig` and
`rustfmt.toml` define formatting.

## Updating the toolchain

Rust:

1. Change `channel` in `rust-toolchain.toml` and `rust-version` in the root `Cargo.toml`.
2. Run `cargo xtask ci` and fix new warnings (new clippy lints are expected).
3. Open a pull request titled `build(toolchain): update Rust to X.Y.Z`.

Vulkan SDK:

1. Change `.github/vulkan-sdk-version` and install the same version locally.
2. Run `cargo xtask shaders` and commit any regenerated `.spv` files.
3. Run the GPU tests with validation (`KOEBI_VALIDATION=required`), then open a pull request.

Mesa for Windows CI (lavapipe) is pinned by the `mesa-version` input of
`.github/actions/setup-vulkan`, and MoltenVK for iOS by `.github/moltenvk-version`. Dependabot
proposes updates for Cargo, GitHub Actions and Gradle every week.
