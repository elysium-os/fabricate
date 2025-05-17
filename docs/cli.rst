===
CLI
===

The Fabricate CLI is broken into subcommands and flags:

.. _cli-flag-help:

-h, --help
    Displays help with available flag, subcommand, and positional value parameters.

.. _cli-flag-version:

--version
    Displays the program version string.

.. _cli-flag-builddir:

--builddir BUILDDIR
    Specify the build directory path (default: ./build) [Environment Variable: BUILDDIR].

.. _cli-subcommand-configure:

configure
---------
Configures the build directory (and downloads dependencies) with the given arguments.

.. _cli-flag-config:

--config CONFIG
    Specify the configuration file path (default: fab.lua).

.. _cli-flag-prefix:

--prefix PREFIX
    Specify installation prefix (default: /usr).

.. _cli-flag-option:

-o OPTION, --option OPTION
    Specify the value of a *user defined* option (an option defined in the :ref:`configuration <config>`).

.. _cli-subcommand-build:

build
-----
Build the project using :ref:`ninja <ninjaa>`, this is equivalent to calling ``ninja -C <builddir>``.

.. _cli-subcommand-install:

install
-------
Install the output files specified in the configuration. Files are installed in the format of ``<destdir?><prefix>/<install_path>``.

.. _cli-flag-destdir:

--destdir:
    Specify the destdir of the install [Environment Variable: DESTDIR].
