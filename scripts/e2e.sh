#!/bin/bash
# E2E UI test of explorer2 (Linux, Xvfb, no window manager).
# Usage: xvfb-run -a -s "-screen 0 1280x800x24" scripts/e2e.sh <path/to/explorer2> <empty-workdir>
# Needs: xdotool, python3; optional: imagemagick (screenshots for debugging).
# Click coordinates assume the default 1100x700 window at (0,0).
set -u
EXE=$1; W=$2; LOG=$W/trace.log; FAIL=0
pass() { echo "PASS  $1"; }
fail() { echo "FAIL  $1"; FAIL=1; }
check() { if eval "$2"; then pass "$1"; else fail "$1"; fi; }
mkdir -p $W; rm -rf $W/big $W/watch $W/*.png; mkdir -p $W/big $W/watch/sub/inner
python3 - "$W/big" <<'PY'
import os,sys
d=sys.argv[1]
for i in range(100000):
    p=os.path.join(d, f"dir_{i}" if i%50==0 else f"file_{i}.txt")
    if i%50==0: os.mkdir(p)
    else: open(p,'w').close()
PY
echo "hi" > $W/watch/a.txt; touch $W/watch/.hidden
export EXPLORER2_TRACE=1 SLINT_BACKEND=winit-software
shot() { command -v import >/dev/null && import -window root "$W/$1.png" || true; }
focus() { xdotool windowfocus --sync "$(xdotool search --name 'Explorer 2.0' | head -1)"; }
ms() { grep -oE "after [0-9.]+(ms|µs|s)" | head -1 | awk '{v=$2; if (v ~ /µs/) {sub(/µs/,"",v); v/=1000} else if (v ~ /ms/) {sub(/ms/,"",v)} else {sub(/s/,"",v); v*=1000}; print int(v)}'; }

echo "== [1] 100k folder"
$EXE $W/big > $LOG 2>&1 & PID=$!
sleep 6; shot s1_big
FP=$(grep "first paint" $LOG | ms); DN=$(grep "done gen=1" $LOG | ms)
echo "first paint ${FP} ms, done ${DN} ms"
check "first paint < 100 ms" "[ -n '$FP' ] && [ $FP -lt 100 ]"
check "full load < 1000 ms" "[ -n '$DN' ] && [ $DN -lt 1000 ]"
check "100.000 shown" "grep -q 'total=100000 shown=100000' $LOG"
kill $PID; wait $PID 2>/dev/null

echo "== [2-6] watch dir scenarios"
$EXE $W/watch/sub > $LOG 2>&1 & PID=$!
sleep 3
touch $W/watch/sub/new_file.txt; sleep 2; shot s2_refresh
check "[2] auto-refresh picks up new file" "grep -q 'done gen=2 total=2' $LOG"
rm -rf $W/watch/sub; sleep 2; shot s3_deleted
check "[3] deleted folder -> parent" "grep -qE 'navigate gen=[0-9]+ User $W/watch at=' $LOG"
focus; xdotool mousemove 400 20 click 1; sleep 0.3
xdotool key ctrl+a; xdotool type --delay 5 "$W/does/not/exist"; xdotool key Return; sleep 1.5; shot s4_invalid
check "[4] invalid path -> NotFound error" "grep -q 'failed gen=[0-9]* NotFound $W/does/not/exist' $LOG"
ADDR=$(grep '^\[trace\] address=' $LOG | tail -1)
check "[4b] address bar restored to current folder" '[ "$ADDR" = "[trace] address=$W/watch" ]' 
BEFORE=$(grep -c "navigate" $LOG)
xdotool mousemove 1040 20 click 1; sleep 1.5; shot s5_hidden
check "[5] hidden toggle reloads" "[ \$(grep -c navigate $LOG) -gt $BEFORE ] && grep -q 'total=2 shown=2' $LOG"
xdotool mousemove 24 20 click 1; sleep 1; shot s6_up
check "[6] go up to e2e" "grep -qE 'navigate gen=[0-9]+ User $W at=' $LOG"
ADDR=$(grep '^\[trace\] address=' $LOG | tail -1)
check "[6b] address bar follows navigation after typing" '[ "$ADDR" = "[trace] address=$W" ]' 
echo "== [7] throttle: own trace.log in watched folder (write-feedback loop)"
N0=$(grep -c "watch: refresh$" $LOG); sleep 3; N1=$(grep -c "watch: refresh$" $LOG)
echo "refreshes in 3 s: $((N1-N0))"
check "[7] <= 7 refreshes in 3 s (was ~30 before fix)" "[ $((N1-N0)) -le 7 ]"
check "process alive" "kill -0 $PID"
kill $PID; wait $PID 2>/dev/null
check "no panic in log" "! grep -qi panic $LOG"
exit $FAIL
