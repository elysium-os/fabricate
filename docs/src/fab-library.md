# Fab Library

The global `fab` table exposes functions implemented by Fabricate. These
functions are the authorative way of interacting with Fabricate but are
also very crude. This is why Fabricate provides many helpers written in
Lua for a more user friendly interface.

## `fab.glob(pattern, opts?)`

Runs a glob relative to the project root and returns an array of matching path strings. `opts` is optional and may contain:

| Field                         | Type       | Description                                                                                                      |
| ----------------------------- | ---------- | ---------------------------------------------------------------------------------------------------------------- |
| `case_sensitive`              | `boolean`  | Override the default case-sensitive behavior.                                                                    |
| `require_literal_separator`   | `boolean`  | If true, `*` and `?` will never match `s/`. This is false by default.                                            |
| `require_literal_leading_dot` | `boolean`  | If true, wildcards will not match files that start with `.`. This is false by default.                           |
| `excludes`                    | `string[]` | Additional glob patterns that are applied after the main glob; any match here is removed from the returned list. |

```lua
-- collect all C sources outside the tests directory
local c_files = fab.glob("src/**/*.c", {
    case_sensitive = false,
    excludes = { "src/tests/**" }
})
```

## `fab.project_dir()`

Returns an absolute path to the project root.

## `fab.build_dir()`

Returns an absolute path to the build directory.

## `fab.path_join(...)`

Joins the provided path fragments using the host platform’s separator and
returns the joined string. If a component is absolute, it replaces the entire path.

## `fab.path_rel(path)`

Resolves a build directory relative path from an absolute path or a project root relative path.

## `fab.which(name)`

Find an executable binary’s path by name. Returns an `Executable` userdata when the binary is found or `nil` otherwise.

- If given an absolute path, returns it if the file exists and is executable.
- If given a relative path, returns an absolute path to the file if it exists and is executable.
- If given a string without path separators, looks for a file named binary_name at each directory in `$PATH` and if it finds an executable file there, returns it.

## `fab.option(name, type, required)`

Declares a user option that can be provided on the CLI via
`--option name=value`. The `type` argument controls validation:

- `"string"`, `"number"`, or `"boolean"` expect the corresponding type and
  transform the CLI string automatically.
- A table of allowed strings works as an enum (Fabricate checks that the CLI
  value matches one of the table entries and returns the matching value).

If `required` is false or omitted the option may be omitted and `nil` is returned. Otherwise
Fabricate raises a setup-time error if the option is missing.

```lua
local selected_cc = fab.option("toolchain", { "gcc", "clang" })
```

## `fab.git(name, url, revision)`

Clones (or reuses) a git repository into the build directory and returns an
`Artifact` pointing at the repository directory. The clone is skipped when the
cache already contains a matching URL and revision.

## `fab.def_source(path)`

Declares a source file relative to the project root, returning a `Source`.
Fabricate validates that the path stays inside the source tree. Note that
`Source`s must exist at setup-time whereas `Artifact`s might not.

## `fab.def_rule(name, command, description?, depstyle?, build_compdb?)`

Creates a rule object. Arguments:

- `name`: must be unique, contain only alphanumeric characters plus `_` or `-`,
  and must not start with `fab_`.
- `command`: shell command template. Allows for "embed variables", the embeds take the following form: @EMBED@.
  The names of the embeds are case-insensitive. They are replaced by values passed at each invocation of a rule build.
  Fabricate supports a few special embeds:
  | Name | Description |
  | ----- | ----------- |
  | `@IN@` | Source file path(s) |
  | `@OUT@` | Output file path |
  | `@DEPFILE@` | Dependency file path |
- `description`: optional description displayed by Ninja at build time.
- `depstyle`: one of `"normal"`, `"gcc"`, `"clang"`, or `"msvc"` and controls
  how dependency files are interpreted. If unsure, set to `"normal"`.
- `build_compdb`: set to `true` to include builds using this rule when generating
  `compile_commands.json`.

The returned `Rule` object exposes the `rule:build(...)` method documented in the
Rules chapter.

## `fab.typeof(userdata)`

A helper that inspects an arbitrary userdata value and returns
`"executable"`, `"source"`, `"rule"`, `"artifact"`, or `"unknown"`.
