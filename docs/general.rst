=======
General
=======

.. _project-root:

Project root
============
The project root is defined as the directory where the root :ref:`configuration <config>` file resides.

Build Directory
===============
The build directory is the directory where fabricate stores its own and :ref:`ninja <ninjaa>` configuration.
It is also where dependencies are downloaded and where :ref:`ninja <ninjaa>` stores build output.

.. _dependencies:

Dependencies
============
Fabricate's philosophy on dependencies differs greatly from its counterparts.
Instead of trying to build the projects itself it exposes an interface for you (the user) to tell fabricate how to build them.
This is more tedious but in general is a more flexible solution that can be adapted for projects using any build system.

Dependencies are currently only supported as git repositories.
Check out :ref:`fab.dependency() <config-fab-dependency>` and :ref:`Dependency <config-obj-dependency>`.

.. _ninjaa:

Ninja
=====
Fabricate is a meta build system. This means Fabricate does not build anything itself, rather it generates instructions for another build system.
This other build system is `Ninja <https://ninja-build.org/>`_, it is the one and only build system supported by Fabricate.

.. _compile-commands:

Compile Commands
================
Fabricate will generate a "compile_commands.json" into the build directory by default.
Note that only :ref:`rule <config-obj-rule>` objects that turn on the compdb flag will actually produce output into compile commands.
