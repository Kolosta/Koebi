#!/usr/bin/env bash
# Runs on a booted Android emulator (CI, see .github/workflows/ci.yml):
#   1. every Rust test binary listed in target/android-test-binaries.txt, through adb;
#   2. a smoke test of the debug APK rendering frames with Vulkan.
set -uo pipefail

root="$(cd "$(dirname "$0")/../.." && pwd)"
out="$root/target/x86_64-linux-android/debug"
remote=/data/local/tmp/koebi
package=io.github.kolosta.koebi
failed=0

adb wait-for-device
adb shell mkdir -p "$remote"
adb push "$out/libSDL3.so" "$remote/" > /dev/null

echo "::group::Rust tests"
while read -r binary; do
    [ -n "$binary" ] || continue
    name="$(basename "$binary")"
    adb push "$binary" "$remote/$name" > /dev/null
    echo "--- $name"
    # Environment variables are not forwarded by adb: pass them explicitly.
    if ! adb shell "cd $remote && LD_LIBRARY_PATH=$remote RUST_BACKTRACE=1 KOEBI_LOG=info ./$name --test-threads=1"; then
        echo "::error title=Android tests::$name failed"
        failed=1
    fi
done < "$root/target/android-test-binaries.txt"
echo "::endgroup::"

echo "::group::APK smoke test"
apk="$(ls "$root"/platform/android/app/build/outputs/apk/debug/*.apk | head -n1)"
adb install -r -g "$apk"
adb logcat -c
adb shell am start -W -n "$package/.KoebiActivity" --es koebi.args "--frames 120"
outcome=timeout
for _ in $(seq 1 120); do
    log="$(adb logcat -d -s koebi:V SDL:V AndroidRuntime:E DEBUG:V)"
    if grep -q "Koebi exits after" <<< "$log" || grep -qE "FATAL|Fatal signal" <<< "$log"; then
        outcome=finished
        break
    fi
    sleep 1
done
echo "$log"
if grep -q "presented 120 frames (validation: 0 errors" <<< "$log"; then
    echo "APK smoke test passed"
elif grep -qE "Vulkan is not available|GPU initialization failed" <<< "$log"; then
    echo "::warning title=Android smoke test::the emulator exposes no usable Vulkan driver; rendering was not exercised"
else
    echo "::error title=Android smoke test::Koebi did not render 120 frames on the emulator ($outcome)"
    failed=1
fi
echo "::endgroup::"
exit "$failed"
