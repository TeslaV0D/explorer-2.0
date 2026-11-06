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
kill $PID; wait $PID 2>/dev/null
exit $FAIL
