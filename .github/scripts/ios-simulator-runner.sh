#!/usr/bin/env bash
# Cargo target runner: executes a test binary inside the booted iOS simulator (CI).
# Configured with CARGO_TARGET_AARCH64_APPLE_IOS_SIM_RUNNER; KOEBI_SIMULATOR holds the UDID.
set -euo pipefail

device="${KOEBI_SIMULATOR:-booted}"
# simctl forwards variables prefixed with SIMCTL_CHILD_ to the simulated process.
export SIMCTL_CHILD_RUST_BACKTRACE=1
while IFS='=' read -r name value; do
    case "$name" in
        KOEBI_*) export "SIMCTL_CHILD_${name}=${value}" ;;
    esac
done < <(env)
exec xcrun simctl spawn "$device" "$@"
