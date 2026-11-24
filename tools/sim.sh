#!/bin/sh
# SPDX-License-Identifier: GPL-2.0-or-later
# sim.sh - builds oos-ui of the OOS commit in OOS_REV (from the checkout OOS_DIR, in a worktree of its own under
# target/oos) and runs it with --serve on port 55555: the demo source behind SCPI, no hardware. With arguments it runs
# them with OOS_SIM set and stops the simulator afterwards: `tools/sim.sh cargo test -p oos-session --test sim`.
set -eu
here=$(cd "$(dirname "$0")/.." && pwd)
: "${OOS_DIR:?OOS_DIR: a checkout of OOS}"
rev=$(cat "$here/OOS_REV")
tree="$here/target/oos"
if [ ! -d "$tree" ]; then
	git -C "$OOS_DIR" fetch -q origin "$rev" 2>/dev/null || true
	git -C "$OOS_DIR" worktree add -q --detach "$tree" "$rev"
fi
git -C "$tree" checkout -q --detach "$rev"
(cd "$tree/ui" && cargo build -q --release)
"$tree/ui/target/release/oos-ui" --serve --scpi-port 55555 --web-port 58080 > "$here/target/oos-sim.log" 2>&1 &
pid=$!
trap 'kill $pid 2>/dev/null' EXIT INT TERM
sleep 2
export OOS_SIM=127.0.0.1:55555
if [ $# -gt 0 ]; then
	"$@"
else
	echo "oos-ui --serve of ${rev%"${rev#????????}"} at $OOS_SIM (Ctrl-C ends it)"
	wait $pid
fi
