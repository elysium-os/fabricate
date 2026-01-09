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

--- Split a string by separator.
--- @param str string
--- @param separator string
--- @return string[]
function string.split(str, separator)
    separator = separator or "%s"

    local t = {}
    for str in string.gmatch(str, "([^" .. separator .. "]+)") do
        table.insert(t, str)
    end

    return t
end
