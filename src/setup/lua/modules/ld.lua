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
    --- @param linker_script (Source | Artifact)?
    --- @param implicit_inputs (Source | Artifact)[]?
    --- @return Artifact
    function Linker:link(output, objects, args, linker_script, implicit_inputs)
        local implicits = {}
        local args_str = table.join(args or {}, " ")

        if linker_script ~= nil then
            table.insert(implicits, linker_script)
            args_str = args_str .. " -T" .. linker_script.path
        end

        if implicit_inputs ~= nil then
            table.extend(implicits, implicit_inputs)
        end

        return self.rule:build(output, objects, { args = args_str }, implicits)
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
