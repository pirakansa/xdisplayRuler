# Output Modes

Use output mode commands to list modes reported by RandR and switch an output to
one of those existing modes.

These commands are transitional in `xdisplay-ruler`. They still run during the
migration window, but each invocation prints a warning to standard error because
display pipeline control is moving to `xdisplay-attach`.

## List Output Modes

```bash
xdisplay-ruler modes --output HDMI-2
```

The current mode is marked with `current`, and preferred modes are marked with
`preferred`.

## Switch Mode

Switch the output to one of the listed modes:

```bash
xdisplay-ruler mode --output HDMI-2 --width 1280 --height 720
```

When multiple modes share the same size, select a refresh rate:

```bash
xdisplay-ruler mode --output HDMI-2 --width 1920 --height 1080 --rate 60
```

`mode` selects from modes already reported by the Xorg RandR extension for the
output. It does not create custom modelines.

## Rotate Output

Rotate an output while keeping its current mode:

```bash
xdisplay-ruler mode --output HDMI-2 --rotate left
```

Change mode and rotation together:

```bash
xdisplay-ruler mode --output HDMI-2 --width 1920 --height 1080 --rate 60 --rotate inverted
```

For left or right rotation, pass the displayed size after rotation:

```bash
xdisplay-ruler mode --output HDMI-2 --width 1080 --height 1920 --rate 60 --rotate left
```

`--rotate` accepts `normal`, `left`, `right`, or `inverted`.

For exact option requirements and rotation behavior, see the
[CLI specification](../specifications/cli.md).
