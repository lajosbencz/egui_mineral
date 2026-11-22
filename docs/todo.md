# TODO

Core APIs kept but not exercised by any layer: none.

## Deferred requests

Larger features requested by users, not scheduled yet.

- `TextField`: show the end of an overflowing value while unfocused (path file names).
- Locale: translations of the library strings beyond en-US.
- Density for the `DatePicker`/`DateRangePicker` day grid and the `TimePicker` dial: their 48dp cells are baked into constants and scroll math.
- RTL tests for the Fluent extras (SplitPane, StatusBar, Pagination, CommandPalette, MenuBar) and the Fluent SpinButton unit suffix.

## CI

- macOS clippy: a `macos-latest` job for the core `cfg(target_os = "macos")` arms. Cross-target clippy from Linux fails: the `mac-notification-sys` build script needs the Apple SDK.
