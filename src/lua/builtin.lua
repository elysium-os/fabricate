-- Fab Related

function sources(...)
    local collect = {}
    for _, v in ipairs({ ... }) do
        if type(v) == "string" then
            table.insert(collect, source(v))
        elseif type(v) == "table" then
            for _, v in ipairs(v) do
                table.insert(collect, source(v))
            end
        end
    end
    return collect
end

function includes(...)
    local collect = {}
    for _, v in ipairs({ ... }) do
        if type(v) == "string" then
            table.insert(collect, include_directory(v))
        elseif type(v) == "table" then
            for _, v in ipairs(v) do
                table.insert(collect, include_directory(v))
            end
        end
    end
    return collect
end

local c_compilers = { "clang", "gcc", "clang*", "gcc*", "msvc", "msvc*" }
local c_compiler = nil
while c_compiler == nil and #c_compilers ~= 0 do
    c_compiler = fab.find_executable(table.remove(c_compilers, 1))
end

if c_compiler ~= nil then
    CC = fab.create_compiler {
        name = "cc",
        format_include_dir = function(include_dir) return "-I" .. include_dir end,
        executable = c_compiler,
        compile_command_format = "@EXEC@ @FLAGS@ -c @IN@ -o @OUT@",
        command = "@EXEC@ -MD -MF @DEPFILE@ -MQ @OUT@ @FLAGS@ -c @IN@ -o @OUT@",
        description = "Compiling C object @OUT@"
    }

    CC_LD = fab.create_linker({
        name = "cc",
        executable = c_compiler,
        command = "@EXEC@ @FLAGS@ -o @OUT@ @IN@",
        description = "Linking @OUT@"
    })
else
    CC = nil
    CC_LD = nil
    warn("Failed to locate a C compiler")
end

-- General Helpers
function string.starts_with(str, start)
    return str:sub(1, #start) == start
end

function string.ends_with(str, ending)
    return ending == "" or str:sub(- #ending) == ending
end

function table.extend(tbl, other)
    for _, v in ipairs(other) do
        table.insert(tbl, v)
    end
end

function print_table(table)
    for k, v in pairs(table) do
        print(k, v)
    end
end
