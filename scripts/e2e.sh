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
mkdir -p $W; rm -rf $W/big $W/watch $W/config $W/*.png; mkdir -p $W/big $W/watch/sub/inner
python3 - "$W/big" <<'PY'
import os,sys
d=sys.argv[1]
for i in range(100000):
    p=os.path.join(d, f"dir_{i}" if i%50==0 else f"file_{i}.txt")
    if i%50==0: os.mkdir(p)
    else: open(p,'w').close()
PY
echo "hi" > $W/watch/a.txt; touch $W/watch/.hidden
export EXPLORER2_TRACE=1 SLINT_BACKEND=winit-software EXPLORER2_CONFIG_DIR=$W/config
CFG=$W/config/settings.ini
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
focus; xdotool key ctrl+l; sleep 0.3
xdotool key ctrl+a; xdotool type --delay 5 "$W/does/not/exist"; xdotool key Return; sleep 1.5; shot s4_invalid
check "[4] invalid path -> NotFound error" "grep -q 'failed gen=[0-9]* NotFound $W/does/not/exist' $LOG"
ADDR=$(grep '^\[trace\] address=' $LOG | tail -1)
check "[4b] address bar restored to current folder" '[ "$ADDR" = "[trace] address=$W/watch" ]' 
BEFORE=$(grep -c "navigate" $LOG)
xdotool mousemove 880 21 click 1; sleep 1.5; shot s5_hidden
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
echo "== [8] settings panel: disable hardware acceleration"
xdotool mousemove 1043 21 click 1; sleep 0.8; shot s8_panel
xdotool mousemove 735 93 click 1; sleep 0.8; shot s8_toggled
check "[8] setting saved to settings.ini" "grep -qx 'hardware_acceleration = false' $CFG"
check "[9] show_hidden persisted" "grep -qx 'show_hidden = true' $CFG"
kill $PID; wait $PID 2>/dev/null
check "no panic in log" "! grep -qi panic $LOG"

echo "== [10] restart honours settings (no SLINT_BACKEND -> app decides)"
env -u SLINT_BACKEND $EXE $W/watch > $LOG 2>&1 & PID=$!
sleep 3; shot s10_restart
check "[10] renderer software because disabled in settings" "grep -q 'renderer choice=Software reason=\"hardware acceleration disabled in settings\"' $LOG"
check "[10b] hidden files still shown after restart" "grep -q 'done gen=1 total=2 shown=2' $LOG"
check "[10c] process alive" "kill -0 $PID"
xprop -id "$(xdotool search --name 'Explorer 2.0' | head -1)" -notype 32c _NET_WM_ICON > $W/icon.txt 2>/dev/null
check "[11] window icon set (yellow folder)" "python3 -c \"
import sys
v=[int(x) for x in open('$W/icon.txt').read().split('=',1)[1].replace(chr(10),'').split(',')]
w,h=v[0],v[1]; p=v[2+(h//2+20)*w+w//2]
r,g,b=(p>>16)&255,(p>>8)&255,p&255
sys.exit(0 if w>=32 and r>200 and g>150 and b<120 else 1)\""
kill $PID; wait $PID 2>/dev/null
check "[10d] no panic" "! grep -qi panic $LOG"

echo "== [12-26] M2: details view, sorting, selection, keyboard, open, breadcrumb"
M=$W/m2; rm -rf "$M"; mkdir -p "$M/Bilder" "$M/Musik"
for f in b.txt a.png foto2.png foto10.png Makefile; do echo x > "$M/$f"; done
OPENLOG=$W/opened.txt; OPENER=$W/opener.sh; rm -f "$OPENLOG"
printf '#!/bin/sh\nprintf "%%s\\n" "$1" >> "%s"\n' "$OPENLOG" > "$OPENER"; chmod +x "$OPENER"
EXPLORER2_OPEN_CMD=$OPENER $EXE "$M" > $LOG 2>&1 & PID=$!
sleep 2.5; focus
last() { grep "$1" $LOG | tail -1; }
row_y() { echo $((70 + 24 * $1 + 12)); }
check "[12] name sort: folders first, natural order" "last '\[trace\] order' | grep -qF '[\"Bilder\", \"Musik\", \"a.png\", \"b.txt\", \"foto2.png\", \"foto10.png\", \"Makefile\"]'"
xdotool mousemove 920 57 click 1; sleep 1.2
check "[13] header 'Typ' sorts by type (no ext first, then natural)" "grep -q 'sort key=type desc=false' $LOG && last '\[trace\] order' | grep -qF '[\"Bilder\", \"Musik\", \"Makefile\", \"a.png\", \"foto2.png\", \"foto10.png\", \"b.txt\"]'"
xdotool mousemove 920 57 click 1; sleep 1.2
check "[14] second click reverses direction" "grep -q 'sort key=type desc=true' $LOG && last '\[trace\] order' | grep -qF '[\"Musik\", \"Bilder\", \"b.txt\"'"
check "[15] sort persisted in settings.ini" "grep -qx 'sort_key = type' $CFG && grep -qx 'sort_desc = true' $CFG"
xdotool mousemove 300 57 click 1; sleep 1.2
xdotool mousemove 300 $(row_y 0) click 1; sleep 0.4
xdotool key Down Down; sleep 0.5
check "[16] arrow keys move selection" "last 'selection count' | grep -q 'count=1 focus=a.png'"
xdotool key shift+End; sleep 0.5
check "[17] Shift+End extends selection to the end" "last 'selection count' | grep -q 'count=5 focus=Makefile'"
xdotool key ctrl+a; sleep 0.5
check "[18] Ctrl+A selects all" "last 'selection count' | grep -q 'count=7 '"
xdotool mousemove 900 650; sleep 0.3; import -window root $W/sel.png 2>/dev/null
PX=$(convert $W/sel.png -format "%[pixel:p{400,$(row_y 2)}]" info: 2>/dev/null)
check "[19] selected rows are drawn in selection color (pixel $PX)" "[ '$PX' = 'srgb(204,232,255)' ]"
xdotool key Home; sleep 0.3; xdotool type --delay 60 "fo"; sleep 0.5
check "[20] type-to-select jumps to 'fo…'" "last 'selection count' | grep -q 'count=1 focus=foto2.png'"
xdotool key Return; sleep 1
check "[21] Enter opens a file with its default app" "grep -qxF '$M/foto2.png' $OPENLOG"
xdotool key Home Return; sleep 1.2
check "[22] Enter on a folder navigates into it" "grep -qE 'navigate gen=[0-9]+ User $M/Bilder at=' $LOG"
xdotool key BackSpace; sleep 1.2
check "[23] Backspace goes up and selects the folder we came from" "last 'selection count' | grep -q 'count=1 focus=Bilder'"
xdotool key ctrl+l; sleep 0.3; xdotool type --delay 5 "/nonsense"; xdotool key Escape; sleep 0.5
check "[24] Esc in the address bar restores the real path" "last 'address(cancel)=' | grep -qxF '[trace] address(cancel)=$M'"
xdotool mousemove 300 $(row_y 1) click --repeat 2 --delay 80 1; sleep 1.2
check "[25] double-click opens a folder" "grep -qE 'navigate gen=[0-9]+ User $M/Musik at=' $LOG"
xdotool mousemove 62 21 click 1; sleep 1.2
NAVP=$(grep -oE 'navigate gen=[0-9]+ User .* at=' $LOG | tail -1 | sed -E 's/navigate gen=[0-9]+ User (.*) at=/\1/')
# "$NAVP" must be a proper ancestor; the root "/" counts too (short CI paths show no "…")
OK=0; [ -n "$NAVP" ] && [ "$NAVP" != "$M/Musik" ] && case "$M/Musik/" in "${NAVP%/}/"*) OK=1;; esac
check "[26] breadcrumb click navigates to an ancestor ($NAVP)" "[ $OK = 1 ]"
check "[26b] process alive" "kill -0 $PID"
kill $PID; wait $PID 2>/dev/null
check "[26c] no panic" "! grep -qi panic $LOG"
exit $FAIL
