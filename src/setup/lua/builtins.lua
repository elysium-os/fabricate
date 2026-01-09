--- Collect paths and generate a list of sources.
--- @vararg string | string[]
--- @return Source[]
function Sources(...)
    local collect = {}

    local function checked_insert(v)
        assert(type(v) == "string", "invalid type `" .. type(v) .. "` passed to sources")
        table.insert(collect, fab.def_source(v))
    end

    for _, v in ipairs({ ... }) do
        if type(v) == "table" then
            for _, v in ipairs(v) do
                checked_insert(v)
            end
        else
            checked_insert(v)
        end
    end

    return collect
end
