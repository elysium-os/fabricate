================
Global Variables
================


.. confval:: fab
    
    Explained in detail :ref:`here <config-fab>`.

.. confval:: CC/CC_LD

    The system C compiler and respectively the C compiler acting as a linker.
    It is a :ref:`Compiler <config-obj-compiler>` object or ``nil`` if no system C compiler was found.
    The compiler is found by matching the following in the order its listed.

    - ``clang``
    - ``gcc``
    - ``*clang``
    - ``*gcc``
    - ``msvc``