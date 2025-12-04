-- builtins.fab.lua

--- Collect paths and generate a list of sources.
--- @vararg string | string[]
--- @return Source[]
function sources(...)
    local collect = {}
    for _, v in ipairs({ ... }) do
        if type(v) == "string" then
            table.insert(collect, fab.source(v))
        elseif type(v) == "table" then
            for _, v in ipairs(v) do
                table.insert(collect, fab.source(v))
            end
        end
    end
    return collect
end

--- Join paths together.
--- @vararg string
--- @return string
function path(...)
    return fab.path_join(...)
end

builtins = { c = {}, nasm = {} }

--- Resolve executable from a list of names.
--- @param names string[]
--- @return Executable?
--- @return string? name
function builtins.resolve_executable(names)
    for _, name in ipairs(names) do
        local exec = fab.find_executable(name)
        if exec ~= nil then
            return exec, name
        end
    end
    return nil, nil
end

--- Get output path for a source file for generator type rules.
--- @param source Source
--- @return string
function builtins.generator_output_path(source)
    return "gen_" .. fab.path_rel(source.path)
end

--- Generate objects from sources based on their file extension.
--- @param sources Source[]
--- @param generators { [string]: fun(sources: Source[]): Output[] }
--- @return Output[]
function builtins.generate(sources, generators)
    local mapped = {}
    for _, source in ipairs(sources) do
        for extension, generator in pairs(generators) do
            if source.name:ends_with("." .. extension) then
                mapped[extension] = mapped[extension] or { generator = generator, sources = {} }
                table.insert(mapped[extension].sources, source)
            end
        end
    end

    local objects = {}
    for _, m in pairs(mapped) do
        table.extend(objects, m.generator(m.sources))
    end

    return objects
end

--- Get a linker object.
--- @param linker ("ld.lld" | "ld")?
--- @param path string?
--- @return Linker?
function builtins.get_linker(linker, path)
    local linkers = { "ld.lld", "ld" }

    local name = nil
    local exec = nil
    if type(linker) == "string" and table.contains(linkers, linker) then
        linkers = { linker }

        if path ~= nil then
            name = linker
            exec = fab.get_executable(path)
            goto found
        end
    end

    exec, name = builtins.resolve_executable(linkers)

    ::found::

    if exec == nil then
        return nil
    end

    --- @class Linker
    --- @field name string
    --- @field rule Rule
    local Linker = {
        name,
        rule = fab.rule({
            name = "linker_" .. name,
            description = "Linking @IN@ to @OUT@",
            command = { exec, "-o", "@OUT@", "@ARGS@", "@IN@" }
        })
    }

    --- Link object files together.
    --- @param name string Name of the output
    --- @param objects (Source | Output)[]
    --- @param args string[]
    --- @param implicitDependencies (Source | Output)[]?
    --- @return Output
    function Linker:link(name, objects, args, implicitDependencies)
        return self.rule:build(name, objects, { args = table.join(args or {}, " ") }, implicitDependencies)
    end

    return Linker
end

--- Get a C compiler object.
--- @param compiler ("clang" | "gcc")?
--- @param path string?
--- @param compdb boolean?
--- @return Compiler?
function builtins.c.get_compiler(compiler, path, compdb)
    compdb = compdb == nil or compdb

    local compilers = { "clang", "gcc" } -- Must match depstyle values

    local name = nil
    local exec = nil
    if type(compiler) == "string" and table.contains(compilers, compiler) then
        compilers = { compiler }

        if path ~= nil then
            name = compiler
            exec = fab.get_executable(path)
            goto found
        end
    end

    exec, name = builtins.resolve_executable(compilers)

    ::found::

    if exec == nil then
        return nil
    end

    --- @class Compiler
    --- @field name string
    --- @field compile_rule Rule
    --- @field link_rule Rule
    local Compiler = {
        name,
        compile_rule = fab.rule({
            name = "compiler_c_" .. name .. "_compile",
            description = "Compiling C object @OUT@ from @IN@",
            command = { exec, "-MD", "-MF", "@DEPFILE@", "-MQ", "@OUT@", "@ARGS@", "-c", "-o", "@OUT@", "@IN@" },
            depstyle = name,
            compdb = compdb
        }),
        link_rule = fab.rule({
            name = "compiler_c_" .. name .. "_link",
            description = "Linking C objects @IN@ to @OUT@",
            command = { exec, "@ARGS@", "-o", "@OUT@", "@IN@" }
        })
    }

    --- Compile source file into an object file.
    --- @param name string
    --- @param source Source
    --- @param include_dirs IncludeDirC[]
    --- @param args string[]
    --- @param depfile string?
    --- @param implicitDependencies (Source | Output)[]?
    --- @return Output
    function Compiler:compile_object(name, source, include_dirs, args, depfile, implicitDependencies)
        args = table.shallow_clone(args or {})

        for _, include_dir in ipairs(include_dirs or {}) do
            table.insert(args, "-I" .. include_dir.path)
        end

        return self.compile_rule:build(name, { source }, {
            args = table.join(args, " "),
            depfile = depfile or name .. ".d"
        }, implicitDependencies)
    end

    --- Compile source files into separate object files.
    --- @param sources Source[]
    --- @param args string[]
    --- @param include_dirs IncludeDirC[]
    --- @param implicitDependencies (Source | Output)[]?
    --- @return Output[]
    function Compiler:generate(sources, args, include_dirs, implicitDependencies)
        local outputs = {}
        for _, source in ipairs(sources) do
            local genpath = builtins.generator_output_path(source)
            table.insert(outputs,
                self:compile_object(genpath .. ".o", source, include_dirs, args, genpath .. ".d", implicitDependencies))
        end
        return outputs
    end

    --- Use the compiler to link object files together.
    --- @param name string Name of the output
    --- @param objects (Source | Output)[]
    --- @param args string[]
    --- @param implicitDependencies (Source | Output)[]?
    --- @return Output
    function Compiler:link(name, objects, args, implicitDependencies)
        return self.link_rule:build(name, objects, { args = table.join(args or {}, " ") }, implicitDependencies)
    end

    --- Use the compiler to compiler and link source files.
    --- @param name string Name of the output
    --- @param sources Source[]
    --- @param args string[]
    --- @param include_dirs IncludeDirC[]
    --- @param implicitDependencies (Source | Output)[]?
    --- @return Output
    function Compiler:compile(name, sources, args, include_dirs, implicitDependencies)
        return self:link(name, self:generate(sources, args or {}, include_dirs), args or {}, implicitDependencies)
    end

    return Compiler
end

-- Create an include directory object.
-- @param path
-- @return IncludeDirC
function builtins.c.include_dir(path)
    --- @class IncludeDirC
    --- @field path string
    local IncludeDirC = {
        path = fab.path_rel(path)
    }

    setmetatable(IncludeDirC, {
        __tostring = function(self) return "IncludeDirC(" .. self.path .. ")" end
    })

    return IncludeDirC
end

--- Get a NASM assembler object.
--- @param path string?
--- @param compdb boolean?
--- @return Assembler?
function builtins.nasm.get_assembler(path, compdb)
    compdb = compdb == nil or compdb

    local exec = nil
    if type(path) == "string" then
        exec = fab.get_executable(path)
    else
        exec = fab.find_executable("nasm")
    end

    if exec == nil then
        return nil
    end

    --- @class Assembler
    --- @field name string
    --- @field rule Rule
    local Assembler = {
        name = "nasm",
        rule = fab.rule({
            name = "nasm",
            description = "Assembling @IN@ from @OUT@",
            command = { exec, "@ARGS@", "-MD", "@DEPFILE@", "-MQ", "@OUT@", "-o", "@OUT@", "@IN@" },
            depstyle = "gcc",
            compdb = compdb
        })
    }

    --- Assemble source file.
    --- @param name string
    --- @param source Source
    --- @param args string[]
    --- @param depfile string?
    --- @param implicitDependencies (Source | Output)[]?
    --- @return Output
    function Assembler:assemble(name, source, args, depfile, implicitDependencies)
        return self.rule:build(name, { source }, {
            args = table.join(args, " "),
            depfile = depfile or name .. ".d"
        }, implicitDependencies)
    end

    --- Assemble source files into separate object files.
    --- @param sources Source[]
    --- @param args string[]
    --- @param implicitDependencies (Source | Output)[]?
    --- @return Output[]
    function Assembler:generate(sources, args, implicitDependencies)
        local outputs = {}
        for _, source in ipairs(sources) do
            local genpath = builtins.generator_output_path(source)
            table.insert(outputs, self:assemble(genpath .. ".o", source, args, genpath .. ".d", implicitDependencies))
        end
        return outputs
    end

    return Assembler
end
