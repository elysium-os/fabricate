local mod = {}

local function get_ar_generic(ar_type, path)
    --- @class Ar
    --- @field rule Rule
    local Ar = {
        create_rule = fab.def_rule(
            "ar_" .. ar_type .. "_create",
            path .. " rcs @OUT@ @IN@",
            "Creating archive @OUT@ from @IN@"
        )
    }

    --- Create an archive out of objects.
    --- @param output string
    --- @param objects (Source | Artifact)[]
    --- @return Artifact
    function Ar:create(output, objects)
        return self.create_rule:build(output, objects, {})
    end

    return Ar
end

--- Get an AR object.
--- @param ar_type ("llvm_ar" | "ar")?
--- @param path string?
--- @return Ar?
function mod.get_ar(ar_type, path)
    local lookup = { "llvm_ar", "ar" }

    if path ~= nil then
        lookup = { path }
    elseif ar_type ~= nil then
        lookup = { ar_type }
    end

    for _, ar in ipairs(lookup) do
        path = fab.which(ar)
        if path ~= nil then
            return get_ar_generic(ar_type or ar, path)
        end
    end

    return nil
end

return mod
