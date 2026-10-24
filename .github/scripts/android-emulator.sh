#!/usr/bin/env bash
# Runs on a booted Android emulator (CI, see .github/workflows/ci.yml):
#   1. every Rust test binary listed in target/android-test-binaries.txt, through adb;
#   2. a smoke test of the debug APK rendering frames with Vulkan.
# Failures are reported as error annotations that include the relevant output.
set -uo pipefail

root="$(cd "$(dirname "$0")/../.." && pwd)"
out="$root/target/x86_64-linux-android/debug"
remote=/data/local/tmp/koebi
package=io.github.kolosta.koebi
failed=0

# Prints a workflow error annotation whose message is the given multi-line text.
annotate() {
    local title="$1" text="$2"
    local message
    message="$(printf '%s\n' "$text" | tail -n 80 | awk 'BEGIN { ORS = "%0A" } { gsub(/%/, "%25"); gsub(/\r/, ""); print }')"
    echo "::error title=$title::$message"
}

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
    output="$(adb shell "cd $remote && LD_LIBRARY_PATH=$remote RUST_BACKTRACE=1 KOEBI_LOG=info ./$name --test-threads=1" 2>&1)"
    status=$?
    printf '%s\n' "$output"
    if [ "$status" -ne 0 ]; then
        excerpt="$(printf '%s\n' "$output" | grep -E -A 12 'panicked|FAILED|failures:|error|CANNOT LINK' || printf '%s\n' "$output")"
        annotate "Android tests: $name (exit $status)" "$excerpt"
        failed=1
    fi
done < "$root/target/android-test-binaries.txt"
echo "::endgroup::"

echo "::group::APK smoke test"
apk="$(ls "$root"/platform/android/app/build/outputs/apk/debug/*.apk | head -n1)"
adb install -r -g "$apk"
adb logcat -c
# adb joins its arguments into one remote command line: quote the extra for that shell too.
adb shell am start -W -n "$package/.KoebiActivity" --es koebi.args "'--frames 120'"
outcome=timeout
log=""
for _ in $(seq 1 120); do
    log="$(adb logcat -d -s koebi:V SDL:V AndroidRuntime:E DEBUG:V)"
    if grep -qE "Koebi exits after|Finished main function|FATAL|Fatal signal" <<< "$log"; then
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
    annotate "Android smoke test: Koebi did not render 120 frames ($outcome)" "$log"
    failed=1
fi
echo "::endgroup::"
exit "$failed"
