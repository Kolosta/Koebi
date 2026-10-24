#!/usr/bin/env bash
# Custom shell for workflow `run:` steps (`defaults.run.shell: bash .github/scripts/ci-step.sh {0}`).
#
# Runs the step script exactly like GitHub's default bash shell (`bash -eo pipefail`). When the
# step fails, the relevant output lines (compiler errors, panics, CMake errors, test failures)
# are reported as an error annotation, so the cause shows up in the run summary and on pull
# requests without opening the full log.
set -uo pipefail

script="$1"
log="$(mktemp)"

bash --noprofile --norc -eo pipefail "$script" 2>&1 | tee "$log"
status="${PIPESTATUS[0]}"

if [ "$status" -ne 0 ]; then
    # Strip colors, keep error lines with some context, fall back to the end of the output.
    plain="$(sed -E 's/\x1B\[[0-9;]*[A-Za-z]//g' "$log")"
    pattern='(^|[^A-Za-z_-])(error|panicked|FAILED|failures:|fatal|CMake Error|Could not|not found)([^A-Za-z_]|$)'
    excerpt="$(printf '%s\n' "$plain" | grep -E -A 12 "$pattern" | head -n 150)"
    if [ -z "$excerpt" ]; then
        excerpt="$(printf '%s\n' "$plain" | tail -n 60)"
    fi
    # Workflow commands are single-line: encode '%', drop CR, encode LF.
    message="$(printf '%s\n' "$excerpt" | awk 'BEGIN { ORS = "%0A" } { gsub(/%/, "%25"); gsub(/\r/, ""); print }')"
    echo "::error title=Step failed (exit code $status)::$message"
fi

rm -f "$log"
exit "$status"
