# templates

`tuning` interprets the main.toml file as a
[tera](https://github.com/Keats/tera) template

## facts

the following `tuning`-specific values are available,
for use within template expressions

see the [`Facts`](../src/lib/facts.rs) struct for low-level details

### cache_dir (path)

as defined over in the [dirs crate](https://crates.io/crates/dirs)

e.g. ~/.cache (Linux)

### config_dir (path)

as defined over in the [dirs crate](https://crates.io/crates/dirs)

e.g. ~/.config (Linux)

### has_executable (exe:string -> boolean)

`true` if a given executable is available (i.e. in the PATH)

e.g. `{{ has_executable(exe="tuning") }}`

### home_dir (path)

as defined over in the [dirs crate](https://crates.io/crates/dirs)

e.g. ~/ (Linux)

### is_os_linux (boolean)

`true` if OS is Linux

### is_os_macos (boolean)

`true` if OS is macOS

### is_os_windows (boolean)

`true` if OS is Windows
