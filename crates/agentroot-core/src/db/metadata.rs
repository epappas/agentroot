//! User-defined metadata system
//!
//! Supports typed metadata fields that users can add manually to documents.
//! This complements the LLM-generated metadata and enables rich filtering/querying.

use crate::error::{AgentRootError, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Metadata value types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", content = "value")]
pub enum MetadataValue {
    /// Text string
    Text(String),

    /// Integer number
    Integer(i64),

    /// Floating point number
    Float(f64),

    /// Boolean value
    Boolean(bool),

    /// ISO 8601 timestamp
    DateTime(String),

    /// Array of strings (tags, labels)
    Tags(Vec<String>),

    /// Enum value (predefined set of options)
    Enum { value: String, options: Vec<String> },

    /// Qualitative measure (e.g., "high", "medium", "low")
    Qualitative { value: String, scale: Vec<String> },

    /// Quantitative measure with unit
    Quantitative { value: f64, unit: String },

    /// JSON object for complex nested data
    Json(serde_json::Value),
}

impl MetadataValue {
    /// Create a datetime value from timestamp
    pub fn datetime_now() -> Self {
        MetadataValue::DateTime(Utc::now().to_rfc3339())
    }

    /// Create a datetime value from a DateTime
    pub fn datetime(dt: DateTime<Utc>) -> Self {
        MetadataValue::DateTime(dt.to_rfc3339())
    }

    /// Create tags from strings
    pub fn tags<I, S>(iter: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        MetadataValue::Tags(iter.into_iter().map(|s| s.into()).collect())
    }

    /// Create enum with validation
    pub fn enum_value(value: impl Into<String>, options: Vec<String>) -> Result<Self> {
        let value = value.into();
        if !options.contains(&value) {
            return Err(AgentRootError::InvalidInput(format!(
                "Invalid enum value '{}'. Must be one of: {:?}",
                value, options
            )));
        }
        Ok(MetadataValue::Enum { value, options })
    }

    /// Create qualitative value
    pub fn qualitative(value: impl Into<String>, scale: Vec<String>) -> Result<Self> {
        let value = value.into();
        if !scale.contains(&value) {
            return Err(AgentRootError::InvalidInput(format!(
                "Invalid qualitative value '{}'. Must be one of: {:?}",
                value, scale
            )));
        }
        Ok(MetadataValue::Qualitative { value, scale })
    }

    /// Create quantitative value
    pub fn quantitative(value: f64, unit: impl Into<String>) -> Self {
        MetadataValue::Quantitative {
            value,
            unit: unit.into(),
        }
    }
}

/// User-defined metadata for a document
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UserMetadata {
    /// Field name -> value mapping
    pub fields: HashMap<String, MetadataValue>,
}

impl UserMetadata {
    /// Create empty metadata
    pub fn new() -> Self {
        Self {
            fields: HashMap::new(),
        }
    }

    /// Add a metadata field
    pub fn add(&mut self, key: impl Into<String>, value: MetadataValue) -> &mut Self {
        self.fields.insert(key.into(), value);
        self
    }

    /// Get a metadata field
    pub fn get(&self, key: &str) -> Option<&MetadataValue> {
        self.fields.get(key)
    }

    /// Remove a metadata field
    pub fn remove(&mut self, key: &str) -> Option<MetadataValue> {
        self.fields.remove(key)
    }

    /// Check if a field exists
    pub fn contains(&self, key: &str) -> bool {
        self.fields.contains_key(key)
    }

    /// Serialize to JSON string
    pub fn to_json(&self) -> Result<String> {
        serde_json::to_string(&self.fields).map_err(|e| e.into())
    }

    /// Deserialize from JSON string
    pub fn from_json(json: &str) -> Result<Self> {
        let fields = serde_json::from_str(json)?;
        Ok(Self { fields })
    }

    /// Merge with another metadata instance (other takes precedence)
    pub fn merge(&mut self, other: &UserMetadata) {
        for (key, value) in &other.fields {
            self.fields.insert(key.clone(), value.clone());
        }
    }
}

/// Builder for constructing metadata
pub struct MetadataBuilder {
    metadata: UserMetadata,
}

impl MetadataBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self {
            metadata: UserMetadata::new(),
        }
    }

    /// Add text field
    pub fn text(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.add(key, MetadataValue::Text(value.into()));
        self
    }

    /// Add integer field
    pub fn integer(mut self, key: impl Into<String>, value: i64) -> Self {
        self.metadata.add(key, MetadataValue::Integer(value));
        self
    }

    /// Add float field
    pub fn float(mut self, key: impl Into<String>, value: f64) -> Self {
        self.metadata.add(key, MetadataValue::Float(value));
        self
    }

    /// Add boolean field
    pub fn boolean(mut self, key: impl Into<String>, value: bool) -> Self {
        self.metadata.add(key, MetadataValue::Boolean(value));
        self
    }

    /// Add datetime field (now)
    pub fn datetime_now(mut self, key: impl Into<String>) -> Self {
        self.metadata.add(key, MetadataValue::datetime_now());
        self
    }

    /// Add datetime field
    pub fn datetime(mut self, key: impl Into<String>, dt: DateTime<Utc>) -> Self {
        self.metadata.add(key, MetadataValue::datetime(dt));
        self
    }

    /// Add tags field
    pub fn tags<I, S>(mut self, key: impl Into<String>, tags: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.metadata.add(key, MetadataValue::tags(tags));
        self
    }

    /// Add enum field
    pub fn enum_value(
        mut self,
        key: impl Into<String>,
        value: impl Into<String>,
        options: Vec<String>,
    ) -> Result<Self> {
        let metadata_value = MetadataValue::enum_value(value, options)?;
        self.metadata.add(key, metadata_value);
        Ok(self)
    }

    /// Add qualitative field
    pub fn qualitative(
        mut self,
        key: impl Into<String>,
        value: impl Into<String>,
        scale: Vec<String>,
    ) -> Result<Self> {
        let metadata_value = MetadataValue::qualitative(value, scale)?;
        self.metadata.add(key, metadata_value);
        Ok(self)
    }

    /// Add quantitative field
    pub fn quantitative(
        mut self,
        key: impl Into<String>,
        value: f64,
        unit: impl Into<String>,
    ) -> Self {
        self.metadata
            .add(key, MetadataValue::quantitative(value, unit));
        self
    }

    /// Add JSON field
    pub fn json(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.add(key, MetadataValue::Json(value));
        self
    }

    /// Build the metadata
    pub fn build(self) -> UserMetadata {
        self.metadata
    }
}

impl Default for MetadataBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Metadata query filter
#[derive(Debug, Clone)]
pub enum MetadataFilter {
    /// Text equals
    TextEq(String, String),

    /// Text contains
    TextContains(String, String),

    /// Integer comparison
    IntegerEq(String, i64),
    IntegerGt(String, i64),
    IntegerLt(String, i64),
    IntegerRange(String, i64, i64),

    /// Float comparison
    FloatEq(String, f64),
    FloatGt(String, f64),
    FloatLt(String, f64),
    FloatRange(String, f64, f64),

    /// Boolean equals
    BooleanEq(String, bool),

    /// DateTime comparison
    DateTimeAfter(String, String),
    DateTimeBefore(String, String),
    DateTimeRange(String, String, String),

    /// Tags contains
    TagsContain(String, String),
    TagsContainAll(String, Vec<String>),
    TagsContainAny(String, Vec<String>),

    /// Enum equals
    EnumEq(String, String),

    /// Field exists
    Exists(String),

    /// AND combination
    And(Vec<MetadataFilter>),

    /// OR combination
    Or(Vec<MetadataFilter>),

    /// NOT
    Not(Box<MetadataFilter>),
}

impl MetadataFilter {
    /// Check if metadata matches this filter
    pub fn matches(&self, metadata: &UserMetadata) -> bool {
        match self {
            MetadataFilter::TextEq(key, value) => {
                matches!(metadata.get(key), Some(MetadataValue::Text(v)) if v == value)
            }
            MetadataFilter::TextContains(key, substring) => {
                matches!(metadata.get(key), Some(MetadataValue::Text(v)) if v.contains(substring))
            }
            MetadataFilter::IntegerEq(key, value) => {
                matches!(metadata.get(key), Some(MetadataValue::Integer(v)) if v == value)
            }
            MetadataFilter::IntegerGt(key, value) => {
                matches!(metadata.get(key), Some(MetadataValue::Integer(v)) if v > value)
            }
            MetadataFilter::IntegerLt(key, value) => {
                matches!(metadata.get(key), Some(MetadataValue::Integer(v)) if v < value)
            }
            MetadataFilter::IntegerRange(key, min, max) => {
                matches!(metadata.get(key), Some(MetadataValue::Integer(v)) if v >= min && v <= max)
            }
            MetadataFilter::BooleanEq(key, value) => {
                matches!(metadata.get(key), Some(MetadataValue::Boolean(v)) if v == value)
            }
            MetadataFilter::TagsContain(key, tag) => {
                matches!(metadata.get(key), Some(MetadataValue::Tags(tags)) if tags.contains(tag))
            }
            MetadataFilter::TagsContainAll(key, search_tags) => {
                matches!(metadata.get(key), Some(MetadataValue::Tags(tags))
                    if search_tags.iter().all(|t| tags.contains(t)))
            }
            MetadataFilter::TagsContainAny(key, search_tags) => {
                matches!(metadata.get(key), Some(MetadataValue::Tags(tags))
                    if search_tags.iter().any(|t| tags.contains(t)))
            }
            MetadataFilter::EnumEq(key, value) => {
                matches!(metadata.get(key), Some(MetadataValue::Enum { value: v, .. }) if v == value)
            }
            MetadataFilter::Exists(key) => metadata.contains(key),
            MetadataFilter::And(filters) => filters.iter().all(|f| f.matches(metadata)),
            MetadataFilter::Or(filters) => filters.iter().any(|f| f.matches(metadata)),
            MetadataFilter::Not(filter) => !filter.matches(metadata),
            _ => false, // Other filters need proper implementation
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metadata_builder() {
        let metadata = MetadataBuilder::new()
            .text("author", "John Doe")
            .integer("version", 2)
            .float("score", 4.5)
            .boolean("published", true)
            .datetime_now("created_at")
            .tags("labels", vec!["rust", "programming"])
            .quantitative("size", 1024.0, "KB")
            .build();

        assert_eq!(
            metadata.get("author"),
            Some(&MetadataValue::Text("John Doe".to_string()))
        );
        assert_eq!(metadata.get("version"), Some(&MetadataValue::Integer(2)));
        assert!(metadata.contains("created_at"));
    }

    #[test]
    fn test_enum_validation() {
        let result =
            MetadataValue::enum_value("active", vec!["draft".to_string(), "published".to_string()]);
        assert!(result.is_err());

        let result = MetadataValue::enum_value(
            "published",
            vec!["draft".to_string(), "published".to_string()],
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_metadata_filter() {
        let metadata = MetadataBuilder::new()
            .text("status", "published")
            .tags("labels", vec!["rust", "tutorial"])
            .integer("views", 100)
            .build();

        assert!(
            MetadataFilter::TextEq("status".to_string(), "published".to_string())
                .matches(&metadata)
        );
        assert!(
            MetadataFilter::TagsContain("labels".to_string(), "rust".to_string())
                .matches(&metadata)
        );
        assert!(MetadataFilter::IntegerGt("views".to_string(), 50).matches(&metadata));
    }

    #[test]
    fn test_metadata_merge() {
        let mut meta1 = MetadataBuilder::new()
            .text("author", "Alice")
            .integer("version", 1)
            .build();

        let meta2 = MetadataBuilder::new()
            .text("author", "Bob")
            .tags("labels", vec!["rust"])
            .build();

        meta1.merge(&meta2);

        assert_eq!(
            meta1.get("author"),
            Some(&MetadataValue::Text("Bob".to_string()))
        );
        assert!(meta1.contains("labels"));
        assert!(meta1.contains("version"));
    }

    #[test]
    fn test_json_serialization() {
        let metadata = MetadataBuilder::new()
            .text("name", "Test")
            .integer("count", 42)
            .build();

        let json = metadata.to_json().unwrap();
        let restored = UserMetadata::from_json(&json).unwrap();

        assert_eq!(metadata.fields, restored.fields);
    }
}
