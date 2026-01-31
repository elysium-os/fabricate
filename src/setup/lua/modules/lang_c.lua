local mod = {}

local function get_gnu_compiler(compiler_type, path)
    path = fab.which(path or compiler_type)

    if path == nil then
        return nil
    end

    --- @class CCompiler
    --- @field compile_rule Rule
    --- @field link_rule Rule
    local CCompiler = {
        compile_rule = fab.def_rule(
            "compiler_" .. compiler_type .. "_compile",
            path .. " -MD -MF @DEPFILE@ -MQ @OUT@ @ARGS@ -c -o @OUT@ @IN@",
            "Compiling C object @OUT@ from @IN@",
            compiler_type,
            true
        ),
        link_rule = fab.def_rule(
            "compiler_" .. compiler_type .. "_link",
            path .. " @ARGS@ -o @OUT@ @IN@",
            "Linking C objects @IN@ to @OUT@"
        )
    }

    --- Compile source file into an object file.
    --- @param artifact string
    --- @param source Source
    --- @param include_dirs CIncludeDir[]?
    --- @param args string[]?
    --- @param depfile string?
    --- @param implicit_inputs (Source | Artifact)[]?
    --- @return Artifact
    function CCompiler:compile_object(artifact, source, include_dirs, args, depfile, implicit_inputs)
        local include_args = {}
        for _, include_dir in ipairs(include_dirs or {}) do
            table.insert(include_args, "-I" .. include_dir.path)
        end

        return self.compile_rule:build(artifact, { source }, {
            args = table.join(args or {}, " ") .. " " .. table.join(include_args, " "),
            depfile = depfile or artifact .. ".d"
        }, implicit_inputs)
    end

    --- Use the compiler to link object files together.
    --- @param artifact string
    --- @param objects (Source | Artifact)[]
    --- @param args string[]?
    --- @param linker_script (Source | Artifact)?
    --- @param implicit_inputs (Source | Artifact)[]?
    --- @return Artifact
    function CCompiler:link(artifact, objects, args, linker_script, implicit_inputs)
        local implicits = {}
        local args_str = table.join(args or {}, " ")

        if linker_script ~= nil then
            table.insert(implicits, linker_script)
            args_str = args_str .. " -T" .. linker_script.path
        end

        if implicit_inputs ~= nil then
            table.extend(implicits, implicit_inputs)
        end

        return self.link_rule:build(artifact, objects, { args = args_str }, implicits)
    end

    --- Compile source files into separate object files.
    --- @param sources Source[]
    --- @param args string[]?
    --- @param include_dirs CIncludeDir[]?
    --- @param implicit_inputs (Source | Artifact)[]?
    --- @return Artifact[]
    function CCompiler:generate(sources, args, include_dirs, implicit_inputs)
        local artifacts = {}
        for _, source in ipairs(sources) do
            local genpath = generator_artifact_name(source)
            table.insert(artifacts,
                self:compile_object(genpath .. ".o", source, include_dirs, args, genpath .. ".d", implicit_inputs))
        end
        return artifacts
    end

    --- Use the compiler to compiler and link source files.
    --- @param artifact string
    --- @param sources Source[]
    --- @param args string[]?
    --- @param include_dirs CIncludeDir[]?
    --- @param linker_script (Source | Artifact)?
    --- @param implicit_inputs (Source | Artifact)[]?
    --- @return Artifact
    function CCompiler:compile(artifact, sources, args, include_dirs, linker_script, implicit_inputs)
        return self:link(artifact, self:generate(sources, args or {}, include_dirs), args or {}, linker_script, implicit_inputs)
    end

    setmetatable(CCompiler, {
        __tostring = function(self) return "CCompiler(" .. compiler_type .. ", " .. path .. ")" end
    })

    return CCompiler
end

--- Get a clang compiler.
--- @param path string? Optional full path to a clang binary
--- @return CCompiler?
function mod.get_clang(path)
    return get_gnu_compiler("clang", path)
end

--- Get a GCC compiler.
--- @param path string? Optional full path to a gcc binary
--- @return CCompiler?
function mod.get_gcc(path)
    return get_gnu_compiler("gcc", path)
end

--- Get a any C compiler.
--- @return CCompiler?
function mod.get_compiler()
    local compiler_fns = { mod.get_clang, mod.get_gcc }

    for _, compiler_fn in ipairs(compiler_fns) do
        local compiler = compiler_fn()
        if compiler ~= nil then
            return compiler
        end
    end

    return nil
end

--- Create an include directory object.
--- @param path string
--- @return CIncludeDir
function mod.include_dir(path)
    --- @class CIncludeDir
    --- @field path string
    local CIncludeDir = {
        path = fab.path_rel(path)
    }

    setmetatable(CIncludeDir, {
        __tostring = function(self) return "CIncludeDir(" .. self.path .. ")" end
    })

    return CIncludeDir
end

return mod
