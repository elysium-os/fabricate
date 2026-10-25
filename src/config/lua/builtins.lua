--- Print the table key-values.
--- @param tbl table
function table.print(tbl)
    for k, v in pairs(tbl) do
        print(k, v)
    end
end

--- Add all of the values of other into the table.
--- @param t table
--- @param other table
function table.extend(t, other)
    for _, v in ipairs(other) do
        table.insert(t, v)
    end
end

--- Merge two tables. On collision table2 has priority.
--- @param table1 table
--- @param table2 table
--- @return table
function table.merge(table1, table2)
    local merged = {}
    for k, v in pairs(table1) do
        merged[k] = v
    end
    for k, v in pairs(table2) do
        merged[k] = v
    end
    return merged
end

--- Join a table of strings by separator.
--- @param t string[]
--- @param separator string
--- @return string
function table.join(t, separator)
    local str = ""
    local first = true

    for _, v in ipairs(t) do
        if not first then
            str = str .. separator
        else
            first = false
        end
        str = str .. v
    end

    return str
end

--- Check whether the table contains a given value.
--- @param t table
--- @param value any
--- @return boolean
function table.contains(t, value)
    for _, v in ipairs(t) do
        if v == value then
            return true
        end
    end
    return false
end

--- Collect the keys of a table.
--- @param t table
--- @return any[]
function table.keys(t)
    local keys = {}
    for key, _ in pairs(t) do
        table.insert(keys, key)
    end
    return keys
end

--- Map a table to a list of values using a function.
--- @param t table
--- @param fn fun(k: any, v: any): any
--- @return any[]
function table.map(t, fn)
    local values = {}
    for k, v in pairs(t) do
        table.insert(values, fn(k, v))
    end
    return values
end

--- Check whether a string starts with a given substring.
--- @param str string
--- @param start string
function string.starts_with(str, start)
    return str:sub(1, #start) == start
end

--- Check whether a string ends with a given substring.
--- @param str string
--- @param ending string
function string.ends_with(str, ending)
    return ending == "" or str:sub(- #ending) == ending
end

--- Split a string by separator.
--- @param str string
--- @param separator string
--- @return string[]
function string.split(str, separator)
    separator = separator or "%s"

    local t = {}
    for s in string.gmatch(str, "([^" .. separator .. "]+)") do
        table.insert(t, s)
    end

    return t
end
