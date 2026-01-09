# Command-Line Interface

The `fabricate` binary exposes three subcommands: `setup`, `build`, and
`install`. All commands share the `--build-dir` (`-b`) flag that chooses which
build directory to operate on. If omitted, the build directory defaults to
`build`.

```
fabricate [GLOBAL OPTIONS] <SUBCOMMAND> [OPTIONS]
```

| Global flag                | Default | Description                                                                     |
| -------------------------- | ------- | ------------------------------------------------------------------------------- |
| `-b`, `--build-dir <path>` | `build` | Directory that stores `build.ninja`, cached metadata, and intermediate outputs. |
| `-h`, `--help`             | –       | Show help for the selected command.                                             |
| `-V`, `--version`          | –       | Show the Fabricate version.                                                     |

## `setup`

Evaluates the Lua configuration and writes/updates `build.ninja`.

| Flag                              | Default (per code) | Description                                                                                                                                                       |
| --------------------------------- | ------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `--config <path>`                 | `fab.lua`          | Lua configuration file to execute.                                                                                                                                |
| `--prefix <path>`                 | `fab.lua`          | Installation prefix recorded in `fabricate_cache.toml`. (The help text mentions `/usr`, but the current build sets the default to `fab.lua`.)                     |
| `-o`, `--option key=value`        | –                  | Collects user-defined options that Lua can read via `fab.option`. Repeat the flag for each key/value pair.                                                        |
| `--dependency-override name=path` | –                  | Overrides the git dependency declared via `fab.git(name, …)` to use an existing checkout at `path` instead of cloning into the build directory. Repeat as needed. |

Example:

```sh
fabricate setup \
    --config fab.lua \
    --prefix /usr \
    --build-dir build \
    --option toolchain=clang \
    --option enable-tests=yes
```

Dependency overrides let you substitute local checkouts for remote git dependencies during `setup`. Each override uses the dependency name (the first argument passed to `fab.git`) and either an absolute path or a path relative to the directory that contains `fab.lua`. When present, Fabricate records the dependency metadata but returns the provided path to Lua, so rules can consume your locally modified sources without triggering network fetches.

If Ninja is installed, `setup` also invokes `ninja -t cleandead` inside the
existing build directory before rewriting the graph.

## `build`

Runs Ninja in the selected build directory. This is identical to running `ninja -C <build-dir>`.

## `install`

Copies all artifacts listed in the `install` map.
This subcommand fails if the cache is missing, or if any artifact is absent.
Note that Fabricate does not allow for installation of directory artifacts.

| Flag                | Default        | Description                                                          |
| ------------------- | -------------- | -------------------------------------------------------------------- |
| `--dest-dir <path>` | (empty string) | Optional DESTDIR-style prefix prepended to each install destination. |

Example:

```sh
fabricate --build-dir build install --dest-dir /tmp/sysroot
```

Install computes each destination as `DESTDIR + prefix + dest path` where `prefix`
comes from the last `setup` invocation and `dest path` is the install map key (such as
`bin/foo`). Before copying a file Fabricate creates the parent directories.
