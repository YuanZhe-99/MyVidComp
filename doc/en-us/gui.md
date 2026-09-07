# The application

The application under `gui/` embeds the conversion engine and builds for Windows, macOS, Linux and
Android from one source tree. It drives the engine through the ABI in
[ffi-abi.md](ffi-abi.md) and consumes structured events rather than parsing terminal output.

## Screens

Four, in the order a person meets them:

| Screen | Holds |
|---|---|
| Convert | The folder, the output codec, the quality target, how careful to be, and Start. |
| Progress | What is happening now, the totals, every finished file, and a collapsible technical log. |
| Review | Conversions waiting for a decision, with their scores, sizes and changes. |
| Settings | Everything else, with the rarely-needed parts behind an Advanced section. |

The Review destination carries a badge whenever something is waiting.

## Layout

One layout, three widths, chosen from the window rather than the device:

| Width | Navigation | Options |
|---|---|---|
| under 600 | along the bottom | dropdowns |
| 600 to 1100 | rail down the side, icons and labels | segmented rows where they fit |
| 1100 and above | rail opened out | segmented rows |

Page content is capped at a readable width and centred, so a maximised window does not stretch a
form across the screen. Every list scrolls; nothing is sized so that it can be cut off.

## Wording

The engine's own values, such as `hevc`, `flexible` and `mkv-fallback`, are never shown. Every one
has a label and, where a choice needs explaining, a sentence saying what picking it means. Encoder
names are described rather than printed: `av1_nvenc` reads as an NVIDIA graphics card.

Four languages are translated in full: English, Simplified Chinese, Traditional Chinese and
Japanese. The language picker can follow the device. Tests assert that every choice has a label
distinct from its wire value in every language.

## How it runs

- Conversion runs on a worker isolate, so the interface stays responsive.
- The interface requires ABI version 1 or newer and reports plainly when the engine is missing or
  mismatched, rather than crashing while it starts.
- Stopping is graceful: the running ffmpeg is never interrupted, and the run ends after the file it
  is on.
- The cancellation token is freed on every path, including when the isolate is killed.
- Every `ffmpeg` and `ffprobe` process the engine starts goes through the shared constructor, so
  Windows never flashes a console window. The interface must not reintroduce a direct `-version`
  check for the same reason: on Windows it checks the file exists instead of running it.

## Settings

- Stored in the user's profile as `settings.json`, under `MyVidComp` on Windows and macOS and
  `myvidcomp` elsewhere. Never in `config.yaml`, which belongs to the user.
- A settings file written by the previous version is read once, so an existing install keeps its
  folder, tool paths and language. The retired conversion-mode value is mapped onto the encoder
  preference.
- An unknown or out-of-range stored value falls back to the default rather than reaching the
  engine.
- The media tools are looked for beside the application, in a sibling `bin/` directory, in the
  working directory and on `PATH`. Paths given in Settings override that.

## Progress

Two bars: the file being worked on, and the whole run. Both are determinate. Every phase a file
passes through reports where it has got to, including choosing a quality setting and measuring the
result, so only the folder scan is ever left indeterminate. Neither bar moves backwards, and they
are never left in an unknown state while idle.

The run bar counts every file the run is finished with, skipped ones included, so a second pass over
a folder that is already converted still reaches the end.

Beside the file bar are the phase, the speed FFmpeg reports and the time left, and each line of the
details log carries how serious it is.

## Identity

The Windows executable is `MyVidComp.exe`, the window title and product metadata are `MyVidComp`,
and the icon is generated from one source image for every platform.

## Build

Windows builds compile the engine through CMake and install `myvidcomp_core.dll` beside the
executable. The media tools are bundled separately; see [packaging.md](packaging.md). The Android
build is covered in [android.md](android.md).
