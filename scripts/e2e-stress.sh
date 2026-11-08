#!/bin/bash
# M8.3c endurance (Linux, Xvfb, no window manager): many navigations, tabs
# opened/closed, view switches and file churn in the watched folder. Memory
# (RSS), open files and threads must stay flat after the warm-up.
# Called by e2e.sh, runnable alone:
#   xvfb-run -a -s "-screen 0 1280x800x24" scripts/e2e-stress.sh <exe> <workdir> [rounds]
set -u
EXE=$1; W=$2; N=${3:-150}; FAIL=0
pass() { echo "PASS  $1"; }
fail() { echo "FAIL  $1"; FAIL=1; }
check() { if eval "$2"; then pass "$1"; else fail "$1"; fi; }

export EXPLORER2_TRACE=1 SLINT_BACKEND=winit-software EXPLORER2_CONFIG_DIR=$W/config-stress HOME=$W/home-stress
mkdir -p "$HOME"
focus() { xdotool windowfocus --sync "$(xdotool search --name 'Explorer 2.0' | head -1)"; }

echo "== [75] M8.3c endurance: $N rounds (navigate, tab open/close, view switch, file churn)"
SD=$W/stress-$$; mkdir -p "$SD"
for i in $(seq -w 1 20); do
  mkdir -p "$SD/d$i"
  for j in $(seq 1 120); do : > "$SD/d$i/f$j.e$((j % 9))"; done
done
LOG=$W/trace-75.log
$EXE "$SD" > $LOG 2>&1 & PID=$!
sleep 2.5; focus
# RSS (KiB), open files, threads
stats() { echo "$(awk '/VmRSS/{print $2}' /proc/$PID/status) $(ls /proc/$PID/fd | wc -l) $(ls /proc/$PID/task | wc -l)"; }
cycle() {
  # 20 folders, then files: back to the top before Down passes the last folder
  [ $(( $1 % 19 )) -eq 0 ] && xdotool key Home
  xdotool key Down Return; sleep 0.12
  if [ $(( $1 % 5 )) -eq 0 ]; then xdotool key ctrl+shift+6; sleep 0.1; xdotool key ctrl+shift+1; fi
  if [ $(( $1 % 7 )) -eq 0 ]; then : > "$SD/churn-$1"; rm -f "$SD/churn-$(( $1 - 7 ))"; fi
  xdotool key BackSpace; sleep 0.12
  xdotool key ctrl+t; sleep 0.05; xdotool key ctrl+w; sleep 0.05
}
for i in $(seq 1 30); do cycle $i; done
sleep 1.5
read -r R1 F1 T1 <<< "$(stats)"
NAV1=$(grep -c 'navigate tab=' $LOG)
T0=$(date +%s)
for i in $(seq 31 $(( 30 + N ))); do cycle $i; done
sleep 2
read -r R2 F2 T2 <<< "$(stats)"
NAV2=$(grep -c 'navigate tab=' $LOG)
SECS=$(( $(date +%s) - T0 ))
echo "    after warm-up: rss=${R1} KiB fds=$F1 threads=$T1 | after $N more rounds (${SECS}s): rss=${R2} KiB fds=$F2 threads=$T2 | navigations $NAV1 -> $NAV2"
check "[75] the rounds really ran ($(( NAV2 - NAV1 )) navigations)" "[ $(( NAV2 - NAV1 )) -ge $N ]"
check "[75b] memory stays flat (+$(( (R2 - R1) / 1024 )) MiB, limit +24 MiB)" "[ $(( R2 - R1 )) -lt 24576 ]"
check "[75c] no leaked files/handles (+$(( F2 - F1 )), limit +3)" "[ $(( F2 - F1 )) -le 3 ]"
check "[75d] no leaked threads (+$(( T2 - T1 )), limit +2)" "[ $(( T2 - T1 )) -le 2 ]"
check "[75e] process alive, no panic" "kill -0 $PID 2>/dev/null && ! grep -qi 'panic' $LOG"
kill $PID; wait $PID 2>/dev/null
exit $FAIL
