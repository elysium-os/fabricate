.. _configuration:

=============
Configuration
=============
The configuration file that Fabricate looks for by default is "fab.lua".
However, by passing the :ref:`config <cli-arg-config>` argument any name can be used.

The configuration is written in `lua <https://www.lua.org/docs.html>`_ (Lua 5.4).
All of the standard lua functions are available, that is to say, the configuration is not sandboxed.
Fabricate exposes a lot of helpers and the :ref:`fab <config-fab>` global for interacting with fab.

In the configuration documentation there are a couple concepts that need to be described:

- In function descriptions the a **returns** field describes the, quite intuitively, return value. Respectively **varargs** describes the arguments to be passed as variable arguments.
- A field annotated with the \*type\*[] type describes a table where all of the keys are numbers and sequential, basically an array.
- A field where the type is annotated with ? is optional, meaning it can be nil.

.. toctree::
   :maxdepth: 1
   :caption: Globals

   fab.rst
   globals.rst
   functions.rst
   object-types.rst