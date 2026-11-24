#!/bin/sh
# SPDX-License-Identifier: GPL-2.0-or-later
# check.sh - every check CI runs, in layers: fmt clippy test layers deps scpi text. `tools/check.sh` runs them all,
# `tools/check.sh test` one, `tools/check.sh list` names them. A warning fails. `scpi` needs OOS_DIR (a checkout of
# OOS) and says so without it.
set -u
cd "$(dirname "$0")/.."
all="fmt clippy test layers deps scpi text"
[ "${1:-}" = list ] && { echo "$all"; exit 0; }
failed=
run() {
	case $1 in
	fmt) cargo fmt --all -- --check ;;
	clippy) cargo clippy -q --workspace --all-targets -- -D warnings ;;
	test) RUSTFLAGS="${RUSTFLAGS:-} -D warnings" cargo test -q --workspace ;;
	layers) "${PYTHON:-python3}" tools/layers.py ;;
	deps) "${PYTHON:-python3}" tools/deps.py ;;
	scpi) "${PYTHON:-python3}" tools/scpi-used.py ;;
	text) "${PYTHON:-python3}" tools/text.py ;;
	*) echo "no layer $1 (tools/check.sh list)"; return 1 ;;
	esac
}
for l in ${1:-$all}; do
	echo "== $l"
	run "$l" || failed="$failed $l"
done
[ -z "$failed" ] && { echo "== passed: ${1:-$all}"; exit 0; }
echo "== failed:$failed"
exit 1
