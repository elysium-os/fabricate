---@meta

---@alias BuildInput Source|Artifact
---@alias BuildVariables table<string, string>

---@class GlobOptions
---@field case_sensitive? boolean
---@field require_literal_separator? boolean

---@class Rule : userdata
---@field name string
local Rule = {}

--- Registers a build using this rule and returns a handle to its output.
--- `output` must be a relative path and must not collide with an existing build output.
---@param output string
---@param input BuildInput[]
---@param variables BuildVariables
---@param implicit_inputs? BuildInput[]
---@return Artifact
function Rule:build(output, input, variables, implicit_inputs) end

---@class Path : userdata
---@operator div(string): Path
local Path = {}

--- Joins a path onto this path.
---@param other string
---@return Path
function Path:join(other) end

--- Globs for paths under this path.
--- Patterns are matched against paths relative to this path; a pattern
--- prefixed with `!` excludes matches. The final argument may be a
--- `GlobOptions` table.
---@param ... string|GlobOptions
---@return Path[]
function Path:glob(...) end

---@class Source : userdata
---@field path string Path of source (relative to build directory)

---@class Artifact : userdata
---@field path string Path of artifact (relative to build directory)

---@class Binary : userdata
---@field path string Absolute path to the binary

---@alias RuleDepStyle
---| '"normal"'
---| '"gcc"'
---| '"msvc"'
---| string # `"msvc:<prefix>"`

---@alias FabOptionType
---| '"string"'
---| '"number"'
---| '"boolean"'
---| string[] # a closed set of valid string values

--- Table that exposes the Fab API.
---@class FabAPI
---@field project_dir Path
fab = {}

--- Joins two paths together in a platform correct way.
---@param a string
---@param b string
---@return string
function fab.path_join(a, b) end

--- Looks up a binary on `PATH`. Returns `nil` if it cannot be found.
---@param lookup string
---@return Binary?
function fab.which(lookup) end

--- Resolves a declared dependency by name to its root path.
--- Errors if no dependency with that name exists.
---@param name string
---@return Path
function fab.dependency(name) end

--- Resolves a path to a `Source`. Errors if the path does not exist.
---@param path Path
---@return Source
function fab.resolve_source(path) end

--- Defines a new build rule and returns a handle to it.
--- Errors if a rule with the same name is already defined.
--- **NOTE**: Occurrences of `@name@` in `command` and `description` are
--- substituted with build-specific variables when the rule is used.
---@param name string
---@param command string
---@param description? string
---@param depstyle? RuleDepStyle
---@param build_compdb? boolean
---@return Rule
function fab.define_rule(name, command, description, depstyle, build_compdb) end

--- Reads a user supplied option, converting it to `option_type`.
--- Errors if `required` is true and the option was not supplied, or if the
--- supplied value cannot be converted to `option_type`.
---@param name string
---@param option_type FabOptionType
---@param required? boolean
---@return string|number|boolean|nil
function fab.option(name, option_type, required) end
