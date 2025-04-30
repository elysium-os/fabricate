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
