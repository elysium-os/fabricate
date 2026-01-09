# Object Reference

Fabricate injects several userdata types into Lua. These behave like opaque objects
with fields/methods. Fabricate uses them to collect build graph information.

## Source

Represents an input file inside the source directory.

| Field      | Type     | Description                            |
| ---------- | -------- | -------------------------------------- |
| `rel_path` | `string` | Path relative to the source directory. |
| `abs_path` | `string` | Absolute path pointing at the source.  |

## Artifact

Represents a build artifact produced during at build time.

| Field      | Type     | Description                             |
| ---------- | -------- | --------------------------------------- |
| `rel_path` | `string` | Path relative to the build directory.   |
| `abs_path` | `string` | Absolute path pointing at the artifact. |

## Rule

| Field  | Type     | Description       |
| ------ | -------- | ----------------- |
| `name` | `string` | Name of the rule. |

A rule defines how to run a command. Besides the `name` field, the important API
is `rule:build(output, inputs, variables, implicit_inputs?)`:

- `output`: Relative path (relative to the build directory) describing the logical output.
  This ends up as the special `@OUT@` variable.
- `inputs`: Array of `Source` or `Artifact` objects that the build depends on.
  These also end up in the special `@IN@` variable.
- `variables`: Table containing custom `@VAR@` values declared when the rule was
  defined.
- `implicit_inputs`: Optional additional sources/artifacts that should be wired
  as implicit dependencies (dependend on but not directly used).

The method returns an `Artifact` describing the produced file.

## Executable

Returned by `fab.which`. Available fields:

| Field  | Type     | Description                           |
| ------ | -------- | ------------------------------------- |
| `name` | `string` | Basename of the executable.           |
| `path` | `string` | Absolute path to the executable file. |

Executables expose `exec:invoke(arg1, arg2, ...)`, a convenience wrapper that
runs the program immediately during configuration, returns captured `stdout`, and
propagates any non-zero exit status as a Lua error.
