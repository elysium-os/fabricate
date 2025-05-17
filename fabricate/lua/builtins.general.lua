-- builtins.general.lua

--- Check whether a string starts with a given substring.
--- @param s string
--- @param start string
function string.starts_with(s, start)
    return s:sub(1, #start) == start
end

--- Check whether a string ends with a given substring.
--- @param s string
--- @param ending string
function string.ends_with(s, ending)
    return ending == "" or s:sub(- #ending) == ending
end

--- Add all of the values of other into the table.
--- @param t table
--- @param other table
function table.extend(t, other)
    for _, v in ipairs(other) do
        table.insert(t, v)
    end
end

--- Check whether the table contains a given value.
--- @param t table
--- @param value any
function table.contains(t, value)
    for _, v in ipairs(t) do
        if v == value then
            return true
        end
    end
    return false
end

--- Join a table of strings
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

--- Print the table key-values.
--- @param t table
function table.print(t)
    for k, v in pairs(t) do
        print(k, v)
    end
end
