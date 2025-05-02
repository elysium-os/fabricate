=======
General
=======

.. _project-root:

Project root
============
The project root is defined as the directory where the root :ref:`configuration <configuration>` file resides. 

.. _dependencies:

Dependencies
============
Fabricate's philosophy on dependencies differs greatly from its counterparts.
Instead of trying to build the projects itself it exposes an interface for you (the user) to tell fabricate how to build them.
This is more tedious but in general is a more flexible solution that can be adapted for projects using any build system.

Dependencies are currently only supported as git repositories.
Check out :ref:`fab.dependency() <config-fab-dependency>` and :ref:`Dependency <config-obj-dependency>`.

.. _compile-commands:

Compile Commands
================
Fabricate will generate a "compile_commands.json" into the build directory by default.
Note that only :ref:`compiler <config-obj-compiler>` objects that implement :ref:`compile_command_format <config-fab-compiler-table>` will actually produce output into compile commands.