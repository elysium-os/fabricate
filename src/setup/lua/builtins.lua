--- Collect paths and generate a list of sources.
--- @vararg string | string[]
--- @return Source[]
function sources(...)
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

--- Join paths together. Identical to `fab.path_join(...)`.
--- @vararg string
--- @return string
function path(...)
    return fab.path_join(...)
end

--- Get an artifact name for a given source file.
--- @param source Source
--- @return string
function generator_artifact_name(source)
    return "gen_" .. source.path:gsub("_", "__"):gsub("[^A-Za-z0-9._-]", "_")
end

--- Generate artifacts from sources based on their file extension.
--- @param sources Source[]
--- @param generators { [string]: fun(sources: Source[]): Artifact[] }
--- @return Artifact[]
function generate(sources, generators)
    local mapped = {}
    for _, source in ipairs(sources) do
        for extension, generator in pairs(generators) do
            if source.path:ends_with("." .. extension) then
                mapped[extension] = mapped[extension] or { generator = generator, sources = {} }
                table.insert(mapped[extension].sources, source)
            end
        end
    end

    local artifacts = {}
    for _, m in pairs(mapped) do
        table.extend(artifacts, m.generator(m.sources))
    end

    return artifacts
end
