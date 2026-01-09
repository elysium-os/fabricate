# Introduction

Fabricate is a Lua-configured build system that produces Ninja build files. The
`fabricate` CLI evaluates your `fab.lua` configuration, downloads declared git
dependencies, records build graph information, and writes an `output/build.ninja`
file (and optionally a `compile_commands.json`). After `fabricate setup` runs you
invoke Ninja to actually build your project.

This guide documents how to _use_ Fabricate: the CLI options, how to structure a
`fab.lua`, and the helper functions and object types that Fabricate makes
available to Lua. Development internals, contributing guidelines, and other
non-user topics are intentionally omitted.

# Getting Started

1. Install the `fabricate` binary (Cargo `cargo install fabricate2` if you are
   building from source, or copy the compiled binary into your `$PATH`).
2. Create a working directory that contains a `fab.lua` configuration file. All
   paths referenced by the configuration are interpreted relative to this
   directory.
3. Run `fabricate setup` to generate the build directory. This will create the
   Ninja build file, compile commands, a fabricate cache file, ...
4. Build either with Ninja directly or using Fabricates wrapper. The default build directory is `build`.  
   4.1. with Ninja: `ninja -C <build dir>`.  
   4.2. with Fabricate: `fabricate --build-dir <build dir>`.
5. Finally install artifacts with `fabricate install`.
