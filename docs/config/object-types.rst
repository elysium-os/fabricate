============
Object Types
============

There are a number of opaque *objects* (really `userdata <https://www.lua.org/pil/28.1.html>`_) that fabricate defines.
They are used both for convenience and correctness.

.. _config-obj-compiler:

Compiler
========
Represents a compiler (or assembler... anything that produces object files really).

Compile:build(sources, arguments, include_directories)
------------------------------------------------------
sources: Source[]
    List of sources to compile into object files.
arguments: string[]
    Arguments to pass to the compiler.
include_directories: IncludeDirectory[]?
    Include directories to pass to the compiler.
returns: Object[]
    List of produces object files.

Compile source files into object files using the provided arguments.
Include directories can be given as well as long as the compiler supports them.

.. _config-obj-linker:

Linker
======
Represent a linker.

Linker:link(objects, arguments, output_filename)
------------------------------------------------
objects: Object[]
    List of object files to link together.
arguments: string[]
    Arguments to pass to the linker.
output_filename: string
    Filename of the linked file.

Link object files together using the provided arguments.

.. _config-obj-source:

Source
======
filename: string
    Filename of the source file.
full_path: string
    Path of the source file.

Represents a singular source file.

.. _config-obj-include-directory:

IncludeDirectory
================
filename: string
    Filename of the include directory file.
full_path: string
    Path of the include directory file.

Represents a singular include directory.

.. _config-obj-object:

Object
======
Represent an object file.

.. _config-obj-executable:

Executable
==========
filename: string
    Filename of the executable.

Represents an executable, whether that it is one built by fabricate or found on the system.

.. _config-obj-dependency:

Dependency
==========
name: string
    Name given to the dependency.
path: string
    Path to the downloaded dependency.

Represents a downloaded dependency. See the :ref:`dependencies <dependencies>` section.

Dependency:glob(pattern)
------------------------
pattern: string
    Glob pattern.
returns: string[]
    List of found paths.

Practically identical to :ref:`fab.glob <config-fab-glob>` except relative to the directory of the downloaded dependency.