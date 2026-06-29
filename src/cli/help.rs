pub(super) const HELP: &str = "\
xdisplay-ruler

Overview:
  Inspect Xorg display state, list or change RandR output modes, and send
  low-level X11 requests to move, resize, raise, lower, or place windows.

Quick Start:
  xdisplay-ruler
  xdisplay-ruler snapshot --backend x11
  xdisplay-ruler raise --window-class Gnome-terminal

Usage:
  Snapshot:
    xdisplay-ruler [snapshot] [--backend NAME]
    xdisplay-ruler watch [--backend NAME] [--iterations N]

  Output Modes:
    xdisplay-ruler modes --output NAME [--backend x11]
    xdisplay-ruler mode --output NAME [--width N --height N] [--rate HZ] [--rotate DIR] [--backend x11]

  Window Control:
    xdisplay-ruler enforce --layout FILE [--once] [--dry-run] [--interval MS] [--backend x11]
    xdisplay-ruler raise WINDOW_SELECTOR [--backend x11]
    xdisplay-ruler lower WINDOW_SELECTOR [--backend x11]
    xdisplay-ruler configure WINDOW_SELECTOR [--x N] [--y N] [--width N] [--height N] [--backend x11]
    xdisplay-ruler place WINDOW_SELECTOR --output NAME --fullscreen [--backend x11]

  Other:
    xdisplay-ruler --help
    xdisplay-ruler --version

Window Selector:
  Use exactly one selector with raise, lower, configure, or place.
    --window ID             X11 window ID, for example 0x800003.
    --window-title NAME     Exact X11 window title.
    --window-class NAME     Exact WM_CLASS class name.
    --window-instance NAME  Exact WM_CLASS instance name.

Commands:
  snapshot  Print one display-state snapshot. This is the default command.
  watch     Keep refreshing and printing display-state snapshots.
  modes     List available modes for an output.
  mode      Change an output mode.
  enforce   Keep layout-defined windows fitted to their output.
  place     Place a window on an output.
  configure Move or resize a window.
  raise     Raise a window above its siblings.
  lower     Lower a window below its siblings.

Global Options:
  --backend NAME      Backend to use. Supported: x11, xorg, in-memory.
  --iterations N      Stop watch after N snapshots. Must be positive.
  --layout FILE       Layout JSON file for enforce.
  --once              Apply enforce once and exit.
  --dry-run           Print the enforce plan without X11 changes.
  --interval MS       Enforce reapply interval in milliseconds. Must be positive.

Output Options:
  --output NAME       X11 RandR output name, for example HDMI-2.
  --rate HZ           Refresh rate for mode, for example 60 or 59.94.
  --rotate DIR        Output rotation: normal, left, right, or inverted.

Window Options:
  --fullscreen        Resize and move the selected window to fill the output.

Geometry Options:
  --x N               Window X position for configure.
  --y N               Window Y position for configure.
  --width N           Window width for configure. Must be positive.
  --height N          Window height for configure. Must be positive.

Notes:
  mode requires --output and either --width with --height or --rotate.
  --rate is optional when --width and --height are provided.
  enforce requires --layout. Without --once or --dry-run, it keeps running.
  place requires WINDOW_SELECTOR, --output, and --fullscreen.
  configure requires WINDOW_SELECTOR and at least one geometry option.
  Window selector name matches are exact and must identify one mapped window.

Examples:
  xdisplay-ruler modes --output HDMI-2
  xdisplay-ruler mode --output HDMI-2 --width 1920 --height 1080 --rate 60
  xdisplay-ruler mode --output HDMI-2 --rotate left
  xdisplay-ruler raise --window-class Gnome-terminal
  xdisplay-ruler lower --window 0x800003
  xdisplay-ruler configure --window-class Gnome-terminal --x 0 --y 0
  xdisplay-ruler place --window-class Gnome-terminal --output HDMI-2 --fullscreen
  xdisplay-ruler enforce --layout layout.json --once --dry-run
";
