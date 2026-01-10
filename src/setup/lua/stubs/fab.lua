--- @meta

--- @class Fab
fab = {}

--- Matches a glob against the project root, returns the remaining matches.
--- @param pattern string
--- @param options { case_sensitive: boolean, require_literal_separator: boolean, require_literal_leading_dot: boolean, excludes: string[] }?
function fab.glob(pattern, options) end

--- Join path components together.
--- @vararg string
--- @return string
function fab.path_join(...) end

--- Get an absolute path to the project root.
--- @return string
function fab.project_dir() end

--- Get an absolute path to the build directory.
--- @return string
function fab.build_dir() end

--- Retrieve the fab type of userdata.
--- @param value userdata
--- @return "unknown" | "source" | "rule" | "executable"
function fab.typeof(value) end

--- Find an executable binary’s path by name.
--- If given an absolute path, returns it if the file exists and is executable.
--- If given a relative path, returns an absolute path to the file if it exists and is executable.
--- If given a string without path separators, looks for a file named binary_name at each directory in $PATH and if it finds an executable file there, returns it.
--- @param lookup string
--- @return Executable?
function fab.which(lookup) end

--- Clones a git repository into the build directory.
--- @param name string
--- @param url string
--- @param revision string
--- @return Artifact
function fab.git(name, url, revision) end

--- Declare an option that can be passed by the user to fabricate.
--- @param name string
--- @param type "string" | "number" | "boolean" | string[]
--- @param required boolean?
--- @return string | number | boolean | nil
function fab.option(name, type, required) end

--- Define a [Source](lua://Source).
--- @param path string
--- @return Source
function fab.def_source(path) end

--- Define a [Rule](lua://Rule).
--- @param name string
--- @param command string
--- @param description string?
--- @param depstyle ("normal" | "gcc" | "clang" | "msvc")?
--- @param compdb boolean?
--- @return Rule
function fab.def_rule(name, command, description, depstyle, compdb) end
