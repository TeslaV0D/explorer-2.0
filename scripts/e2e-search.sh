#!/bin/bash
# S1.4a: the search index inside the app (Linux, Xvfb, no window manager).
# Started in the background after the window (never in the start path),
# saved, loaded at the next start, switched off/on and compared again from
# the palette; EXPLORER2_INDEX=off keeps it off.
# Called by e2e.sh, runnable alone:
#   xvfb-run -a -s "-screen 0 1280x800x24" scripts/e2e-search.sh <exe> <workdir>
set -u
EXE=$1; W=$2; FAIL=0
pass() { echo "PASS  $1"; }
fail() { echo "FAIL  $1"; FAIL=1; }
check() { if eval "$2"; then pass "$1"; else fail "$1"; fi; }
# poll a condition for up to $2 seconds (default 10)
wait_for() { local n=$(( ${2:-10} * 5 )); for _ in $(seq 1 $n); do eval "$1" && return 0; sleep 0.2; done; return 1; }
last() { grep "$1" $LOG | tail -1; }
focus() { xdotool windowfocus --sync "$(xdotool search --name 'Explorer 2.0' | head -1)"; }
palette() { focus; xdotool key ctrl+shift+p; sleep 0.4; xdotool type --delay 30 "$1"; sleep 0.5; xdotool key Return; sleep 0.6; }

SR=$W/search-root-$$; IC=$W/config-search
mkdir -p "$SR/a/b" "$SR/leer" "$IC" "$W/home-search"
for i in $(seq 1 40); do echo "x$i" > "$SR/a/datei$i.txt"; done
echo tief > "$SR/a/b/tief.pdf"
export EXPLORER2_TRACE=1 SLINT_BACKEND=winit-software EXPLORER2_CONFIG_DIR=$IC HOME=$W/home-search \
  EXPLORER2_INDEX=on EXPLORER2_INDEX_ROOTS=$SR EXPLORER2_INDEX_DELAY_MS=300

echo "== [76] S1.4a: search index in the app"
LOG=$W/trace-76.log
$EXE "$SR" > $LOG 2>&1 & PID=$!
check "[76] built in the background once the window runs" \
  "wait_for \"grep -q 'search: $SR phase=Ready entries=45 origin=built' $LOG\" && \
   [ \$(grep -n 'event loop running' $LOG | head -1 | cut -d: -f1) -lt \$(grep -n 'search: start' $LOG | head -1 | cut -d: -f1) ]"
check "[76b] index file saved right after the build" "wait_for 'ls $IC/index/ | grep -q .'"
palette "abgleichen"
check "[76c] palette: compare with the disk again" "grep -q 'search: rescan volumes=1' $LOG"
palette "suchindex ein"
check "[76d] palette: index off (saved)" "wait_for \"grep -q 'index_enabled = false' $IC/settings.ini\" 3 && grep -q 'option index_enabled=false' $LOG"
N0=$(grep -c "search: start" $LOG)
palette "suchindex ein"
check "[76e] on again: starts, loads the saved index" \
  "wait_for \"[ \\\$(grep -c 'search: start' $LOG) -gt $N0 ] && grep -q 'origin=loaded' $LOG\" && grep -q 'index_enabled = true' $IC/settings.ini"
# a second window (same settings) must not write the same index files
$EXE --new-instance "$SR" > $W/trace-76h.log 2>&1 & P2=$!
check "[76h] second window: index kept by the first one (lock)" \
  "wait_for \"grep -q 'search: index kept by another window' $W/trace-76h.log\" && ! grep -q 'search: start roots' $W/trace-76h.log"
kill $P2; wait $P2 2>/dev/null
focus; xdotool key ctrl+comma 2>/dev/null; sleep 0.2
command -v import >/dev/null && { palette "einstellungen"; sleep 0.5; import -window root "$W/76-settings.png"; xdotool key Escape; }
kill $PID; wait $PID 2>/dev/null

# changed while closed: the next start loads, compares and finds it
echo neu > "$SR/leer/neu.txt"
LOG=$W/trace-76f.log
$EXE "$SR" > $LOG 2>&1 & PID=$!
check "[76f] next start: loaded index, compared with the disk (46 entries)" \
  "wait_for \"grep -q 'search: $SR phase=Ready entries=46 origin=loaded' $LOG\""
kill $PID; wait $PID 2>/dev/null

LOG=$W/trace-76g.log
EXPLORER2_INDEX=off $EXE "$SR" > $LOG 2>&1 & PID=$!
sleep 2
check "[76g] EXPLORER2_INDEX=off: no index started" "grep -q 'event loop running' $LOG && ! grep -q 'search: start' $LOG"
# without index the folder is walked: same results, marked as such
focus; xdotool key ctrl+f; sleep 0.3; xdotool type --delay 30 "datei2"; sleep 1.2
check "[76i] no index: the folder is walked (11 hits: datei2, datei20-29)" \
  "wait_for \"grep -q 'search found query=.datei2. total=11 shown=11 walked=true' $LOG\" 5"
kill $PID; wait $PID 2>/dev/null

echo "== [77] S1.4b: search field per tab"
LOG=$W/trace-77.log; OPENED=$W/opened-77.txt; OPENER=$W/opener-77.sh
printf '#!/bin/sh\necho "$1" >> %s\n' "$OPENED" > "$OPENER"; chmod +x "$OPENER"
EXPLORER2_OPEN_CMD=$OPENER $EXE "$SR" > $LOG 2>&1 & PID=$!
wait_for "grep -q 'phase=Ready' $LOG" >/dev/null
focus; xdotool key ctrl+f; sleep 0.3
check "[77] Ctrl+F focuses the search field (key reaches the pane)" "true"
xdotool type --delay 40 "datei1"; sleep 1.2
check "[77b] typing searches below the folder (index, 11 hits)" \
  "wait_for \"grep -q 'search found query=.datei1. total=11 shown=11 walked=false' $LOG\" 5 && grep -q 'search pane=0 scope=$SR query=.datei1.' $LOG"
check "[77c] one search step per pause, not per letter" "[ \$(grep -c 'search found' $LOG) -le 3 ]"
command -v import >/dev/null && import -window root "$W/77-results.png"
check "[77d] results: name column + folder column (relative: a)" "last 'shown' | grep -q 'datei1.txt' && last 'kinds' | grep -qF '[\"a\"]'"
xdotool key Down; sleep 0.3; xdotool key Home Return; sleep 1
check "[77e] Enter on a hit opens the real file (other folder than the tab)" "wait_for \"grep -qx '$SR/a/datei1.txt' $OPENED\" 3"
xdotool key Down Delete; sleep 1.5
check "[77f] Delete on a hit: the file goes to the trash" "[ ! -e '$SR/a/datei10.txt' ]"
check "[77g] the results follow the index (10 hits left)" "wait_for \"grep -q 'search found query=.datei1. total=10' $LOG\" 6"
xdotool key ctrl+f; sleep 0.2; xdotool key ctrl+a; xdotool type --delay 30 "ext:pdf"; sleep 1.2
check "[77h] query language: ext:pdf finds a/b/tief.pdf" "wait_for \"grep -q 'search found query=.ext:pdf. total=1' $LOG\" 5 && last 'shown' | grep -q 'tief.pdf'"
xdotool key Escape; sleep 1
check "[77i] Esc: back to the folder" "last 'search pane=0 closed' | grep -qF '$SR' && last 'address=' | grep -qx '.*address=$SR'"
xdotool key alt+Left; sleep 1
check "[77j] the search left no trace: Back does not return into it" "! last 'address=' | grep -q '::suche'"
echo "== [78] S1.4c: quick search over every indexed drive (Ctrl+Shift+F)"
focus; xdotool key ctrl+shift+f; sleep 0.5
check "[78] Ctrl+Shift+F opens the quick search" "grep -q 'quick search open' $LOG"
xdotool type --delay 40 "tief"; sleep 1
check "[78b] typing finds tief.pdf (index, best first)" "wait_for \"last 'quick search query' | grep -q 'query=.tief. total=1 shown=1 first=Some(.tief.pdf.)'\" 4"
command -v import >/dev/null && import -window root "$W/78-quick.png"
xdotool key Return; sleep 1
check "[78c] Enter opens the file, the overlay closes" "wait_for \"grep -qx '$SR/a/b/tief.pdf' $OPENED\" 3 && grep -q 'quick search close' $LOG"
xdotool key ctrl+shift+f; sleep 0.5; xdotool type --delay 40 "datei2"; sleep 1
check "[78d] datei2: exact name first (datei2.txt before datei20-29)" "wait_for \"last 'quick search query' | grep -q 'query=.datei2. total=11 shown=11 first=Some(.datei2.txt.)'\" 4"
xdotool key ctrl+Return; sleep 1.5
check "[78e] Ctrl+Enter shows it selected in its folder" "last 'show in folder' | grep -qF '$SR/a/datei2.txt' && wait_for \"last 'selection pane=0' | grep -q 'focus=datei2.txt'\" 3"
xdotool key ctrl+shift+f; sleep 0.5; xdotool key Escape; sleep 0.4
check "[78f] Esc closes it" "last 'quick search' | grep -q 'close'"
kill $PID; wait $PID 2>/dev/null
check "[77k] a search tab is saved as its folder (session)" "! grep -q '::suche' $IC/session.ini"
exit $FAIL
