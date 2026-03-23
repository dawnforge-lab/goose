//! Skill CRUD with YAML file storage.
//!
//! Skills are stored as individual YAML files in `~/.spawnbot/skills/`.

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillInfo {
    pub name: String,
    #[serde(rename = "type")]
    pub skill_type: String,
    pub description: String,
    pub definition: String,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

fn default_enabled() -> bool {
    true
}

pub fn create_skill(skills_dir: &Path, skill: &SkillInfo) -> Result<()> {
    std::fs::create_dir_all(skills_dir)?;
    let path = skills_dir.join(format!("{}.yaml", skill.name));
    if path.exists() {
        bail!("Skill '{}' already exists", skill.name);
    }
    let content = serde_yaml::to_string(skill)?;
    std::fs::write(&path, content)?;
    Ok(())
}

pub fn list_skills(skills_dir: &Path) -> Result<Vec<SkillInfo>> {
    if !skills_dir.exists() {
        return Ok(vec![]);
    }
    let mut skills = Vec::new();
    for entry in std::fs::read_dir(skills_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().map_or(false, |e| e == "yaml" || e == "yml") {
            let content = std::fs::read_to_string(&path)?;
            let skill: SkillInfo = serde_yaml::from_str(&content)?;
            skills.push(skill);
        }
    }
    skills.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(skills)
}

pub fn read_skill(skills_dir: &Path, name: &str) -> Result<SkillInfo> {
    let path = skills_dir.join(format!("{}.yaml", name));
    let content = std::fs::read_to_string(&path)
        .with_context(|| format!("Skill '{}' not found", name))?;
    Ok(serde_yaml::from_str(&content)?)
}

pub fn edit_skill(skills_dir: &Path, name: &str, definition: &str) -> Result<()> {
    let path = skills_dir.join(format!("{}.yaml", name));
    let content = std::fs::read_to_string(&path)
        .with_context(|| format!("Skill '{}' not found", name))?;
    let mut skill: SkillInfo = serde_yaml::from_str(&content)?;
    skill.definition = definition.to_string();
    let updated = serde_yaml::to_string(&skill)?;
    std::fs::write(&path, updated)?;
    Ok(())
}

pub fn delete_skill(skills_dir: &Path, name: &str) -> Result<()> {
    let path = skills_dir.join(format!("{}.yaml", name));
    if !path.exists() {
        bail!("Skill '{}' not found", name);
    }
    std::fs::remove_file(&path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_skill() -> SkillInfo {
        SkillInfo {
            name: "memory-management".to_string(),
            skill_type: "routine".to_string(),
            description: "Manages memory consolidation".to_string(),
            definition: "When idle, review recent interactions and store important memories...".to_string(),
            enabled: true,
        }
    }

    #[test]
    fn create_and_read_skill() {
        let dir = tempfile::tempdir().unwrap();
        let skills_dir = dir.path().join("skills");

        let skill = test_skill();
        create_skill(&skills_dir, &skill).unwrap();

        // Verify file was created
        let path = skills_dir.join("memory-management.yaml");
        assert!(path.exists());

        // Read it back
        let read = read_skill(&skills_dir, "memory-management").unwrap();
        assert_eq!(read.name, "memory-management");
        assert_eq!(read.skill_type, "routine");
        assert_eq!(read.description, "Manages memory consolidation");
        assert!(read.enabled);
    }

    #[test]
    fn create_duplicate_skill_fails() {
        let dir = tempfile::tempdir().unwrap();
        let skills_dir = dir.path().join("skills");

        let skill = test_skill();
        create_skill(&skills_dir, &skill).unwrap();

        let err = create_skill(&skills_dir, &skill).unwrap_err();
        assert!(err.to_string().contains("already exists"));
    }

    #[test]
    fn list_skills_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        let skills_dir = dir.path().join("skills");

        let skills = list_skills(&skills_dir).unwrap();
        assert!(skills.is_empty());
    }

    #[test]
    fn list_skills_returns_sorted() {
        let dir = tempfile::tempdir().unwrap();
        let skills_dir = dir.path().join("skills");

        let mut skill_a = test_skill();
        skill_a.name = "alpha".to_string();
        create_skill(&skills_dir, &skill_a).unwrap();

        let mut skill_b = test_skill();
        skill_b.name = "beta".to_string();
        create_skill(&skills_dir, &skill_b).unwrap();

        let skills = list_skills(&skills_dir).unwrap();
        assert_eq!(skills.len(), 2);
        assert_eq!(skills[0].name, "alpha");
        assert_eq!(skills[1].name, "beta");
    }

    #[test]
    fn edit_skill_definition() {
        let dir = tempfile::tempdir().unwrap();
        let skills_dir = dir.path().join("skills");

        let skill = test_skill();
        create_skill(&skills_dir, &skill).unwrap();

        edit_skill(&skills_dir, "memory-management", "New definition content").unwrap();

        let read = read_skill(&skills_dir, "memory-management").unwrap();
        assert_eq!(read.definition, "New definition content");
        // Other fields should be preserved
        assert_eq!(read.skill_type, "routine");
    }

    #[test]
    fn edit_nonexistent_skill_fails() {
        let dir = tempfile::tempdir().unwrap();
        let skills_dir = dir.path().join("skills");
        std::fs::create_dir_all(&skills_dir).unwrap();

        let err = edit_skill(&skills_dir, "nope", "whatever").unwrap_err();
        assert!(err.to_string().contains("not found"));
    }

    #[test]
    fn delete_skill_removes_file() {
        let dir = tempfile::tempdir().unwrap();
        let skills_dir = dir.path().join("skills");

        let skill = test_skill();
        create_skill(&skills_dir, &skill).unwrap();

        let path = skills_dir.join("memory-management.yaml");
        assert!(path.exists());

        delete_skill(&skills_dir, "memory-management").unwrap();
        assert!(!path.exists());
    }

    #[test]
    fn delete_nonexistent_skill_fails() {
        let dir = tempfile::tempdir().unwrap();
        let skills_dir = dir.path().join("skills");
        std::fs::create_dir_all(&skills_dir).unwrap();

        let err = delete_skill(&skills_dir, "nope").unwrap_err();
        assert!(err.to_string().contains("not found"));
    }

    #[test]
    fn read_nonexistent_skill_fails() {
        let dir = tempfile::tempdir().unwrap();
        let skills_dir = dir.path().join("skills");
        std::fs::create_dir_all(&skills_dir).unwrap();

        let err = read_skill(&skills_dir, "nope").unwrap_err();
        assert!(err.to_string().contains("not found"));
    }
}
