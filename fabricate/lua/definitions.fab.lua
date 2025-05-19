--- @meta

--- @class Fab
fab = {}

--- Find files in the project directory using a glob pattern.
--- @param pattern string
--- @vararg string Patterns to ignore.
--- @return string[] found
function fab.glob(pattern, ...) end

--- Join path components together.
--- @vararg string
--- @return string
function fab.path_join(...) end

--- Make a relative path absolute based on the current directory or return the original if it is absolute.
--- @param path string
--- @return string absolute_path
function fab.path_abs(path) end

--- Make a path relative to the build directory.
--- @param path string
--- @return string relative_path
function fab.path_rel(path) end

--- Get an absolute path to the project root.
--- @return string path
function fab.project_root() end

--- Get an absolute path to the build directory.
--- @return string path
function fab.build_directory() end

--- Find an executable on the system, behaves like the *which* tool on linux.
--- @param name string
--- @return Executable?
function fab.find_executable(name) end

--- Get an executable by path or [Output](lua://Output).
--- @param from string | Output
--- @return Executable
function fab.get_executable(from) end

--- Declare an option that can be passed by the user to Fab.
--- @param name string
--- @param type "string" | "number" | string[]
--- @param required boolean?
--- @return any
function fab.option(name, type, required) end

--- Define a [Source](lua://Source).
--- @param path string
--- @return Source
function fab.source(path) end

--- Define a [Rule](lua://Rule).
--- @param args { name: string, command: string | (Executable | string)[], description: (string | (Executable | string)[])?, depstyle: ("normal" | "gcc" | "clang" | "msvc")?, compdb: boolean? }
--- @return Rule
function fab.rule(args) end

--- Declare a [Dependency](lua://Dependency).
--- @param name string
--- @param url string
--- @param revision string
--- @return Dependency
function fab.dependency(name, url, revision) end
