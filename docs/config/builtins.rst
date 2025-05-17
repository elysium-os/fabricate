========
Builtins
========

Fabricate exposes its native api via the :ref:`fab library <config-fab>` and :ref:`object types <config-obj>`.
However, this api can be cumbersome to use directly which is why Fabricate exposes a set of *builtin* functions written in lua that wrap the api.

The builtins range from language support to just utility functions.
Only the utility functions are described here, language support is left undocumented for the time being.
For information about language support, check out existing usage of Fabricate and the builtins lua files.

.. _config-builtin-sources:

sources(...)
============
varargs: string | string[]
    Source file path or list of source file paths.
returns: Source[]
    Source objects representing the paths.

Collect paths and generate a list of sources.

*Hint*: works well paired with :ref:`fab.glob <config-fab-glob>`

.. code:: lua

    -- Example usage
    local c_sources = source(fab.glob("**/*.c"))

.. _config-builtin-path:

path(...)
=========
varargs: string
    Path components to join.
returns: string
    Joined path.

A 1:1 wrapper around :ref:`fab.path_join <config-fab-path_join>`.
