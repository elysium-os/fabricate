===
CLI
===

Fabricate CLI is structured into many subcommands, as well as some global options expected prior to the subcommand:

.. _cli-arg-config:

--config
    Specify the :ref:`configuration file <configuration>` path.

.. _cli-subcommand-configure:

configure
---------
Configures the build directory (and downloads dependencies) with the given configuration.

.. _cli-arg-build:

--build BUILDDIR
    Specify the build directory path. Default: ``build``.

.. _cli-arg-prefix:

--prefix PREFIX
    Set the install prefix. Default: ``/usr/local``.

.. _cli-arg-option:

-D OPTION, --option OPTION
    Set the value of a *user defined* option (an option defined in the :ref:`configuration <configuration>`).
