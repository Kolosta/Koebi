# Platforms

| Platform | Minimum | Vulkan | Windowing | Package |
|---|---|---|---|---|
| Windows | 10, x86-64 | driver loader (`vulkan-1.dll`) | SDL3, static | zip with `koebi.exe` |
| Linux | glibc distro, X11 or Wayland, x86-64 | system loader (`libvulkan.so.1`) | SDL3, static | tar.gz |
| macOS | 11, Apple silicon or Intel | MoltenVK (bundled) | SDL3, static | `Koebi.app` (universal) |
| Android | 8.0 (API 26), arm64 and x86-64 | system loader | SDL3 (`libSDL3.so`) + `KoebiActivity` | APK |
| iOS | 15.0, arm64 | MoltenVK (static) | SDL3, static | `Koebi.app` |

## Desktop

```sh
cargo run -p koebi                          # debug build, validation on when available
cargo run -p koebi --release -- --frames 600 --present-mode mailbox
cargo xtask package                         # release archive for the host in target/dist/
cargo xtask package --universal             # macOS: arm64 + x86-64 Koebi.app
```

Command-line options: `--frames <N>` renders N frames and exits, `--present-mode
fifo|mailbox|immediate` selects pacing, `--help`, `--version`.

macOS packaging needs `VULKAN_SDK` (MoltenVK is copied into `Koebi.app/Contents/Frameworks`).

## Android

Requirements: Android SDK and NDK (Android Studio or command-line tools), JDK 17, the Rust
targets and cargo-ndk.

```sh
rustup target add aarch64-linux-android x86_64-linux-android
cargo install cargo-ndk --locked
export ANDROID_HOME=~/Android/Sdk
export ANDROID_NDK_HOME=$ANDROID_HOME/ndk/<version>
cargo xtask android                 # debug APK for arm64-v8a and x86_64
cargo xtask android --release       # release APK
adb install -r platform/android/app/build/outputs/apk/debug/app-debug.apk
```

How it fits together:

- `cargo xtask android` builds `libkoebi.so` (the `koebi` library as a `cdylib`) and
  `libSDL3.so` per ABI with cargo-ndk, copies them into `app/src/main/jniLibs/`, copies SDL's
  Java glue from the locked `sdl3-src` crate, then runs Gradle.
- SDL is compiled by CMake through `platform/android/cmake/android.toolchain.cmake`, which
  fixes the processor names `sdl3-sys` passes and delegates to the NDK. xtask selects it with
  `TARGET_CMAKE_TOOLCHAIN_FILE`; set that variable yourself when you run `cargo ndk` directly.
- On Windows, install [Ninja](https://ninja-build.org): xtask sets `CMAKE_GENERATOR=Ninja`,
  because CMake's default Visual Studio generator cannot target Android.
- `KoebiActivity` extends `SDLActivity`, loads `SDL3` then `koebi`, and SDL calls the exported
  `SDL_main`, which runs the same `App` as on desktop.
- Arguments can be passed with an intent extra. adb hands the whole command line to the
  device's shell, so quote the value twice:
  `adb shell am start -n io.github.kolosta.koebi/.KoebiActivity --es koebi.args "'--frames 120'"`.
- Logs go to logcat with the tag `koebi`: `adb logcat -s koebi`.
- The app drops its swapchain and surface when it goes to the background and recreates them
  when it returns.
- Release signing reads `KOEBI_ANDROID_KEYSTORE`, `KOEBI_ANDROID_KEYSTORE_PASSWORD`,
  `KOEBI_ANDROID_KEY_ALIAS` and `KOEBI_ANDROID_KEY_PASSWORD`; without them the debug key signs.

The Gradle project in `platform/android/` can be opened in Android Studio after one
`cargo xtask android` run.

## iOS

Requirements: macOS with Xcode, the Rust targets, and a static `MoltenVK.xcframework` with
simulator slices. The Vulkan SDK's iOS component only has device slices, so use the MoltenVK
release pinned in `.github/moltenvk-version`:

```sh
rustup target add aarch64-apple-ios aarch64-apple-ios-sim
version="$(cat .github/moltenvk-version)"
mkdir -p ~/moltenvk && cd ~/moltenvk
curl -fLO "https://github.com/KhronosGroup/MoltenVK/releases/download/v$version/MoltenVK-all.tar"
tar -xf MoltenVK-all.tar MoltenVK/MoltenVK/static/MoltenVK.xcframework
export KOEBI_MOLTENVK_XCFRAMEWORK=~/moltenvk/MoltenVK/MoltenVK/static/MoltenVK.xcframework
cd -
cargo xtask ios --run               # simulator build, install and launch (120 frames)
cargo xtask ios --release --device  # device build, ad-hoc signed
```

- `crates/koebi-gpu/build.rs` links MoltenVK statically from `KOEBI_MOLTENVK_XCFRAMEWORK`
  (falling back to the Vulkan SDK's iOS component for device builds).
- `cargo xtask ios` wraps the binary in `Koebi.app` with `platform/ios/Info.plist` and signs it
  ad hoc.
- To run on a device, sign the bundle with your development identity and an embedded
  provisioning profile (`codesign --sign "Apple Development: ..."`), or create an Xcode
  project that embeds the built binary. An Xcode project generator is planned.
- Tests run inside the simulator: `CARGO_TARGET_AARCH64_APPLE_IOS_SIM_RUNNER` points to
  `.github/scripts/ios-simulator-runner.sh`, which uses `xcrun simctl spawn`.

## Platform differences to keep in mind

- **Surface loss**: Android destroys the native window in the background; any platform may
  report `SurfaceLost`. The app recreates the presenter.
- **Pre-rotation**: Android may report a transformed surface; the presenter uses the identity
  transform for now, which costs a compositor rotation on some devices.
- **Portability subset**: MoltenVK forbids some features (for example triangle fans); the
  device enables `VK_KHR_portability_subset` automatically.
- **High DPI**: windows are created with high pixel density; the scale factor comes from SDL
  and is applied on the GPU.
