use std::path::PathBuf;

/// Trait for merging configuration values with CLI precedence.
/// 
/// This trait provides type-safe merging of CLI values, configuration file values,
/// and default values, with CLI values taking highest precedence.
pub trait ConfigMerger<T> {
    /// Merge CLI value, file value, and default value with CLI precedence.
    /// 
    /// # Arguments
    /// 
    /// * `cli_value` - Value from CLI arguments (highest precedence)
    /// * `file_value` - Value from configuration file (medium precedence)
    /// * `default_value` - Default value (lowest precedence)
    /// 
    /// # Returns
    /// 
    /// The merged value according to precedence rules
    fn merge(cli_value: T, file_value: Option<T>, default_value: T) -> T;
}

/// Merger implementation for Vec<String> fields.
/// 
/// For vectors, CLI values take precedence if non-empty, otherwise use file values,
/// falling back to default (empty vector).
impl ConfigMerger<Vec<String>> for Vec<String> {
    fn merge(cli_value: Vec<String>, file_value: Option<Vec<String>>, _default_value: Vec<String>) -> Vec<String> {
        if !cli_value.is_empty() {
            cli_value
        } else {
            file_value.unwrap_or_default()
        }
    }
}

/// Merger implementation for Vec<PathBuf> fields.
/// 
/// Similar to Vec<String> but for PathBuf vectors.
impl ConfigMerger<Vec<PathBuf>> for Vec<PathBuf> {
    fn merge(cli_value: Vec<PathBuf>, file_value: Option<Vec<PathBuf>>, _default_value: Vec<PathBuf>) -> Vec<PathBuf> {
        if !cli_value.is_empty() {
            cli_value
        } else {
            file_value.unwrap_or_default()
        }
    }
}

/// Merger implementation for String fields with default values.
/// 
/// For strings, CLI values take precedence if different from default,
/// otherwise use file values, falling back to default.
impl ConfigMerger<String> for String {
    fn merge(cli_value: String, file_value: Option<String>, default_value: String) -> String {
        if cli_value != default_value {
            cli_value
        } else {
            file_value.unwrap_or(default_value)
        }
    }
}

/// Merger implementation for Option<T> fields.
/// 
/// For optional fields, CLI values take precedence if Some,
/// otherwise use file values (which may also be None).
impl<T> ConfigMerger<Option<T>> for Option<T> {
    fn merge(cli_value: Option<T>, file_value: Option<Option<T>>, _default_value: Option<T>) -> Option<T> {
        cli_value.or_else(|| file_value.flatten())
    }
}

/// Merger implementation for boolean fields with Option<bool> CLI values.
/// 
/// For booleans, CLI values take precedence if Some,
/// otherwise use file values, falling back to default.
pub struct BoolMerger;

impl BoolMerger {
    pub fn merge(cli_value: Option<bool>, file_value: Option<bool>, default_value: bool) -> bool {
        cli_value.or(file_value).unwrap_or(default_value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vec_string_merger() {
        // CLI value takes precedence when non-empty
        let result = Vec::<String>::merge(
            vec!["cli".to_string()],
            Some(vec!["file".to_string()]),
            vec!["default".to_string()]
        );
        assert_eq!(result, vec!["cli"]);

        // File value used when CLI is empty
        let result = Vec::<String>::merge(
            vec![],
            Some(vec!["file".to_string()]),
            vec!["default".to_string()]
        );
        assert_eq!(result, vec!["file"]);

        // Default used when both CLI and file are empty/None
        let result = Vec::<String>::merge(
            vec![],
            None,
            vec!["default".to_string()]
        );
        assert_eq!(result, Vec::<String>::new());
    }

    #[test]
    fn test_string_merger() {
        // CLI value takes precedence when different from default
        let result = String::merge(
            "cli_value".to_string(),
            Some("file_value".to_string()),
            "default".to_string()
        );
        assert_eq!(result, "cli_value");

        // File value used when CLI equals default
        let result = String::merge(
            "default".to_string(),
            Some("file_value".to_string()),
            "default".to_string()
        );
        assert_eq!(result, "file_value");

        // Default used when CLI equals default and no file value
        let result = String::merge(
            "default".to_string(),
            None,
            "default".to_string()
        );
        assert_eq!(result, "default");
    }

    #[test]
    fn test_option_merger() {
        // CLI value takes precedence when Some
        let result = Option::<String>::merge(
            Some("cli".to_string()),
            Some(Some("file".to_string())),
            None
        );
        assert_eq!(result, Some("cli".to_string()));

        // File value used when CLI is None
        let result = Option::<String>::merge(
            None,
            Some(Some("file".to_string())),
            None
        );
        assert_eq!(result, Some("file".to_string()));

        // None when both CLI and file are None
        let result = Option::<String>::merge(
            None,
            None,
            None
        );
        assert_eq!(result, None);
    }

    #[test]
    fn test_bool_merger() {
        // CLI value takes precedence when Some
        let result = BoolMerger::merge(
            Some(true),
            Some(false),
            false
        );
        assert_eq!(result, true);

        // File value used when CLI is None
        let result = BoolMerger::merge(
            None,
            Some(true),
            false
        );
        assert_eq!(result, true);

        // Default used when both CLI and file are None
        let result = BoolMerger::merge(
            None,
            None,
            true
        );
        assert_eq!(result, true);
    }
}