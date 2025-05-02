.. _config-fab:

==========
Fab Object
==========
.. contents::

.. _config-fab-find-executable:

fab.find_executable(search)
===========================
search: string
    Executable name or a regex matching the executable name(s).
returns: :ref:`Executable <config-obj-executable>` | nil
    If the executable is find, it is returned. If many are found, an arbitrary one is picked.

Searches the system for an executable.
The default system search paths are used (for example, `/bin` and `/usr/bin`).
Internally this uses the `which rust package <https://crates.io/crates/which>`_.

.. code:: lua
    
    -- Example usage
    local neofetch = fab.find_executable("neofetch")

.. _config-fab-glob:

fab.glob(pattern)
=================
pattern: string
    Glob pattern to use.
returns: string[]
    List of found paths.

Matches a `glob <https://pubs.opengroup.org/onlinepubs/9699919799/functions/glob.html>`_ against the project root and returns the matching file paths.

.. code:: lua

    -- Example usage
    local c_files = fab.glob("**/*.c")

.. _config-fab-project-root:

fab.project_root()
==================
returns: string
    An absolute path to the :ref:`root of the project <project-root>`.

.. code:: lua

    -- Example usage
    local something_file = path(fab.project_root(), "support/something.txt")

.. _config-fab-option:

fab.option(option, default)
===========================
option: string
    The name of the option.
default: any
    Default value in case the option is not passed.
returns: any
    Value of the option, it is always a string unless the default option is returned.

Defines and returns the value of an option that can be passed to fab by the caller using the :ref:`option CLI argument <cli-arg-option>`.

.. code:: lua

    -- Example usage
    local build_type = fab.option("buildtype", "dev")

    if build_type == "dev" then
        ...
    end

.. _config-fab-dependency:

fab.dependency(name, url, revision)
===================================
name: string
    Unique name of the dependency.
url: string
    Git URL to the repository of the dependency.
revision: string
    Git revision to use. Can be commit hash, branch, tag, etc.
returns: :ref:`Dependency <config-obj-dependency>`
    A :ref:`Dependency <config-obj-dependency>` representing the input parameters.

Defines a project dependency, in short these are just git repos that fab handles.
For more information check out the :ref:`dependencies <dependencies>` section.

.. code:: lua

    -- Example usage
    local stb = fab.dependency("stb", "https://github.com/nothings/stb.git", "master")
    local stb_headers = fab.include_directory(stb.path)

.. _config-fab-source:

fab.source(path)
================
path: string
    Path to a source file.
returns: :ref:`Source <config-obj-source>`
    A :ref:`Source <config-obj-source>` representing the path given.

Ensures the path is a valid source file and turns it into a :ref:`Source <config-obj-source>` object.
Check out the :ref:`sources <config-fn-sources>` function as it might be more convenient.

.. code:: lua

    -- Example usage
    local c_sources = {}
    for _, v in ipairs(fab.glob("*.c")) do
        table.insert(c_sources, fab.source(v))
    end

.. _config-fab-include-directory:

fab.include_directory(path)
===========================
path: string
    Path to an include directory.
returns: :ref:`IncludeDirectory <config-obj-include-directory>`
    An :ref:`IncludeDirectory <config-obj-include-directory>` representing the path given.

Ensures the path is a valid include directory and turns it into a :ref:`IncludeDirectory <config-obj-include-directory>` object.
Check out the :ref:`includes <config-fn-includes>`

.. code:: lua

    -- Example usage
    local include_dir = fab.include_directory(path(fab.project_root(), "include"))

.. _config-fab-create-compiler:

fab.create_compiler(config)
===========================
config: table
    Compiler configuration, described below.
returns: :ref:`Compiler <config-obj-compiler>`
    Created compiler.

Creates a compiler, based on the following options:

.. _config-fab-compiler-table:

====================== ========================================= ======== ===============================================================================================
Key                    Type                                      Required Description
====================== ========================================= ======== ===============================================================================================
name                   string                                    yes      Unique name of the compiler
executable             :ref:`Executable <config-obj-executable>` yes      Compiler executable
format_include_dir     function(string): string                  no       Function to format an include directory path into an argument
compile_command_format string                                    no       Format string to use for adding build info into :ref:`compile_commands.json <compile-commands>`
command                string                                    yes      Format command to use for invoking the compiler
description            string                                    no       Description of one build invocation
====================== ========================================= ======== ===============================================================================================

Some of the compiler options support embeds:

.. _config-fab-compiler-embeds:

========= =========================== ======= =========== ======================
Name      Description                 command description compile_command_format
========= =========================== ======= =========== ======================
@EXEC@    Path to compiler executable X                   X
@DEPFILE@ Dependency file             X                    
@FLAGS@   Compiler flags              X                   X
@IN@      Source file                 X       X           X
@OUT@     Object file                 X       X           X
========= =========================== ======= =========== ======================

.. code:: lua

    -- Example usage
    local c_compiler = fab.create_compiler {
        name = "cc",
        format_include_dir = function(include_dir) return "-I" .. include_dir end,
        executable = c_compiler,
        compile_command_format = "@EXEC@ @FLAGS@ -c @IN@ -o @OUT@",
        command = "@EXEC@ -MD -MF @DEPFILE@ -MQ @OUT@ @FLAGS@ -c @IN@ -o @OUT@",
        description = "Compiling C object @OUT@"
    }

.. _config-fab-create-linker:

fab.create_linker(config)
=========================
config: table
    Linker configuration, described below.
returns: :ref:`Linker <config-obj-linker>`
    Created linker.

Creates a linker, based on the following options:

.. _config-fab-linker-table:

=========== ========================================= ======== =============================================
Key         Type                                      Required Description
=========== ========================================= ======== =============================================
name        string                                    yes      Unique name of the linker
executable  :ref:`Executable <config-obj-executable>` yes      Linker executable
command     string                                    yes      Format command to use for invoking the linker
description string                                    no       Description of one link invocation
=========== ========================================= ======== =============================================

Some of the linker options support embeds:

.. _config-fab-linker-embeds:

========= =========================== ======= ===========
Name      Description                 command description
========= =========================== ======= ===========
@EXEC@    Path to linker executable   X 
@FLAGS@   Linker flags                X 
@IN@      Object file(s)              X       X 
@OUT@     Linked file (exec/lib/...)  X       X 
========= =========================== ======= ===========

.. code:: lua

    -- Example usage
    local cc_linker = fab.create_linker({
        name = "cc",
        executable = c_compiler,
        command = "@EXEC@ @FLAGS@ -o @OUT@ @IN@",
        description = "Linking @OUT@"
    })