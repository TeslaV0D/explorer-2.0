#!/bin/bash
# E2E UI test of explorer2 (Linux, Xvfb, no window manager).
# Usage: xvfb-run -a -s "-screen 0 1280x800x24" scripts/e2e.sh <path/to/explorer2> <empty-workdir>
# Needs: xdotool, python3; optional: imagemagick (screenshots + pixel checks).
set -u
EXE=$1; W=$2; LOG=$W/trace.log; FAIL=0
pass() { echo "PASS  $1"; }
fail() { echo "FAIL  $1"; FAIL=1; }
check() { if eval "$2"; then pass "$1"; else fail "$1"; fi; }

# ---- click targets: by name from the app's UI dump (scripts/ui-lib.sh, redesign R0);
# rows by index below the column header; LIST_X lies inside the name column ----
. "$(dirname "$0")/ui-lib.sh"
LIST_X=500
row_y() { echo $(( $(ui_list_top) + 24 * $1 + 12 )); }

mkdir -p $W; rm -rf $W/big $W/watch $W/config $W/m2 $W/home $W/*.png; mkdir -p $W/big $W/watch/sub/inner
python3 - "$W/big" <<'PY'
import os,sys
d=sys.argv[1]
for i in range(100000):
    p=os.path.join(d, f"dir_{i}" if i%50==0 else f"file_{i}.txt")
    if i%50==0: os.mkdir(p)
    else: open(p,'w').close()
PY
echo "hi" > $W/watch/a.txt; touch $W/watch/.hidden
# Own HOME so the sidebar shows predictable quick-access folders.
mkdir -p "$W/home/Desktop" "$W/home/Downloads/dl-sub" "$W/home/Documents"
# the search index has its own block (e2e-search.sh); here it would only add load
export EXPLORER2_TRACE=1 SLINT_BACKEND=winit-software EXPLORER2_CONFIG_DIR=$W/config HOME=$W/home EXPLORER2_INDEX=off
CFG=$W/config/settings.ini
SESSION=$W/config/session.ini
shot() { command -v import >/dev/null && import -window root "$W/$1.png" || true; }
focus() { xdotool windowfocus --sync "$(xdotool search --name 'Explorer 2.0' | head -1)"; }
ms() { grep -oE "after [0-9.]+(ms|µs|s)" | head -1 | awk '{v=$2; if (v ~ /µs/) {sub(/µs/,"",v); v/=1000} else if (v ~ /ms/) {sub(/ms/,"",v)} else {sub(/s/,"",v); v*=1000}; print int(v)}'; }
last() { grep "$1" $LOG | tail -1; }
navigated_to() { grep -qE "navigate tab=[0-9]+ gen=[0-9]+ User $1 at=" $LOG; }

LOG=$W/trace-1.log
echo "== [1] 100k folder"
$EXE $W/big > $LOG 2>&1 & PID=$!
sleep 6; shot s1_big
FP=$(grep "first paint tab=1 " $LOG | ms); DN=$(grep "done tab=1 " $LOG | head -1 | ms)
echo "first paint ${FP} ms, done ${DN} ms"
check "first paint < 100 ms" "[ -n '$FP' ] && [ $FP -lt 100 ]"
check "full load < 1000 ms" "[ -n '$DN' ] && [ $DN -lt 1000 ]"
check "100.000 shown" "grep -q 'total=100000 shown=100000' $LOG"
check "[1d] a new installation starts dark (redesign D-026)" "grep -q 'theme why=init choice=dark dark=true' $LOG"
check "[1e] no global bar any more: file actions + ⋮ sit in the pane, settings in the status bar (R2)" "ui_has 'Neuer Ordner' && ui_has 'Mehr' && ui_has 'Einstellungen' && ! ui_has 'Zwei Ansichten'"
kill $PID; wait $PID 2>/dev/null

LOG=$W/trace-2.log
echo "== [2-6] watch dir scenarios"
$EXE $W/watch/sub > $LOG 2>&1 & PID=$!
sleep 3
touch $W/watch/sub/new_file.txt; sleep 2; shot s2_refresh
check "[2] auto-refresh picks up new file" "grep -qE 'done tab=1 gen=[0-9]+ total=2 ' $LOG"
rm -rf $W/watch/sub; sleep 2; shot s3_deleted
check "[3] deleted folder -> parent" "navigated_to '$W/watch'"
focus; xdotool key ctrl+l; sleep 0.3
xdotool key ctrl+a; xdotool type --delay 5 "$W/does/not/exist"; xdotool key Return; sleep 1.5; shot s4_invalid
check "[4] invalid path -> NotFound error" "grep -qE 'failed tab=1 gen=[0-9]+ NotFound $W/does/not/exist' $LOG"
ADDR=$(grep '^\[trace\] address=' $LOG | tail -1)
check "[4b] address bar restored to current folder" '[ "$ADDR" = "[trace] address=$W/watch" ]'
BEFORE=$(grep -c "navigate" $LOG)
ui_click "Mehr"; sleep 0.5; ui_click "Ausgeblendete Elemente anzeigen" last; sleep 1.5; shot s5_hidden
check "[5] hidden toggle reloads" "[ \$(grep -c navigate $LOG) -gt $BEFORE ] && grep -q 'total=2 shown=2' $LOG"
ui_click "Nach oben"; sleep 1; shot s6_up
check "[6] go up to e2e" "navigated_to '$W'"
ADDR=$(grep '^\[trace\] address=' $LOG | tail -1)
check "[6b] address bar follows navigation after typing" '[ "$ADDR" = "[trace] address=$W" ]'
echo "== [7] throttle: own trace.log in watched folder (write-feedback loop)"
N0=$(grep -c "watch: refresh$" $LOG); sleep 3; N1=$(grep -c "watch: refresh$" $LOG)
echo "refreshes in 3 s: $((N1-N0))"
check "[7] <= 7 refreshes in 3 s (was ~30 before fix)" "[ $((N1-N0)) -le 7 ]"
check "process alive" "kill -0 $PID"
echo "== [7b] UI dump (redesign R0): named targets with positions"
check "[7b] dump lists buttons, icon buttons (spoken names), list rows" "ui_has 'Einstellungen' && ui_has 'Zurück' && ui_has 'Nach oben' && ui_has 'Adresszeile' && ui_has 'watch'"
check "[7c] positions are inside the window" "awk -F'\t' 'NR>1 && (\$2<0 || \$3<0 || \$2>1200 || \$3>720){bad=1} END{exit bad}' $UI_DUMP"
check "[7d] navigation row (R4): address bar and icon buttons 26 px high, list header right below the 34 px row" "[ \$(ui_box 'Adresszeile' | cut -d' ' -f4) -eq 26 ] && [ \$(ui_box 'Zurück' | cut -d' ' -f4) -eq 26 ] && [ \$(( \$(ui_box 'Name' | cut -d' ' -f2) - \$(ui_box 'Adresszeile' | cut -d' ' -f2) )) -le 32 ]"
xdotool mousemove 900 680; sleep 0.4; import -window root "$W/s7e_tabs.png" 2>/dev/null
# the tab is named after the current folder ($W since [6]); tabs come before the crumbs in the dump
TB=$(ui_box "$(basename "$W")" | awk '{print $1 + int($3 / 2), $2 + 1}')
TBP=$(convert "$W/s7e_tabs.png" -format "%[pixel:p{${TB% *},${TB#* }}]" info: 2>/dev/null)
check "[7e] the current tab of the active pane carries the accent line (R5: $TBP)" "[ '$TBP' = 'srgb(79,140,255)' ]"
echo "== [8] settings panel: disable hardware acceleration"
ui_click "Einstellungen"; sleep 0.8; shot s8_panel
ui_click "Hardwarebeschleunigung verwenden, wenn verfügbar"; sleep 0.8; shot s8_toggled
check "[8] setting saved to settings.ini" "grep -qx 'hardware_acceleration = false' $CFG"
check "[9] show_hidden persisted" "grep -qx 'show_hidden = true' $CFG"
ui_click "Einstellungen"; sleep 0.3   # close panel again
kill $PID; wait $PID 2>/dev/null
check "no panic in log" "! grep -qi panic $LOG"

LOG=$W/trace-10.log
echo "== [10] restart honours settings (no SLINT_BACKEND -> app decides)"
env -u SLINT_BACKEND $EXE $W/watch > $LOG 2>&1 & PID=$!
sleep 3; shot s10_restart
check "[10] renderer software because disabled in settings" "grep -q 'renderer choice=Software reason=\"hardware acceleration disabled in settings\"' $LOG"
check "[10b] hidden files still shown after restart" "grep -qE 'done tab=1 gen=[0-9]+ total=2 shown=2' $LOG"
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

LOG=$W/trace-12.log
echo "== [12-26] M2: details view, sorting, selection, keyboard, open, breadcrumb"
M=$W/m2; mkdir -p "$M/Bilder" "$M/Musik"
for f in b.txt a.png foto2.png foto10.png Makefile; do echo x > "$M/$f"; done
OPENLOG=$W/opened.txt; OPENER=$W/opener.sh; rm -f "$OPENLOG"
printf '#!/bin/sh\nprintf "%%s\\n" "$1" >> "%s"\n' "$OPENLOG" > "$OPENER"; chmod +x "$OPENER"
EXPLORER2_OPEN_CMD=$OPENER $EXE "$M" > $LOG 2>&1 & PID=$!
sleep 2.5; focus
check "[12] name sort: folders first, natural order" "last '\[trace\] order' | grep -qF '[\"Bilder\", \"Musik\", \"a.png\", \"b.txt\", \"foto2.png\", \"foto10.png\", \"Makefile\"]'"
ui_click "Typ"; sleep 1.2
check "[13] header 'Typ' sorts by type (no ext first, then natural)" "grep -q 'sort key=type desc=false' $LOG && last '\[trace\] order' | grep -qF '[\"Bilder\", \"Musik\", \"Makefile\", \"a.png\", \"foto2.png\", \"foto10.png\", \"b.txt\"]'"
ui_click "Typ"; sleep 1.2
check "[14] second click reverses direction" "grep -q 'sort key=type desc=true' $LOG && last '\[trace\] order' | grep -qF '[\"Musik\", \"Bilder\", \"b.txt\"'"
check "[15] sort persisted in settings.ini" "grep -qx 'sort_key = type' $CFG && grep -qx 'sort_desc = true' $CFG"
ui_click "Name"; sleep 1.2
xdotool mousemove $LIST_X $(row_y 0) click 1; sleep 0.4
xdotool key Down Down; sleep 0.5
check "[16] arrow keys move selection" "last 'selection pane' | grep -q 'count=1 focus=a.png'"
xdotool key shift+End; sleep 0.5
check "[17] Shift+End extends selection to the end" "last 'selection pane' | grep -q 'count=5 focus=Makefile'"
xdotool key ctrl+a; sleep 0.5
check "[18] Ctrl+A selects all" "last 'selection pane' | grep -q 'count=7 '"
xdotool mousemove 900 680; sleep 0.3; import -window root $W/sel.png 2>/dev/null
PX=$(convert $W/sel.png -format "%[pixel:p{$LIST_X,$(row_y 2)}]" info: 2>/dev/null)
check "[19] selected rows are drawn in selection color (dark = default, pixel $PX)" "[ '$PX' = 'srgb(28,63,99)' ]"
xdotool key Home; sleep 0.3; xdotool type --delay 60 "fo"; sleep 0.5
check "[20] type-to-select jumps to 'fo…'" "last 'selection pane' | grep -q 'count=1 focus=foto2.png'"
xdotool key Return; sleep 1
check "[21] Enter opens a file with its default app" "grep -qxF '$M/foto2.png' $OPENLOG"
xdotool key Home Return; sleep 1.2
check "[22] Enter on a folder navigates into it" "navigated_to '$M/Bilder'"
xdotool key BackSpace; sleep 1.2
check "[23] Backspace goes up and selects the folder we came from" "last 'selection pane' | grep -q 'count=1 focus=Bilder'"
xdotool key ctrl+l; sleep 0.3; xdotool type --delay 5 "/nonsense"; xdotool key Escape; sleep 0.5
check "[24] Esc in the address bar restores the real path" "last 'address(cancel)=' | grep -qxF '[trace] address(cancel)=$M'"
xdotool mousemove $LIST_X $(row_y 1) click --repeat 2 --delay 80 1; sleep 1.2
check "[25] double-click opens a folder" "navigated_to '$M/Musik'"
ui_click_after "Adresszeile";   # first breadcrumb sleep 1.2
NAVP=$(grep -oE 'navigate tab=[0-9]+ gen=[0-9]+ User .* at=' $LOG | tail -1 | sed -E 's/navigate tab=[0-9]+ gen=[0-9]+ User (.*) at=/\1/')
# "$NAVP" must be a proper ancestor; the root "/" counts too (short CI paths show no "…")
OK=0; [ -n "$NAVP" ] && [ "$NAVP" != "$M/Musik" ] && case "$M/Musik/" in "${NAVP%/}/"*) OK=1;; esac
check "[26] breadcrumb click navigates to an ancestor ($NAVP)" "[ $OK = 1 ]"
kill $PID; wait $PID 2>/dev/null
check "[26c] no panic" "! grep -qi panic $LOG"

LOG=$W/trace-27.log
echo "== [27-36] M3: sidebar, history, tabs, split view, session"
rm -f "$SESSION"
$EXE "$M" > $LOG 2>&1 & PID=$!
sleep 2.5; focus
check "[27] sidebar lists quick-access folders from HOME" "grep -q 'drives loaded n=' $LOG"
ui_click "Downloads"; sleep 1.2   # sidebar item
check "[27b] sidebar click navigates (Downloads)" "navigated_to '$W/home/Downloads'"
xdotool mousemove $LIST_X $(row_y 0) click --repeat 2 --delay 80 1; sleep 1.2
check "[28] into dl-sub" "navigated_to '$W/home/Downloads/dl-sub'"
xdotool key alt+Left; sleep 1.2
check "[28b] Alt+Left goes back" "last 'history' | grep -qF 'history back -> $W/home/Downloads'"
xdotool key alt+Left; sleep 1.2
check "[28c] back again to the start folder" "last 'history' | grep -qF 'history back -> $M'"
xdotool key alt+Right; sleep 1.2
check "[28d] Alt+Right goes forward" "last 'history' | grep -qF 'history forward -> $W/home/Downloads'"
xdotool mousemove $LIST_X 500 click 8; sleep 1.2   # X11 button 8 = mouse "back" side button
check "[28e] mouse back button" "[ \$(grep -c 'history back' $LOG) -ge 3 ] && last 'history' | grep -qF 'history back -> $M'"
xdotool mousemove $(ui_xy "Zurück"); sleep 0.2
xdotool key ctrl+t; sleep 1.2
check "[29] Ctrl+T opens a new tab on the same folder" "grep -q 'tab new pane=0' $LOG && last 'layout\|session saved' | grep -q 'session saved'"
xdotool key ctrl+l; sleep 0.3; xdotool key ctrl+a; xdotool type --delay 5 "$W/home/Documents"; xdotool key Return; sleep 1.2
check "[29b] new tab navigates independently" "navigated_to '$W/home/Documents'"
xdotool key ctrl+Tab; sleep 1
check "[29c] Ctrl+Tab switches to the other tab" "last 'tab select' | grep -q 'pane=0 tab_index=0'"
xdotool key ctrl+backslash; sleep 1.5
check "[30] Ctrl+\\ opens the split view" "grep -q 'split on' $LOG"
xdotool key Down; sleep 0.5
check "[30a] keyboard focus moved to the new right pane" "last 'selection pane' | grep -q 'pane=1 '"
xdotool key F6; sleep 0.5
check "[30b] F6 switches the active pane" "last 'pane active' | grep -q 'pane active=0'"
xdotool mousemove 1000 $(row_y 0) click 1; sleep 0.5   # click into the right pane
check "[30c] clicking a pane activates it" "last 'pane active' | grep -q 'pane active=1'"
sleep 1   # session save is debounced (400 ms)
check "[31] session saved (2 panes, pane 0 with 2 tabs)" "grep -q 'split = true' $SESSION && [ \$(awk '/\\[pane\\]/{n++} n==1 && /^tab = /{c++} END{print c+0}' $SESSION) = 2 ]"
kill $PID; wait $PID 2>/dev/null
check "[31b] no panic" "! grep -qi panic $LOG"

LOG=$W/trace-32.log
echo "== [32] restart without a path restores the session"
$EXE > $LOG 2>&1 & PID=$!
sleep 2.5; focus
check "[32] restored layout: 2 panes, tabs [2, 1], split, right pane active" "grep -qF 'layout panes=2 tabs=[2, 1] split=true active_pane=1' $LOG"
NAVS=$(grep -c 'navigate tab=' $LOG)
check "[32b] only the visible tab of each pane is loaded at start (lazy)" "[ $NAVS = 2 ]"
xdotool mousemove $(ui_xy "Neuer Tab" | awk '{print 400, $2}'); sleep 0.2   # over the tab strip
xdotool key F6; sleep 0.3; xdotool key ctrl+Tab; sleep 1.2
check "[32c] selecting the restored inactive tab loads it" "[ \$(grep -c 'navigate tab=' $LOG) -gt $NAVS ]"
xdotool key ctrl+backslash; sleep 1
check "[33] Ctrl+\\ closes the split view again" "grep -q 'split off' $LOG"
xdotool key ctrl+w; sleep 1
check "[34] Ctrl+W closes a tab" "grep -q 'tab close pane=0' $LOG"
BEFORE=$(grep -c 'tab close' $LOG)
xdotool key ctrl+w; sleep 0.8
check "[35] the last tab of the window is kept" "[ \$(grep -c 'tab close' $LOG) = $BEFORE ] && kill -0 $PID"
kill $PID; wait $PID 2>/dev/null
check "[36] no panic" "! grep -qi panic $LOG"

LOG=$W/trace-37.log
echo "== [37] a path argument wins over the saved session"
$EXE "$W/home/Desktop" > $LOG 2>&1 & PID=$!
sleep 2
check "[37] start path opens a single tab" "grep -qF 'layout panes=1 tabs=[1] split=false active_pane=0' $LOG && navigated_to '$W/home/Desktop'"
kill $PID; wait $PID 2>/dev/null

# ---- M4 file operations (own script, runnable alone)
"$(dirname "$0")/e2e-ops.sh" "$EXE" "$W" || FAIL=1
# ---- M8.3c endurance (own script, runnable alone)
"$(dirname "$0")/e2e-stress.sh" "$EXE" "$W" || FAIL=1
# ---- S1.4 search (own script, runnable alone)
"$(dirname "$0")/e2e-search.sh" "$EXE" "$W" || FAIL=1
exit $FAIL
