# MyVidComp application

The application that embeds the conversion engine. One source tree builds for Windows, macOS,
Linux and Android.

It loads the engine as a shared library and calls it directly, rather than launching the
command-line tool as a separate program. Events come back as JSON and the interface renders them;
nothing parses terminal output. The full description is in
[doc/en-us/gui.md](../doc/en-us/gui.md).

## Run it

```sh
flutter run -d windows
flutter run -d <your phone>
```

The engine has to exist first. On Windows the build compiles it through CMake; on Android the
Gradle build calls `cargo ndk`. Elsewhere, `cargo build --release` and put the library beside the
executable.

## Check it

```sh
dart format --set-exit-if-changed lib test
flutter analyze
flutter test
```

The tests cover the wording rules (no setting may show the value the engine uses internally, in any
of the four languages), settings storage including files written by the previous version, and the
layout at phone, tablet and desktop widths.

## Package it

```powershell
../scripts/package-gui-windows-all.ps1 -DownloadFfmpeg   # both Windows packages
../scripts/package-android.ps1 -BuildFfmpeg               # the Android package
```

See [doc/en-us/packaging.md](../doc/en-us/packaging.md) and
[doc/en-us/android.md](../doc/en-us/android.md).

## Icons

Every platform's icon is generated from `assets/icon/app_icon.png`:

```sh
dart run flutter_launcher_icons
```
