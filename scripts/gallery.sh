#!/bin/bash
# UI gallery (redesign R0): screenshots of every main screen, light and dark,
# for "before/after" comparisons. Linux, Xvfb, no window manager.
#   xvfb-run -a -s "-screen 0 1280x800x24" scripts/gallery.sh <exe> <outdir>
# Files: <outdir>/<theme>-<nn>-<screen>.png (+ index.txt with what was shown).
set -u
EXE=$1; OUT=$2; W=$OUT/work
mkdir -p "$OUT" "$W"
. "$(dirname "$0")/ui-lib.sh"
command -v import >/dev/null || { echo "imagemagick (import) missing"; exit 1; }

# a folder that looks like real use
D=$W/Projekte
mkdir -p "$D/Assets/Icons" "$D/Dokumente" "$D/Quellcode/src" "$D/Archiv 2025"
for f in README.md package.json Cargo.toml notizen.txt "Bericht Q3 2026.pdf" "Budget 2027.xlsx" \
         "Präsentation.pptx" setup.exe daten.csv .gitignore; do echo "inhalt $f" > "$D/$f"; done
head -c 2400000 /dev/zero > "$D/video-schnitt.mp4"
convert -size 640x400 gradient:'#3a7bd5-#00d2ff' "$D/Titelbild.png" 2>/dev/null
convert -size 400x400 plasma:fractal "$D/Assets/foto.jpg" 2>/dev/null
for i in $(seq 1 30); do echo $i > "$D/Quellcode/src/modul_$i.rs"; done

shot() { # name: hides the mouse in a corner first
  xdotool mousemove 1279 799; sleep 0.4
  import -window root -crop 1200x720+0+0 "$OUT/$THEME-$1.png"
  echo "$THEME-$1" >> "$OUT/index.txt"
}
focus() { xdotool windowfocus --sync "$(xdotool search --name 'Explorer 2.0' | head -1)"; }

run_theme() {
  THEME=$1
  local cfg=$W/cfg-$THEME
  rm -rf "$cfg"; mkdir -p "$cfg"
  printf 'theme = %s\nsystem_accent = false\n' "$THEME" > "$cfg/settings.ini"
  EXPLORER2_CONFIG_DIR=$cfg EXPLORER2_TRACE=1 SLINT_BACKEND=winit-software HOME=$W/home \
    EXPLORER2_INDEX=on EXPLORER2_INDEX_ROOTS=$D EXPLORER2_INDEX_DELAY_MS=200 \
    $EXE "$D" > "$W/trace-$THEME.log" 2>&1 & PID=$!
  ui_wait "README.md" 10 || echo "WARN app did not show the folder"
  sleep 1; focus
  shot 01-ordner
  xdotool mousemove $(ui_xy "Titelbild.png") click 1; sleep 0.3
  xdotool key alt+p; sleep 1.2; shot 02-inspector-bild
  xdotool key alt+p; sleep 0.4
  xdotool mousemove $(ui_xy "README.md") click 1; xdotool key shift+Down shift+Down; sleep 0.3
  shot 03-mehrfachauswahl
  xdotool key ctrl+backslash; sleep 1.2; shot 04-zwei-ansichten
  xdotool key ctrl+backslash; sleep 0.8; focus
  xdotool key ctrl+shift+2; sleep 1.5; shot 05-kacheln
  xdotool key ctrl+shift+6; sleep 0.8
  xdotool key ctrl+f; sleep 0.2; xdotool type --delay 20 "modul_1"; sleep 1.5; shot 06-suche
  xdotool key Escape; sleep 0.8
  xdotool key ctrl+shift+f; sleep 0.4; xdotool type --delay 20 "bericht"; sleep 1.2; shot 07-schnellsuche
  xdotool key Escape; sleep 0.4
  xdotool key ctrl+shift+p; sleep 0.5; shot 08-befehlspalette
  xdotool key Escape; sleep 0.4
  ui_click "Einstellungen"; sleep 1; shot 09-einstellungen
  ui_click "Schließen" last; sleep 0.4
  xdotool mousemove $(ui_xy "notizen.txt") click 1; xdotool key shift+Delete; sleep 0.8; shot 10-bestaetigung
  xdotool key Escape; sleep 0.4
  WID=$(xdotool search --name 'Explorer 2.0' | head -1)
  xdotool windowsize "$WID" 820 560; sleep 1.2; shot 11-schmal
  kill $PID; wait $PID 2>/dev/null
  # the design system (R1): colour roles, controls, icons
  EXPLORER2_CONFIG_DIR=$cfg EXPLORER2_UI_KIT=1 SLINT_BACKEND=winit-software HOME=$W/home EXPLORER2_INDEX=off \
    $EXE --new-instance "$D" > "$W/trace-kit-$THEME.log" 2>&1 & PID=$!
  ui_wait "UI-Kit schließen" 10 || echo "WARN UI kit not shown"
  sleep 0.5; shot 12-ui-kit
  kill $PID; wait $PID 2>/dev/null
}

: > "$OUT/index.txt"
run_theme light
run_theme dark
echo "gallery: $(wc -l < "$OUT/index.txt") screenshots in $OUT"
