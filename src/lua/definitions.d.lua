---@meta

---@param message string
function panic(message) end

---@param message string
function warn(message) end

---@param path string
---@return Source
function source(path) end

---@param path string
---@return IncludeDirectory
function include_directory(path) end

---@param base string
---@vararg string
---@return string
function path(base, ...) end

---@class Fab
fab = {}

---@param search string
---@return Executable?
function fab.find_executable(search) end

---@param pattern string
---@return string[]
function fab.glob(pattern) end

---@param option string
---@param default any
---@return any
function fab.option(option, default) end

---@return string
function fab.project_root() end

---@param table { name: string, format_include_dir: (fun(string): string)?, compile_command_format: string?, executable: Executable, command: string, description: string? }
---@return Compiler
function fab.create_compiler(table) end

---@param table { name: string, executable: Executable, command: string, description: string? }
---@return Linker
function fab.create_linker(table) end

---@param name string
---@param url string
---@param revision string
---@return Dependency
function fab.dependency(name, url, revision) end

---@class Compiler
Compiler = {}

---@param sources Source[]
---@param args string[]
---@param include_dirs string[]?
---@return Object[]
function Compiler:build(sources, args, include_dirs) end

---@class Linker
Linker = {}

---@param objects Object[]
---@param output_filename string
---@param args string[]
function Linker:link(objects, output_filename, args) end

---@class Source
---@field filename string
---@field full_path string
Source = {}

---@class IncludeDirectory
---@field filename string
---@field full_path string
IncludeDirectory = {}

---@class Object
Object = {}

---@class Executable
---@field filename string
Executable = {}

---@class Dependency
---@field name string
---@field path string
Dependency = {}

---@param pattern string
---@return string[]
function Dependency:glob(pattern) end
