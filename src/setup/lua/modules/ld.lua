local mod = {}

local function get_linker_generic(linker_type, path)
    --- @class Linker
    --- @field rule Rule
    local Linker = {
        rule = fab.def_rule(
            "linker_" .. linker_type .. "_link",
            path .. " -o @OUT@ @ARGS@ @IN@",
            "Linking @IN@ to @OUT@"
        )
    }

    --- Link object files together.
    --- @param output string
    --- @param objects (Source | Artifact)[]
    --- @param args string[]
    --- @param implicit_inputs (Source | Artifact)[]?
    --- @return Artifact
    function Linker:link(output, objects, args, implicit_inputs)
        return self.rule:build(output, objects, { args = table.join(args or {}, " ") }, implicit_inputs)
    end

    return Linker
end

--- Get a linker object.
--- @param linker_type ("ld.lld" | "ld")?
--- @param path string?
--- @return Linker?
function mod.get_linker(linker_type, path)
    local lookup = { "ld.lld", "ld" }

    if path ~= nil then
        lookup = { path }
    elseif linker_type ~= nil then
        lookup = { linker_type }
    end

    for _, linker in ipairs(lookup) do
        path = fab.which(linker)
        if path ~= nil then
            return get_linker_generic(linker_type or linker, path)
        end
    end

    return nil
end

return mod
