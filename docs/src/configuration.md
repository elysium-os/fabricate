# Configuration File

Fabricate evaluates a single Lua file (by default `fab.lua`). The configuration is written in Lua (Lua 5.4).
Many of the standard lua functions and libraries are available.

## Install Artifacts

Anywhere in the root scope of `fab.lua` return a table. Fabricate currently reads the
optional `install` field to discover which artifacts should be copied during an
installation step. Destination paths are interpreted relative to the prefix.
The field must be a table mapping destination paths to the
`Artifact` objects produced earlier:

Example:

```lua
return {
    install = {
        ["bin/fabricate-example"] = app_artifact,
        ["lib/libexample.a"] = static_lib,
    }
}
```

If you do not want Fabricate to manage installation simply return a table without the `install` key or omit the return entirely.
