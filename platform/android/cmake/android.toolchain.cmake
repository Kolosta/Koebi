# CMake toolchain for the C dependencies cargo builds for Android (SDL3, through sdl3-sys and
# the cmake crate). `cargo xtask android` selects it with TARGET_CMAKE_TOOLCHAIN_FILE; set that
# variable to this file when you call `cargo ndk` directly (CI does it for clippy and tests).
#
# The file carries the NDK toolchain's name so that the cmake crate recognizes an NDK build and
# leaves compiler selection to it. It:
#   1. maps the Rust-style processor names sdl3-sys passes (arm64, arm, x86) to the names
#      CMake's Android support expects (aarch64, armv7-a, i686);
#   2. forwards the API level chosen by cargo-ndk (ANDROID_PLATFORM environment variable);
#   3. delegates everything else to the NDK's own toolchain file.

set(_koebi_processor_arm64 aarch64)
set(_koebi_processor_arm armv7-a)
set(_koebi_processor_x86 i686)
if(DEFINED _koebi_processor_${CMAKE_SYSTEM_PROCESSOR})
  set(CMAKE_SYSTEM_PROCESSOR "${_koebi_processor_${CMAKE_SYSTEM_PROCESSOR}}")
endif()

if(NOT ANDROID_PLATFORM AND DEFINED ENV{ANDROID_PLATFORM})
  set(ANDROID_PLATFORM "$ENV{ANDROID_PLATFORM}")
endif()

if(DEFINED ENV{CARGO_NDK_CMAKE_TOOLCHAIN_PATH})
  set(_koebi_ndk_toolchain "$ENV{CARGO_NDK_CMAKE_TOOLCHAIN_PATH}")
elseif(DEFINED ENV{ANDROID_NDK_HOME})
  set(_koebi_ndk_toolchain "$ENV{ANDROID_NDK_HOME}/build/cmake/android.toolchain.cmake")
elseif(DEFINED ENV{ANDROID_NDK_ROOT})
  set(_koebi_ndk_toolchain "$ENV{ANDROID_NDK_ROOT}/build/cmake/android.toolchain.cmake")
else()
  message(FATAL_ERROR "Android NDK not found: set ANDROID_NDK_HOME to the NDK directory")
endif()

include("${_koebi_ndk_toolchain}")
