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

--- Get a linker object.
--- @param linker ("ld.lld" | "ld")?
--- @param path string?
--- @return Linker?
function builtins.get_linker(linker, path)
    local linkers = { "ld.lld", "ld" }

    local name = nil
    local exec = nil
    if type(linker) == "string" and linkers:contains(linker) then
        linkers = { linker }

        if path ~= nil then
            name = linker
            exec = fab.get_executable(path)
            goto found
        end
    end

    for _, possible_linker in ipairs(linkers) do
        exec = fab.find_executable(possible_linker)
        name = possible_linker
        if exec ~= nil then
            goto found
        end
    end

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
    --- @return Output
    function Linker:link(name, objects, args)
        return self.rule:build(name, objects, { args = table.join(args or {}, " ") })
    end

    return Linker
end

--- Get a C compiler object.
--- @param compiler ("clang" | "gcc")?
--- @param path string?
--- @return Compiler?
function builtins.c.get_compiler(compiler, path)
    local compilers = { "clang", "gcc" } -- Must match depstyle values

    local name = nil
    local exec = nil
    if type(compiler) == "string" and compilers:contains(compiler) then
        compilers = { compiler }

        if path ~= nil then
            name = compiler
            exec = fab.get_executable(path)
            goto found
        end
    end

    for _, possible_compiler in ipairs(compilers) do
        exec = fab.find_executable(possible_compiler)
        name = possible_compiler
        if exec ~= nil then
            goto found
        end
    end

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
            compdb = true
        }),
        link_rule = fab.rule({
            name = "compiler_c_" .. name .. "_link",
            description = "Linking C objects @IN@ to @OUT@",
            command = { exec, "@ARGS@", "-o", "@OUT@", "@IN@" }
        })
    }

    --- Compile source files into separate object files.
    --- @param sources Source[]
    --- @param include_dirs IncludeDirC[]
    --- @param args string[]
    --- @return Output[]
    function Compiler:compile_objects(sources, include_dirs, args)
        args = args or {}

        for _, include_dir in ipairs(include_dirs or {}) do
            table.insert(args, "-I" .. include_dir.path)
        end

        local outputs = {}
        for _, source in ipairs(sources) do
            local output = self.compile_rule:build(source.path .. ".o", { source }, {
                depfile = source.path .. ".d",
                args = table.join(args, " ")
            })

            table.insert(outputs, output)
        end

        return outputs
    end

    --- Use the compiler to link object files together.
    --- @param name string Name of the output
    --- @param objects (Source | Output)[]
    --- @param args string[]
    --- @return Output
    function Compiler:link(name, objects, args)
        args = args or {}
        return self.link_rule:build(name, objects, { args = table.join(args, " ") })
    end

    --- Use the compiler to compiler and link source files.
    --- @param name string Name of the output
    --- @param sources Source[]
    --- @param include_dirs IncludeDirC[]
    --- @param args string[]
    --- @return Output
    function Compiler:compile(name, sources, include_dirs, args)
        args = args or {}
        return self:link(name, self:compile_objects(sources, include_dirs, args), args)
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
        path = fab.path_abs(path)
    }

    setmetatable(IncludeDirC, {
        __tostring = function(self) return "IncludeDirC(" .. self.path .. ")" end
    })

    return IncludeDirC
end

--- Get a NASM assembler object.
--- @param path string?
--- @return Assembler?
function builtins.nasm.get_assembler(path)
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
            compdb = true
        })
    }

    --- Assemble source files into individual object files.
    --- @param sources Source[]
    --- @param args string[]
    --- @return Output[]
    function Assembler:assemble(sources, args)
        args = args or {}

        local outputs = {}
        for _, source in ipairs(sources) do
            local output = self.rule:build(source.path .. ".o", { source }, {
                depfile = source.path .. ".d",
                args = table.join(args, " ")
            })

            table.insert(outputs, output)
        end

        return outputs
    end

    return Assembler
end
