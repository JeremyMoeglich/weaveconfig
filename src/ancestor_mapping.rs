use lazy_static::lazy_static;
use std::collections::{HashMap, HashSet};
use thiserror::Error;

lazy_static! {
    static ref DEFAULT_ANCESTOR_SET: HashSet<String> = HashSet::new();
}

/// A mapping between ancestor environments and space environments.
/// Every space environment must at most be mapped to one ancestor environment.
/// But multiple ancestor environments can map to the same space environment.
/// For example, you might have the ancestor environments: dev, test, prod1, prod2
/// And the space environments: dev, test, prod
/// Then you might have the following ancestor to space mappings:
/// dev -> dev
/// test -> test
/// prod1 -> prod
/// prod2 -> prod
///
/// And the following space to ancestor mappings:
/// dev -> dev
/// test -> test
/// prod -> [prod1, prod2]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AncestorMapping {
    /// Maps a space environment to a set of ancestor environments.
    space_to_ancestor: HashMap<String, HashSet<String>>,
    /// Maps an ancestor environment to a space environment.
    ancestor_to_space: HashMap<String, String>,
}

#[derive(Error, Debug)]
pub enum RootMappingError {
    #[error("Mapping for ancestor '{0}' already exists and cannot be overwritten")]
    DuplicateAncestor(String),
}

impl AncestorMapping {
    /// Creates a new, empty `AncestorMapping`.
    pub fn new() -> Self {
        AncestorMapping {
            space_to_ancestor: HashMap::new(),
            ancestor_to_space: HashMap::new(),
        }
    }

    pub fn from_space_to_ancestors(
        space_to_ancestors: HashMap<String, HashSet<String>>,
    ) -> Result<Self, RootMappingError> {
        let mut mapping = AncestorMapping::new();
        for (space, ancestors) in space_to_ancestors {
            for ancestor in ancestors {
                mapping.add_mapping(ancestor, space.clone())?;
            }
        }
        Ok(mapping)
    }

    /// Attempts to add a mapping from an ancestor environment to a space environment.
    ///
    /// If the ancestor already exists, returns an error and does not overwrite the existing mapping.
    /// If the space does not exist, it is created.
    ///
    /// # Arguments
    ///
    /// * `ancestor` - The ancestor environment name.
    /// * `space` - The space environment name.
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the mapping was added successfully.
    /// * `Err(RootMappingError)` if the ancestor already exists.
    ///
    /// # Example
    ///
    /// ```rust
    /// root_mapping.add_mapping("prod1".to_string(), "prod".to_string()).unwrap();
    /// ```
    pub fn add_mapping(&mut self, ancestor: String, space: String) -> Result<(), RootMappingError> {
        if self.ancestor_to_space.contains_key(&ancestor) {
            return Err(RootMappingError::DuplicateAncestor(ancestor));
        }

        // Add the ancestor to the ancestor_to_space map.
        self.ancestor_to_space
            .insert(ancestor.clone(), space.clone());

        // Add the ancestor to the space_to_ancestor map.
        self.space_to_ancestor
            .entry(space)
            .or_insert_with(HashSet::new)
            .insert(ancestor);

        Ok(())
    }

    /// Replaces an existing mapping from an ancestor environment to a new space environment.
    ///
    /// If the ancestor does not exist, returns `None` without adding a new mapping.
    /// If the ancestor exists, replaces the mapping and returns the previous space.
    ///
    /// # Arguments
    ///
    /// * `ancestor` - The ancestor environment name to replace.
    /// * `new_space` - The new space environment name.
    ///
    /// # Returns
    ///
    /// * `Some(String)` containing the previous space environment if the mapping was replaced.
    /// * `None` if the ancestor does not exist.
    ///
    /// # Example
    ///
    /// ```rust
    /// let previous = root_mapping.replace_mapping("prod1".to_string(), "staging".to_string());
    /// ```
    pub fn replace_mapping(&mut self, ancestor: String, new_space: String) -> Option<String> {
        // Remove the existing mapping if it exists.
        if let Some(existing_space) = self.ancestor_to_space.get_mut(&ancestor) {
            // Remove the ancestor from the old space's set.
            if let Some(ancestors) = self.space_to_ancestor.get_mut(existing_space) {
                ancestors.remove(&ancestor);
                if ancestors.is_empty() {
                    self.space_to_ancestor.remove(existing_space);
                }
            }

            // Update the ancestor_to_space with the new space.
            let previous_space = self
                .ancestor_to_space
                .insert(ancestor.clone(), new_space.clone());

            // Add the ancestor to the new space's set.
            self.space_to_ancestor
                .entry(new_space)
                .or_insert_with(HashSet::new)
                .insert(ancestor.clone());

            // Return the previous space.
            previous_space
        } else {
            // Ancestor does not exist; do not add a new mapping.
            None
        }
    }

    /// Removes a mapping by the ancestor environment.
    ///
    /// # Arguments
    ///
    /// * `ancestor` - The ancestor environment name to remove.
    ///
    /// # Returns
    ///
    /// `true` if the mapping was found and removed, `false` otherwise.
    ///
    /// # Example
    ///
    /// ```rust
    /// let removed = root_mapping.remove_mapping_by_ancestor("prod1".to_string());
    /// ```
    pub fn remove_mapping_by_ancestor(&mut self, ancestor: &String) -> bool {
        if let Some(space) = self.ancestor_to_space.remove(ancestor) {
            if let Some(ancestors) = self.space_to_ancestor.get_mut(&space) {
                ancestors.remove(ancestor);
                if ancestors.is_empty() {
                    self.space_to_ancestor.remove(&space);
                }
            }
            true
        } else {
            false
        }
    }

    /// Removes all mappings associated with a space environment.
    ///
    /// # Arguments
    ///
    /// * `space` - The space environment name to remove.
    ///
    /// # Returns
    ///
    /// `true` if the space was found and removed, `false` otherwise.
    ///
    /// # Example
    ///
    /// ```rust
    /// let removed = root_mapping.remove_mapping_by_space("prod".to_string());
    /// ```
    pub fn remove_mapping_by_space(&mut self, space: &String) -> bool {
        if let Some(ancestors) = self.space_to_ancestor.remove(space) {
            for ancestor in ancestors {
                self.ancestor_to_space.remove(&ancestor);
            }
            true
        } else {
            false
        }
    }

    /// Retrieves the space environment associated with a given ancestor environment.
    ///
    /// # Arguments
    ///
    /// * `ancestor` - The ancestor environment name.
    ///
    /// # Returns
    ///
    /// `Some(&String)` if the ancestor exists, `None` otherwise.
    ///
    /// # Example
    ///
    /// ```rust
    /// if let Some(space) = root_mapping.get_space(&"prod1".to_string()) {
    ///     println!("prod1 maps to {}", space);
    /// }
    /// ```
    pub fn get_space(&self, ancestor: &String) -> Option<&String> {
        self.ancestor_to_space.get(ancestor)
    }

    /// Retrieves all ancestor environments associated with a given space environment.
    ///
    /// # Arguments
    ///
    /// * `space` - The space environment name.
    ///
    /// # Returns
    ///
    /// `Some(&HashSet<String>)` if the space exists, `None` otherwise.
    ///
    /// # Example
    ///
    /// ```rust
    /// if let Some(ancestors) = root_mapping.get_ancestors(&"prod".to_string()) {
    ///     for ancestor in ancestors {
    ///         println!("prod is mapped by {}", ancestor);
    ///     }
    /// }
    /// ```
    pub fn get_ancestors(&self, space: &String) -> &HashSet<String> {
        self.space_to_ancestor
            .get(space)
            .unwrap_or(&DEFAULT_ANCESTOR_SET)
    }

    /// Lists all ancestor to space mappings.
    ///
    /// # Returns
    ///
    /// A reference to the internal `HashMap` of ancestor to space mappings.
    ///
    /// # Example
    ///
    /// ```rust
    /// for (ancestor, space) in root_mapping.list_ancestor_to_space() {
    ///     println!("{} -> {}", ancestor, space);
    /// }
    /// ```
    pub fn list_ancestor_to_space(&self) -> &HashMap<String, String> {
        &self.ancestor_to_space
    }

    /// Lists all space to ancestor mappings.
    ///
    /// # Returns
    ///
    /// A reference to the internal `HashMap` of space to ancestor mappings.
    ///
    /// # Example
    ///
    /// ```rust
    /// for (space, ancestors) in root_mapping.list_space_to_ancestor() {
    ///     println!("{} -> {:?}", space, ancestors);
    /// }
    /// ```
    pub fn list_space_to_ancestor(&self) -> &HashMap<String, HashSet<String>> {
        &self.space_to_ancestor
    }

    /// Checks if an ancestor environment exists in the mapping.
    ///
    /// # Arguments
    ///
    /// * `ancestor` - The ancestor environment name to check.
    ///
    /// # Returns
    ///
    /// `true` if the ancestor exists, `false` otherwise.
    ///
    /// # Example
    ///
    /// ```rust
    /// if root_mapping.contains_ancestor(&"prod1".to_string()) {
    ///     println!("prod1 exists in the mapping.");
    /// }
    /// ```
    pub fn contains_ancestor(&self, ancestor: &String) -> bool {
        self.ancestor_to_space.contains_key(ancestor)
    }

    /// Checks if a space environment exists in the mapping.
    ///
    /// # Arguments
    ///
    /// * `space` - The space environment name to check.
    ///
    /// # Returns
    ///
    /// `true` if the space exists, `false` otherwise.
    ///
    /// # Example
    ///
    /// ```rust
    /// if root_mapping.contains_space(&"prod".to_string()) {
    ///     println!("prod space exists in the mapping.");
    /// }
    /// ```
    pub fn contains_space(&self, space: &String) -> bool {
        self.space_to_ancestor.contains_key(space)
    }

    /// Clears all mappings.
    ///
    /// # Example
    ///
    /// ```rust
    /// root_mapping.clear();
    /// ```
    pub fn clear(&mut self) {
        self.space_to_ancestor.clear();
        self.ancestor_to_space.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_mapping_success() {
        let mut mapping = AncestorMapping::new();
        assert!(mapping
            .add_mapping("dev".to_string(), "dev".to_string())
            .is_ok());
        assert!(mapping
            .add_mapping("test".to_string(), "test".to_string())
            .is_ok());
        assert!(mapping
            .add_mapping("prod1".to_string(), "prod".to_string())
            .is_ok());
        assert!(mapping
            .add_mapping("prod2".to_string(), "prod".to_string())
            .is_ok());

        assert_eq!(
            mapping.get_space(&"dev".to_string()),
            Some(&"dev".to_string())
        );
        assert_eq!(
            mapping.get_space(&"prod1".to_string()),
            Some(&"prod".to_string())
        );
        assert_eq!(
            mapping.get_space(&"prod2".to_string()),
            Some(&"prod".to_string())
        );
        assert_eq!(
            mapping.get_space(&"test".to_string()),
            Some(&"test".to_string())
        );

        let prod_ancestors = mapping.get_ancestors(&"prod".to_string());
        assert!(prod_ancestors.contains("prod1"));
        assert!(prod_ancestors.contains("prod2"));
        assert_eq!(prod_ancestors.len(), 2);
    }

    #[test]
    fn test_add_mapping_duplicate() {
        let mut mapping = AncestorMapping::new();
        assert!(mapping
            .add_mapping("prod1".to_string(), "prod".to_string())
            .is_ok());

        // Attempting to add the same ancestor again should fail.
        let result = mapping.add_mapping("prod1".to_string(), "staging".to_string());
        assert!(matches!(
            result,
            Err(RootMappingError::DuplicateAncestor(_))
        ));
        if let Err(RootMappingError::DuplicateAncestor(ancestor)) = result {
            assert_eq!(ancestor, "prod1");
        }

        // Ensure the original mapping remains unchanged.
        assert_eq!(
            mapping.get_space(&"prod1".to_string()),
            Some(&"prod".to_string())
        );
    }

    #[test]
    fn test_replace_mapping_existing() {
        let mut mapping = AncestorMapping::new();
        mapping
            .add_mapping("prod1".to_string(), "prod".to_string())
            .unwrap();
        mapping
            .add_mapping("prod2".to_string(), "prod".to_string())
            .unwrap();

        // Replace prod1's mapping from "prod" to "staging"
        let previous = mapping.replace_mapping("prod1".to_string(), "staging".to_string());
        assert_eq!(previous, Some("prod".to_string()));

        // Verify the new mapping
        assert_eq!(
            mapping.get_space(&"prod1".to_string()),
            Some(&"staging".to_string())
        );

        // Verify the old space no longer contains prod1
        let prod_ancestors = mapping.get_ancestors(&"prod".to_string());
        assert!(!prod_ancestors.contains("prod1"));
        assert!(prod_ancestors.contains("prod2"));
        assert_eq!(prod_ancestors.len(), 1);

        // Verify the new space contains prod1
        let staging_ancestors = mapping.get_ancestors(&"staging".to_string());
        assert!(staging_ancestors.contains("prod1"));
        assert_eq!(staging_ancestors.len(), 1);
    }

    #[test]
    fn test_replace_mapping_nonexistent() {
        let mut mapping = AncestorMapping::new();

        // Attempt to replace a non-existent ancestor
        let previous = mapping.replace_mapping("nonexistent".to_string(), "staging".to_string());
        assert_eq!(previous, None);

        // Ensure no new mapping was added
        assert!(!mapping.contains_ancestor(&"nonexistent".to_string()));
        assert!(!mapping.contains_space(&"staging".to_string()));
    }

    #[test]
    fn test_remove_mapping_by_ancestor() {
        let mut mapping = AncestorMapping::new();
        mapping
            .add_mapping("prod1".to_string(), "prod".to_string())
            .unwrap();
        mapping
            .add_mapping("prod2".to_string(), "prod".to_string())
            .unwrap();

        assert!(mapping.remove_mapping_by_ancestor(&"prod1".to_string()));
        assert!(!mapping.contains_ancestor(&"prod1".to_string()));
        let prod_ancestors = mapping.get_ancestors(&"prod".to_string());
        assert!(!prod_ancestors.contains("prod1"));
        assert!(prod_ancestors.contains("prod2"));

        // Remove the last ancestor mapping for "prod"
        assert!(mapping.remove_mapping_by_ancestor(&"prod2".to_string()));
        assert!(!mapping.contains_space(&"prod".to_string()));
    }

    #[test]
    fn test_remove_mapping_by_space() {
        let mut mapping = AncestorMapping::new();
        mapping
            .add_mapping("dev".to_string(), "dev".to_string())
            .unwrap();
        mapping
            .add_mapping("prod1".to_string(), "prod".to_string())
            .unwrap();
        mapping
            .add_mapping("prod2".to_string(), "prod".to_string())
            .unwrap();

        assert!(mapping.remove_mapping_by_space(&"prod".to_string()));
        assert!(!mapping.contains_space(&"prod".to_string()));
        assert!(!mapping.contains_ancestor(&"prod1".to_string()));
        assert!(!mapping.contains_ancestor(&"prod2".to_string()));

        // Attempt to remove a non-existent space
        assert!(!mapping.remove_mapping_by_space(&"staging".to_string()));
    }

    #[test]
    fn test_clear_mappings() {
        let mut mapping = AncestorMapping::new();
        mapping
            .add_mapping("dev".to_string(), "dev".to_string())
            .unwrap();
        mapping
            .add_mapping("test".to_string(), "test".to_string())
            .unwrap();
        mapping
            .add_mapping("prod1".to_string(), "prod".to_string())
            .unwrap();

        mapping.clear();
        assert!(mapping.list_ancestor_to_space().is_empty());
        assert!(mapping.list_space_to_ancestor().is_empty());
    }

    #[test]
    fn test_contains_methods() {
        let mut mapping = AncestorMapping::new();
        mapping
            .add_mapping("dev".to_string(), "dev".to_string())
            .unwrap();
        mapping
            .add_mapping("prod1".to_string(), "prod".to_string())
            .unwrap();

        assert!(mapping.contains_ancestor(&"dev".to_string()));
        assert!(mapping.contains_ancestor(&"prod1".to_string()));
        assert!(!mapping.contains_ancestor(&"test".to_string()));

        assert!(mapping.contains_space(&"dev".to_string()));
        assert!(mapping.contains_space(&"prod".to_string()));
        assert!(!mapping.contains_space(&"test".to_string()));
    }
}
