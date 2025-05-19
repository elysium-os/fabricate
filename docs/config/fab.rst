.. _config-fab:

===========
Fab Library
===========
.. contents::

.. _config-fab-glob:

fab.glob(pattern)
=================
pattern: string
    Glob pattern to match.
varargs: string
    Glob patterns to ignore.
returns: string[]
    List of found paths.

Matches a `glob <https://pubs.opengroup.org/onlinepubs/9699919799/functions/glob.html>`_ against the project root, excludes matches of ignore globs, returns the remaining matches.

.. code:: lua

    -- Example usage
    local c_files = fab.glob("**/*.c", "test/**/*.c")

.. _config-fab-path_join:

fab.path_join(...)
==================
varargs: string
    Path components to join.
returns: string
    Joined path.

Joins path components together using the `golang <https://go.dev/>`_ `filepath.Join <https://pkg.go.dev/path/filepath#Join>`_ function.

.. code:: lua

    -- Example usage
    local path = fab.path_join("/home", username, "project")

.. _config-fab-path_abs:

fab.path_abs(path)
==================
path: string
    Path to make absolute.
returns: string
    Absolute path.

Make a relative path absolute based on the current directory (directory of the config) or return the original path if it already absolute.
Uses the `golang <https://go.dev/>`_ `filepath.Abs <https://pkg.go.dev/path/filepath#Abs>`_ function internally.

.. code:: lua

    -- Example usage
    local abs_path = fab.path_abs("test.c")

.. _config-fab-path_rel:

fab.path_rel(path)
==================
path: string
    Path to make relative.
returns: string
    Relative path.

Make a path relative to the build directory.
Uses the `golang <https://go.dev/>`_ `filepath.Rel <https://pkg.go.dev/path/filepath#Rel>`_ function internally.

.. _config-fab-string_split:

fab.string_split(str, sep, n)
=============================
str: string
    String to split by separator.
sep: string
    Separator to use.
n: number?
    Number of substrings to return.
returns: string[]
    Array of substrings.

Split a string by seprator. The n parameter defines how many substrings to return, negative values mean all of them.

.. _config-fab-project_root:

fab.project_root()
==================
returns: string
    An absolute path to the :ref:`root of the project <project-root>`.

.. code:: lua

    -- Example usage
    local something_file = fab.path_join(fab.project_root(), "support/something.txt")

.. _config-fab-build_directory:

fab.build_directory()
==================
returns: string
    An absolute path to the build directory.

.. _config-fab-find_executable:

fab.find_executable(search)
===========================
search: string
    Executable name.
returns: :ref:`Executable <config-obj-executable>` | nil
    If the executable is found, it is returned, otherwise nil.

Searches the system for an executable.
The default system search paths are used (for example, `/bin` and `/usr/bin`).
Internally this uses the `which golang package <github.com/hairyhenderson/go-which>`_.

.. code:: lua

    -- Example usage
    local neofetch = fab.find_executable("neofetch")

.. _config-fab-get_executable:

fab.get_executable(path)
========================
path: string
    Path to an executable.
returns: :ref:`Executable <config-obj-executable>`
    Executable at the path.

Get an executable by path. Prefer :ref:`find_executable <config-fab-find_executable>`, this is meant for more complex methods of finding the executable.

*Hint*: Can be used to turn an output into an executable.

.. _config-fab-option:

fab.option(option, default)
===========================
name: string
    The name of the option (unique and an :ref:`identifier <config-identifier>`).
type: "string" | "number" | []string
    The type of the option.
required: bool?
    Whether the option is required.
returns: any
    Value of the option, either nil or the value passed by the user.

Defines and returns the value of an option that can be passed to fab by the caller using the :ref:`option CLI argument <cli-flag-option>`.

.. code:: lua

    -- Example usage
    local build_type = fab.option("buildtype", { "development", "release" }) or "development"

    if build_type == "development" then
        ...
    end

.. _config-fab-source:

fab.source(path)
================
path: string
    Path to a source file.
returns: :ref:`Source <config-obj-source>`
    A :ref:`Source <config-obj-source>` representing the path given.

Ensures the path is a valid source file and turns it into a :ref:`Source <config-obj-source>` object.

.. code:: lua

    -- Example usage
    local c_sources = {}
    for _, v in ipairs(fab.glob("*.c")) do
        table.insert(c_sources, fab.source(v))
    end

.. _config-fab-rule:

fab.rule(config)
================
config: table
    Rule configuration, described below.
returns: :ref:`Rule <config-obj-dependency>`
    The :ref:`Rule <config-obj-dependency>` produced by the configuration.

Produces a rule based on the following options:

.. _config-fab-rule-table:

====================== =============================================================== ======== ==================================================================
Key                    Type                                                            Required Description
====================== =============================================================== ======== ==================================================================
name                   string                                                          yes      Unique name of the rule (an :ref:`identifier <config-identifier>`)
command                string | (string | :ref:`Executable <config-obj-executable>`)[] yes      Command invoked on a build of the rule
description            string | (string | :ref:`Executable <config-obj-executable>`)[] no       Description of one rule invocation
depstyle               "normal" | "gcc" | "clang" | "msvc"                             no       The type of dependency files generated by this rule
compdb                 bool                                                            no       Whether to generate the compilation db, defaults to false
====================== =============================================================== ======== ==================================================================

.. _config-fab-rule-embeds:

The command and description allows for "embed variables", the embeds take the following form: ``@EMBED@``.
The names of the embeds are case-insensitive. They are replaced by values passed at each invocation of a rule build.
Fabricate supports a few special embeds:

============= ===========================
Name          Description
============= ===========================
``@IN@``      Source file path
``@OUT@``     Output file path
``@DEPFILE@`` Dependency file path
============= ===========================

.. _config-fab-dependency:

fab.dependency(name, url, revision)
===================================
name: string
    Unique name of the dependency (an :ref:`identifier <config-identifier>`).
url: string
    Git URL to the repository of the dependency.
revision: string
    Git revision to use. Can be commit hash, branch, tag, etc.
returns: :ref:`Dependency <config-obj-dependency>`
    A :ref:`Dependency <config-obj-dependency>` representing the input parameters.

Defines a project dependency, in short these are just git repos that Fabricate handles.
For more information check out the :ref:`dependencies <dependencies>` section.

.. code:: lua

    -- Example usage
    local stb = fab.dependency("stb", "https://github.com/nothings/stb.git", "master")
