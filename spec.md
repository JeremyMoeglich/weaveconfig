# Envoyr Configuration Specification

## **1. Configuration Files and Structure**

- **`_env.jsonc` Files**: Each space (directory) can have an `_env.jsonc` file containing any JSONC-compliant configuration.

  **Example `_env.jsonc`:**

  ```jsonc
  {
    "value1": 5,
    "some_group": {
      "value2": 8
    },
    "some_array": [
      {
        "a": "test"
      }
    ]
  }
  ```

---

## **2. Environments and Variable Lifting**

- **Environment Sections**: Keys in `_env.jsonc` that match environment names (e.g., `"dev"`) define environment-specific configurations.

- **Variable Lifting**: Variables under the active environment's section are "lifted" to the top level when that environment is active.

  **Example `_env.jsonc` with Environment Sections:**

  ```jsonc
  {
    "value0": 3,
    "dev": {
      "value1": 5
    }
  }
  ```

  **Behavior:**

  - **Active Environment is `dev`:**

    ```jsonc
    {
      "value0": 3,
      "value1": 5,          // Lifted from "dev"
      "dev": { "value1": 5 } // Original "dev" section remains
    }
    ```

  - **Active Environment is NOT `dev`:**

    ```jsonc
    {
      "value0": 3,
      "dev": { "value1": 5 } // "dev" section remains nested
    }
    ```

**Ambiguity / Issue:**

- **Variable Conflicts**: If different environments define the same variable names, lifting them to the top level could cause conflicts or unexpected overrides.

- **Resolution Needed**: Define how conflicts are handled when multiple environments have the same variable names.

---

## **3. Alternative Environment Files**

- **Environment-Specific Files**: Using `_dev.env.jsonc` is equivalent to defining `"dev"` in `_env.jsonc`.

  - **Conflict Handling**: Conflicts between `_env.jsonc` and `_dev.env.jsonc` result in errors.

**Ambiguity / Issue:**

- **Conflict Detection**: It's unclear how conflicts are identified between variables in `_env.jsonc` and `_dev.env.jsonc`.

- **Resolution Needed**: Specify whether one source has precedence or if both must be mutually exclusive.

---

## **4. Environment Variables Naming Convention**

- **Conversion to Environment Variables**:

  - **Active Environment Variables**: Variables from the active environment are available both with and without the environment prefix.

    - **Example**: If `dev` is active, `value1` becomes both `VALUE1` and `DEV_VALUE1`.

  - **Other Environments**: Variables from other environments are only available with the environment prefix.

    - **Example**: `prod` variables become `PROD_VARIABLE_NAME`.

- **Top-Level Variables**: Always included as is (e.g., `VALUE0`).

---

## **5. Variable Uniqueness Across Environments**

- **Uniqueness Rule**:

  - All environment variables must be unique across all environments.

  - All supported environments must have the same set of variables.

---

## **6. Spaces and Dependencies**

- **Definition of a Space**:

  - A directory that supports certain environments.

  - May have custom variables.

- **Dependencies**:

  - Defined in `_space.jsonc`.

  - Can depend on other spaces.

  **Example `_space.jsonc`:**

  ```jsonc
  {
    "dependencies": [
      "../service_b",        // Relative to `_space.jsonc`
      "/packages/package_a"  // Absolute from monorepo root
    ]
  }
  ```

---

## **7. Variable Inclusion and Renaming from Dependencies**

- **Selective Inclusion and Templates**:

  - Include subsets of variables and rename them using templates.

  **Example `_space.jsonc` with Template and Keys:**

  ```jsonc
  {
    "dependencies": [
      {
        "name": "../service_b",
        "template": "VITE_{}",
        "keys": ["public"]
      }
    ]
  }
  ```

- **Given `_env.jsonc` in `service_b`:**

  ```jsonc
  {
    "very_secret_variable": "test0",
    "dev": {
      "public_variable": "test1",
      "secret_variable": "test2"
    },
    "public": {
      "not_so_secret": "test3"
    }
  }
  ```

- **Variables Provided:**

  - `VITE_DEV_PUBLIC_VARIABLE = "test1"`
  - `VITE_PUBLIC_NOT_SO_SECRET = "test3"`
  - If `dev` is active: `VITE_PUBLIC_VARIABLE = "test1"`

**Ambiguity / Issue:**

- **Environment Handling**: It's unclear how environment prefixes interact with templates.

- **Resolution Needed**: Define how templates apply to variables from different environments and how conflicts are resolved.

---

## **8. Config Folder and File Copying with Variable Inlining**

- **Structure**:

  - `config/` at monorepo root mirrors the repo's structure.

  - Files are copied into their respective directories, with variables inlined.

- **Reserved Files**:

  - Files starting with `_` are reserved.

  - To include such files, prefix with `__`.

- **Variable Inlining Syntax**:

  - `{{ variable }}` to inline.

  - `\{{ variable }}` to escape.

- **Variable Names**:

  - Use the names from the JSONC files, not the environment variables.

**Ambiguity / Issue:**

- **Variable Availability**: How are variables from different environments handled during inlining?

- **Resolution Needed**: Specify that inlining uses variables from the active environment or define a mechanism to select which environment's variables to use.

---

## **9. Environment Declaration and Mapping**

- **Environment Declaration**:

  - Specify environments in `_space.jsonc`:

    ```jsonc
    {
      "environments": ["prod1", "prod2", "test", "dev"]
    }
    ```

- **Environment Mapping for Dependencies**:

  - Map environments when dependencies have different ones.

  **Example Mapping:**

  ```jsonc
  {
    "mapping": [
      { "from": "prod", "to": "prod1" },
      { "from": "prod", "to": "prod2" },  // Map to multiple targets
      { "from": "dev", "to": "prod1" }    // Valid if no overlap
    ]
  }
  ```

**Ambiguity / Issue:**

- **Overlap Definition**: Unclear what "dev and prod have no overlap" means.

- **Conflict Potential**: Mapping one environment to multiple can cause conflicts.

- **Resolution Needed**: Define how mappings are applied and how conflicts are resolved.

---

## **10. Configuration Generation**

- **Generator Behavior**:

  - Creates a `config_gen` directory in spaces with declared environments.

  - Contains `.env.gen` and `config_bindings.gen.ts`.

- **Environment Selection**:

  - The environment is chosen at runtime, based on the `ENV` variable.

  - The generator doesn't need to know the active environment but needs the list of possible environments.

---

## **11. Conflict Resolution and Errors**

- **Error Handling**:

  - Conflicts result in errors during generation.

  - Variables must be unique across all environments and dependencies.

**Ambiguity / Issue:**

- **Strictness**: The requirement for uniqueness and identical variable sets across environments and dependencies is stringent.

- **Resolution Needed**: Consider mechanisms to handle conflicts gracefully or allow exceptions.