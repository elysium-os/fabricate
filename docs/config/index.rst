.. _config:

=============
Configuration
=============
The configuration file that Fabricate looks for by default is "fab.lua".
However, by passing the :ref:`config <cli-flag-config>` flag any path can be used.

The configuration is written in `Lua <https://www.lua.org/docs.html>`_ (Lua 5.4).
All of the standard lua functions are available, that is to say, the configuration is not sandboxed.
Fabricate exposes a lot of helpers and the :ref:`fab <config-fab>` global for interacting with fab.

In the configuration documentation there are a couple concepts that need to be described:

- In function descriptions the a **returns** field describes the, quite intuitively, return value. Respectively **varargs** describes the arguments to be passed as variable arguments.
- The type annotations follow the style of the `Lua Language Server type annotations <https://luals.github.io/wiki/annotations/#documenting-types>`_.

.. toctree::
   :maxdepth: 1
   :caption: API

   fab.rst
   object-types.rst
   builtins.rst

.. _config-identifier:

Identifier
==========
An identifier is a sequence of characters that:

- Consists of:

    - Alphabetic or numeric characters
    - ``-``, ``_``, ``.``

- Does not begin with ``fab_``
