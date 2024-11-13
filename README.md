# Weaveconfig

Weaveconfig is a configuration tool for monorepos. It allows you to manage all configuration in a single directory in the root of your project.

To use it just run `weaveconfig gen` in the root of your project, to create the initial configuration run `weaveconfig init`.

The weaveconfig contains 3 kinds of files:

- `_space.jsonc` - This file contains the configuration for the space. A space typically is an app / package within your monorepo.
- `_env.jsonc` - This file contains the configuration / variables for the space.
- other files - These files will be copied into each space inlined with variables from the space.

## \_space.jsonc

The `_space.jsonc` file defines a configuration space and supports the following fields:

- `name` (required): A unique identifier for the space, used for dependency references. Must be unique across all spaces.

- `dependencies` (optional): An array of other space names that this space depends on. The referenced spaces must exist within the weaveconfig directory. Circular dependencies are not allowed. If the environment names of the dependency don't match they will be remapped based on the equvalent in the root space.

- `environments` (optional): An array of environment names supported by this space (e.g. "development", "staging", "production"). These names are used in mappings and must be unique within the space.
- `space_to_parent_mapping` (optional): Maps environments in this space to environments in the parent space. For root spaces (those without a parent), this maps to the ENV variable values. For non-root spaces, this maps to environments in the closest parent space (nearest ancestor directory with \_space.jsonc). If omitted, environments are inherited as-is from the parent.

  Example: `{"prod": ["prod1", "prod2"], "dev": ["dev"]}`

- `generate` (optional): Controls configuration generation options:
  - Can be a boolean to toggle all generation
  - Or an object with:
    - `typescript`: Boolean to toggle TypeScript binding generation

When generation is enabled, it creates:

- `gen/config.json`: Contains the resolved configuration
- `gen/binding.ts`: Provides type-safe access to the configuration
- `gen/.gitignore`: Ignores the generated files from the git index, it's recommended to ignore the whole gen folder rather than just individual files.

## \_env.jsonc

The `_env.jsonc` file contains the actual configuration variables for a space. It supports:

- Shared variables in as JSON, these are available in all environments
- Environment-specific variables using `_<env>.env.jsonc` files (e.g. `_prod.env.jsonc`)
- Variables are merged hierarchically from parent spaces to child spaces
- JSON/JSONC format is supported for both file types

The variables defined in these files will be:

1. Merged according to the space hierarchy
2. Made available in the generated config.json
3. Accessible via the TypeScript bindings when enabled
4. Used to substitute values in other files that are copied to the space from the weaveconfig directory

## Runtime

weaveconfig runs purely at build time generating a config that contains variables for all environments at the same time.
This means that the runtime of your application must choose which environment to use at runtime.
The recommended way to do this is to use the `ENV` environment variable, this is also what the TypeScript binding expects.
