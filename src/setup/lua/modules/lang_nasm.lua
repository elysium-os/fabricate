local mod = {}

--- Get a NASM assembler object.
--- @param path string? Optional full path to a nasm binary
--- @return NASM?
function mod.get_nasm(path)
    path = fab.which(path or "nasm")

    if path == nil then
        return nil
    end

    --- @class NASM
    --- @field name string
    --- @field rule Rule
    local Nasm = {
        name = "nasm",
        rule = fab.def_rule(
            "assembler_nasm_assemble",
            path .. " @ARGS@ -MD @DEPFILE@ -MQ @OUT@ -o @OUT@ @IN@",
            "Assembling @IN@ from @OUT@",
            "gcc",
            true
        )
    }

    --- Assemble source file.
    --- @param name string
    --- @param source Source
    --- @param args string[]
    --- @param depfile string?
    --- @param implicit_inputs (Source | Artifact)[]?
    --- @return Artifact
    function Nasm:assemble(name, source, args, depfile, implicit_inputs)
        return self.rule:build(name, { source }, {
            args = table.join(args, " "),
            depfile = depfile or name .. ".d"
        }, implicit_inputs)
    end

    --- Assemble source files into separate object files.
    --- @param sources Source[]
    --- @param args string[]
    --- @param implicitDependencies (Source | Artifact)[]?
    --- @return Artifact[]
    function Nasm:generate(sources, args, implicitDependencies)
        local outputs = {}
        for _, source in ipairs(sources) do
            local genpath = generator_artifact_name(source)
            table.insert(outputs, self:assemble(genpath .. ".o", source, args, genpath .. ".d", implicitDependencies))
        end
        return outputs
    end

    return Nasm
end

return mod
