# Weaveconfig

Weaveconfig is a configuration tool for monorepos. It allows you to manage all configuration in a single directory in the root of your project.

To use it just run `weaveconfig` in the root of your project, to create the initial configuration run `weaveconfig init`.

The weaveconfig contains 3 kinds of files:

- `_space.jsonc` - This file contains the configuration for the space. A space typically is an app / package within your monorepo.
- `_env.jsonc` - This file contains the configuration / variables for the space.
- other files - These files will be copied into each space inlined with variables from the space.

