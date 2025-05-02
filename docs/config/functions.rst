=========
Functions
=========

.. _config-fn-sources:

sources(...)
============
varargs: string | string[]
    Source paths.
returns: :ref:`Source <config-obj-source>`\[]
    Resolved sources.

Consume paths and lists of paths, flatten them, and return them as a list of sources.

.. code:: lua

    -- Example usage
    local c_sources = source(fab.glob("**/*.c"))

.. _config-fn-includes:

includes(...)
=============
varargs: string | string[]
    Include directory paths.
returns: :ref:`IncludeDirectory <config-obj-include-directory>`\[]
    Resolved include directories.

Consume paths and lists of paths, flatten them, and return them as a list of include directories.

.. code:: lua

    -- Example usage
    local include_dirs = includes("include/")

.. _config-fn-table-extend:

table.extend(tbl, other)
========================
tbl: table
    Table to extend.
other: table
    Extend with this table.

Add all values of other to tbl.

.. code:: lua

    -- Example usage
    table.extend(c_sources, fab.glob("other_source/*.c"))

.. _config-fn-string-starts-with:

string.starts_with(str, start)
==============================
str: string
    String to test.
start:
    Substring to test.
returns: bool
    Whether the string `str` starts with `start`.

.. _config-fn-string-ends-with:

string.ends_with(str, ending)
=============================
str: string
    String to test.
ending:
    Substring to test.
returns: bool
    Whether the string `str` ends with `ending`.
