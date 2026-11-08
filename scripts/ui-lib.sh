# UI targets by name instead of pixels (redesign R0). Source it:
#   . "$(dirname "$0")/ui-lib.sh"
# The app must run with EXPLORER2_UI_DUMP=$UI_DUMP (exported here); it writes
# every named element (accessible-label, Text content) with its position.
#
#   ui_xy "Einstellungen"          -> "x y" (center, screen pixels)
#   ui_xy "Größe" last             -> the topmost one (drawn last), e.g. a menu over the header
#   ui_xy "Zurück" 2               -> the 2nd one (e.g. pane 1)
#   ui_click "Einstellungen" [n]   -> moves there and clicks (fails loudly if not found)
#   ui_wait "Dunkel" [secs]        -> waits until the element is shown
#   ui_has "Dunkel"                -> true if shown now
#   ui_box "Neuer Ordner"          -> "x y w h" (e.g. for pixel checks inside an element)
#   ui_click_after "Adresszeile"   -> clicks the first element inside/after it (first breadcrumb)
#   ui_list_top                    -> y of the first row of the (first) file list
#   ui_action "Neuer Ordner"       -> file action of the active pane (button, or ⋮ menu when narrow)
#
# Linux/Xvfb without a window manager: the window sits at the screen origin.
UI_DUMP=${UI_DUMP:-${W:-/tmp}/ui-dump.txt}
export EXPLORER2_UI_DUMP=$UI_DUMP

_ui_line() { # label which -> "x y w h" of the chosen element
  awk -F'\t' -v L="$1" -v N="${2:-1}" '
    NR==1 { split($0, w, " "); wx=w[2]; wy=w[3]; s=w[4]; next }
    $1==L { n++; x[n]=$2; y[n]=$3; ww[n]=$4; hh[n]=$5 }
    END {
      if (n == 0) exit 1
      k = (N == "last") ? n : N + 0
      if (k < 1 || k > n) exit 1
      printf "%d %d %d %d\n", wx + x[k]*s, wy + y[k]*s, ww[k]*s, hh[k]*s
    }' "$UI_DUMP" 2>/dev/null
}

ui_xy() {
  local r; r=$(_ui_line "$1" "${2:-1}") || return 1
  set -- $r
  echo "$(( $1 + $3 / 2 )) $(( $2 + $4 / 2 ))"
}

ui_box() { # label [which] -> "x y w h" (screen pixels)
  _ui_line "$1" "${2:-1}"
}

ui_has() { _ui_line "$1" "${2:-1}" >/dev/null; }

ui_wait() { # label [secs] [which]
  local i n=$(( ${2:-5} * 10 ))
  for i in $(seq 1 $n); do ui_has "$1" "${3:-1}" && return 0; sleep 0.1; done
  return 1
}

ui_click() { # label [which] [button]
  local xy=""; ui_wait "$1" 3 "${2:-1}" && xy=$(ui_xy "$1" "${2:-1}")
  if [ -z "$xy" ]; then echo "UI   target not found: '$1' (${2:-1})"; return 1; fi
  xdotool mousemove $xy click "${3:-1}"
}

ui_click_after() { # label -> clicks the element drawn right after it (e.g. the first breadcrumb)
  local xy=""
  xy=$(awk -F'\t' -v L="$1" '
    NR==1 { split($0, w, " "); wx=w[2]; wy=w[3]; s=w[4]; next }
    hit { printf "%d %d\n", wx + ($2 + $4/2)*s, wy + ($3 + $5/2)*s; exit }
    $1==L { hit=1 }' "$UI_DUMP" 2>/dev/null)
  if [ -z "$xy" ]; then echo "UI   nothing after: '$1'"; return 1; fi
  xdotool mousemove $xy click 1
}

ui_action() { # label -> a file action of the active pane: its button, or (narrow pane) the ⋮ menu
  if ui_has "$1"; then ui_click "$1"; return; fi
  ui_click "Mehr" || return 1
  sleep 0.5; ui_click "$1" last
}

ui_list_top() { # top of the file list = bottom of the "Name" column header (first pane)
  local r; r=$(_ui_line "Name" 1) || { echo 140; return; }
  set -- $r
  echo $(( $2 + $4 ))
}
