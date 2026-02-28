// Course definition types for parsing course-definition.yml

use serde::{Deserialize, Serialize};

/// Stage metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Stage {
    /// Stage slug.
    pub slug: String,

    /// Stage display name.
    pub name: String,

    /// Difficulty level.
    #[serde(default)]
    pub difficulty: String,

    /// Marketing description markdown.
    #[serde(default)]
    pub marketing_md: String,

    /// Associated concept slugs.
    #[serde(default)]
    pub concept_slugs: Vec<String>,

    /// Primary extension slug.
    #[serde(default)]
    pub primary_extension_slug: Option<String>,
}

/// Extension metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Extension {
    /// Extension slug.
    pub slug: String,

    /// Extension name.
    pub name: String,

    /// Extension description markdown.
    #[serde(default)]
    pub description_md: String,
}

/// Parsed `course-definition.yml` content.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CourseDefinition {
    /// Challenge slug.
    #[serde(default)]
    pub slug: String,

    /// Challenge name.
    #[serde(default)]
    pub name: String,

    /// Challenge short name.
    #[serde(default)]
    pub short_name: String,

    /// Stages.
    #[serde(default)]
    pub stages: Vec<Stage>,

    /// Extensions.
    #[serde(default)]
    pub extensions: Vec<Extension>,
}

impl CourseDefinition {
    /// Return first stage slug.
    #[must_use] 
    pub fn first_stage_slug(&self) -> Option<String> {
        self.stages.first().map(|s| s.slug.clone())
    }

    /// Find stage by slug.
    #[must_use] 
    pub fn get_stage(&self, slug: &str) -> Option<&Stage> {
        self.stages.iter().find(|s| s.slug == slug)
    }

    /// Return all stage slugs.
    #[must_use] 
    pub fn stage_slugs(&self) -> Vec<String> {
        self.stages.iter().map(|s| s.slug.clone()).collect()
    }

    /// Return extension name by slug.
    #[must_use] 
    pub fn get_extension_name(&self, slug: &str) -> Option<String> {
        self.extensions
            .iter()
            .find(|e| e.slug == slug)
            .map(|e| e.name.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_course_definition_parsing() {
        let yaml = r#"
slug: "shell"
name: "Build your own Shell"
short_name: "Shell"
stages:
  - slug: "oo8"
    name: "Print a prompt"
    difficulty: very_easy
    marketing_md: "In this stage, you'll implement printing the shell prompt."
  - slug: "cz2"
    name: "Handle invalid commands"
    difficulty: easy
    marketing_md: "In this stage, you'll implement handling invalid commands."
"#;

        let course: CourseDefinition = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(course.slug, "shell");
        assert_eq!(course.stages.len(), 2);
        assert_eq!(course.first_stage_slug(), Some("oo8".to_string()));
        assert_eq!(course.get_stage("oo8").unwrap().name, "Print a prompt");
    }
}
