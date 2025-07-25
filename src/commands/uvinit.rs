use anyhow::{Context, Result};
use std::cmp::Ordering;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;
use toml_edit::{Array, DocumentMut, Key};

use crate::config::*;

static REPLACE_KEY_VER: LazyLock<Key> = LazyLock::new(|| Key::new("version"));
static REPLACE_KEY_DYN: LazyLock<Key> = LazyLock::new(|| Key::new("dynamic"));

// UV init specific functions
fn find_pyproject_files<P: AsRef<Path>>(root_dir: P, skip_dirs: &[String]) -> Result<Vec<PathBuf>> {
    let mut pyproject_files = Vec::new();
    find_pyproject_files_recursive(root_dir.as_ref(), &mut pyproject_files, skip_dirs)?;
    Ok(pyproject_files)
}

fn find_pyproject_files_recursive(
    dir: &Path,
    files: &mut Vec<PathBuf>,
    skip_dirs: &[String],
) -> Result<()> {
    if !dir.is_dir() {
        return Ok(());
    }

    let entries = fs::read_dir(dir)
        .with_context(|| format!("Failed to read directory: {}", dir.display()))?;

    for entry in entries {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() && path.file_name() == Some("pyproject.toml".as_ref()) {
            files.push(path);
        } else if path.is_dir() {
            if let Some(dir_name) = path.file_name().and_then(|n| n.to_str()) {
                if !skip_dirs.contains(&dir_name.to_string()) {
                    find_pyproject_files_recursive(&path, files, skip_dirs)?;
                }
            }
        }
    }

    Ok(())
}

fn has_project_dynamic<P: AsRef<Path>>(file_path: P) -> Result<bool> {
    let file_path = file_path.as_ref();

    let content = fs::read_to_string(file_path)
        .with_context(|| format!("Failed to read file: {}", file_path.display()))?;

    let doc = content
        .parse::<DocumentMut>()
        .with_context(|| format!("Failed to parse TOML in: {}", file_path.display()))?;

    if let Some(project) = doc.get("project") {
        if let Some(project_table) = project.as_table() {
            return Ok(project_table.contains_key("dynamic"));
        }
    }

    Ok(false)
}

fn modify_pyproject_toml<P: AsRef<Path>>(file_path: P, config: &UvinitConfig) -> Result<()> {
    let file_path = file_path.as_ref();

    let content = fs::read_to_string(file_path)
        .with_context(|| format!("Failed to read file: {}", file_path.display()))?;

    let mut doc = content
        .parse::<DocumentMut>()
        .with_context(|| "Failed to parse TOML document")?;

    // 1. Replace project.version with project.dynamic = ["version"]
    if config.enable_dynamic_version {
        if let Some(project) = doc.get_mut("project") {
            if let Some(project_table) = project.as_table_mut() {
                let mut dynamic_array = Array::new();
                dynamic_array.push("version");
                project_table.insert("dynamic", toml_edit::value(dynamic_array));

                project_table.sort_values_by(|key1, _, key2, _| {
                    if key1 == &*REPLACE_KEY_DYN && key2 != &*REPLACE_KEY_VER {
                        Ordering::Less
                    } else {
                        Ordering::Equal
                    }
                });

                project_table.remove("version");
            }
        }
    }

    // 2. Add to build-system.requires
    if config.add_hatch_vcs || !config.additional_requires.is_empty() {
        let mut requires_to_add = Vec::new();

        if config.add_hatch_vcs {
            requires_to_add.push("hatch-vcs");
        }

        for req in &config.additional_requires {
            requires_to_add.push(req.as_str());
        }

        if let Some(build_system) = doc.get_mut("build-system") {
            if let Some(build_system_table) = build_system.as_table_mut() {
                let requires = build_system_table
                    .entry("requires")
                    .or_insert(toml_edit::value(Array::new()));

                if let Some(requires_array) = requires.as_array_mut() {
                    for req in requires_to_add {
                        let has_req = requires_array.iter().any(|v| v.as_str() == Some(req));

                        if !has_req {
                            requires_array.push(req);
                        }
                    }
                }
            }
        }
    }

    // 3. Add tool.hatch.version.source = "vcs"
    if config.enable_dynamic_version {
        if doc.get("tool").is_none() {
            doc.insert("tool", toml_edit::table());
        }

        if let Some(tool) = doc.get_mut("tool") {
            if let Some(tool_table) = tool.as_table_mut() {
                tool_table.set_implicit(true);
                if tool_table.get("hatch").is_none() {
                    tool_table.insert("hatch", toml_edit::table());
                }

                if let Some(hatch) = tool_table.get_mut("hatch") {
                    if let Some(hatch_table) = hatch.as_table_mut() {
                        hatch_table.set_implicit(true);
                        if hatch_table.get("version").is_none() {
                            hatch_table.insert("version", toml_edit::table());
                        }

                        if let Some(version) = hatch_table.get_mut("version") {
                            if let Some(version_table) = version.as_table_mut() {
                                version_table.set_implicit(true);
                                version_table.insert("source", toml_edit::value("vcs"));
                            }
                        }
                    }
                }
            }
        }
    }

    // 4. Add tool.pytest.ini_options.asyncio_mode = "auto"
    if config.enable_pytest_asyncio {
        if doc.get("tool").is_none() {
            doc.insert("tool", toml_edit::table());
        }

        if let Some(tool) = doc.get_mut("tool") {
            if let Some(tool_table) = tool.as_table_mut() {
                tool_table.set_implicit(true);
                if tool_table.get("pytest").is_none() {
                    tool_table.insert("pytest", toml_edit::table());
                }

                if let Some(pytest) = tool_table.get_mut("pytest") {
                    if let Some(pytest_table) = pytest.as_table_mut() {
                        pytest_table.set_implicit(true);
                        if pytest_table.get("ini_options").is_none() {
                            pytest_table.insert("ini_options", toml_edit::table());
                        }

                        if let Some(ini_options) = pytest_table.get_mut("ini_options") {
                            if let Some(ini_options_table) = ini_options.as_table_mut() {
                                ini_options_table.set_implicit(true);
                                ini_options_table.insert("asyncio_mode", toml_edit::value("auto"));
                            }
                        }
                    }
                }
            }
        }
    }

    // 5. Add tool.bandit
    if config.enable_bandit {
        if doc.get("tool").is_none() {
            doc.insert("tool", toml_edit::table());
        }

        if let Some(tool) = doc.get_mut("tool") {
            if let Some(tool_table) = tool.as_table_mut() {
                tool_table.set_implicit(true);
                if tool_table.get("bandit").is_none() {
                    tool_table.insert("bandit", toml_edit::table());
                }

                if let Some(bandit) = tool_table.get_mut("bandit") {
                    if let Some(bandit_table) = bandit.as_table_mut() {
                        // Add skips = ["B101"]
                        let skips_to_add = vec!["B101"];
                        let skips = bandit_table
                            .entry("skips")
                            .or_insert(toml_edit::value(Array::new()));

                        if let Some(skips_array) = skips.as_array_mut() {
                            for skip in skips_to_add {
                                let has_skip = skips_array.iter().any(|v| v.as_str() == Some(skip));

                                if !has_skip {
                                    skips_array.push(skip);
                                }
                            }
                        }

                        // Add exclude_dirs = ["tests", "venv"]
                        let exclude_dirs_to_add = vec![".venv", "venv", "tests"];
                        let exclude_dirs = bandit_table
                            .entry("exclude_dirs")
                            .or_insert(toml_edit::value(Array::new()));

                        if let Some(exclude_dirs_array) = exclude_dirs.as_array_mut() {
                            for exclude_dir in exclude_dirs_to_add {
                                let has_exclude_dir = exclude_dirs_array
                                    .iter()
                                    .any(|v| v.as_str() == Some(exclude_dir));

                                if !has_exclude_dir {
                                    exclude_dirs_array.push(exclude_dir);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    fs::write(file_path, doc.to_string())
        .with_context(|| format!("Failed to write file: {}", file_path.display()))?;

    Ok(())
}

pub fn run_uvinit(path: &Path, yes: bool) -> Result<()> {
    let config = load_config()?;
    let uvinit_config = &config.uvinit;

    println!(
        "🔍 Searching for pyproject.toml files in: {}",
        path.display()
    );

    let pyproject_files = find_pyproject_files(path, &uvinit_config.skip_dirs)?;

    if pyproject_files.is_empty() {
        println!("❌ No pyproject.toml files found.");
        return Ok(());
    }

    println!("📦 Found {} pyproject.toml file(s):", pyproject_files.len());

    let mut files_to_process = Vec::new();

    for file_path in &pyproject_files {
        println!("  {}", file_path.display());

        match has_project_dynamic(file_path) {
            Ok(true) => {
                println!("    ✅ Has project.dynamic - skipping");
            }
            Ok(false) => {
                println!("    ⚠️  No project.dynamic - needs processing");
                files_to_process.push(file_path);
            }
            Err(e) => {
                eprintln!("    ❌ Error checking {}: {}", file_path.display(), e);
            }
        }
    }

    if files_to_process.is_empty() {
        println!("✅ All files already have project.dynamic configured!");
        return Ok(());
    }

    if !yes {
        println!(
            "\n🔧 Will modify {} file(s). Continue? (y/N)",
            files_to_process.len()
        );
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        if !input.trim().to_lowercase().starts_with('y') {
            println!("❌ Cancelled.");
            return Ok(());
        }
    }

    println!("\n🔄 Processing files...");

    for file_path in files_to_process {
        match modify_pyproject_toml(file_path, uvinit_config) {
            Ok(()) => {
                println!("  ✅ {}", file_path.display());
            }
            Err(e) => {
                eprintln!("  ❌ {}: {}", file_path.display(), e);
            }
        }
    }

    println!("\n🎉 Done!");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_find_pyproject_files() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let root_path = temp_dir.path();

        // Create test directory structure
        let sub_dir = root_path.join("subdir");
        let skip_dir = root_path.join(".git");
        let nested_dir = sub_dir.join("nested");
        fs::create_dir_all(&nested_dir)?;
        fs::create_dir_all(&skip_dir)?;

        // Create pyproject.toml files
        let mut file1 = fs::File::create(root_path.join("pyproject.toml"))?;
        file1.write_all(b"[project]\nname = \"test1\"")?;

        let mut file2 = fs::File::create(sub_dir.join("pyproject.toml"))?;
        file2.write_all(b"[project]\nname = \"test2\"")?;

        let mut file3 = fs::File::create(nested_dir.join("pyproject.toml"))?;
        file3.write_all(b"[project]\nname = \"test3\"")?;

        // Create file in skip directory (should be ignored)
        let mut file4 = fs::File::create(skip_dir.join("pyproject.toml"))?;
        file4.write_all(b"[project]\nname = \"skip\"")?;

        let skip_dirs = vec![".git".to_string(), ".venv".to_string()];
        let files = find_pyproject_files(root_path, &skip_dirs)?;

        assert_eq!(files.len(), 3);
        assert!(
            files
                .iter()
                .any(|f| f.file_name() == Some("pyproject.toml".as_ref()))
        );
        assert!(!files.iter().any(|f| f.to_string_lossy().contains(".git")));

        Ok(())
    }

    #[test]
    fn test_has_project_dynamic() -> Result<()> {
        let temp_dir = TempDir::new()?;

        // Test file with dynamic field
        let file_with_dynamic = temp_dir.path().join("with_dynamic.toml");
        fs::write(
            &file_with_dynamic,
            r#"
[project]
name = "test"
dynamic = ["version"]
description = "Test project"
"#,
        )?;

        // Test file without dynamic field
        let file_without_dynamic = temp_dir.path().join("without_dynamic.toml");
        fs::write(
            &file_without_dynamic,
            r#"
[project]
name = "test"
version = "0.1.0"
description = "Test project"
"#,
        )?;

        // Test file with empty project section
        let file_empty_project = temp_dir.path().join("empty_project.toml");
        fs::write(
            &file_empty_project,
            r#"
[project]
name = "test"
"#,
        )?;

        // Test file without project section
        let file_no_project = temp_dir.path().join("no_project.toml");
        fs::write(
            &file_no_project,
            r#"
[build-system]
requires = ["hatchling"]
"#,
        )?;

        assert!(has_project_dynamic(&file_with_dynamic)?);
        assert!(!has_project_dynamic(&file_without_dynamic)?);
        assert!(!has_project_dynamic(&file_empty_project)?);
        assert!(!has_project_dynamic(&file_no_project)?);

        Ok(())
    }

    #[test]
    fn test_modify_pyproject_toml() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let test_file = temp_dir.path().join("test_pyproject.toml");

        // Create initial pyproject.toml
        fs::write(
            &test_file,
            r#"
[project]
name = "test-project"
version = "0.1.0"
description = "A test project"
dependencies = ["requests"]

[build-system]
requires = ["hatchling"]
build-backend = "hatchling.build"
"#,
        )?;

        let config = UvinitConfig {
            additional_requires: vec!["setuptools-scm".to_string()],
            ..Default::default()
        };

        // Modify the file
        modify_pyproject_toml(&test_file, &config)?;

        // Read and verify the modified content
        let modified_content = fs::read_to_string(&test_file)?;
        println!("Modified content:\n{}", modified_content);
        let doc = modified_content.parse::<DocumentMut>()?;

        // Check that version was removed and dynamic was added
        let project = doc.get("project").unwrap().as_table().unwrap();
        assert!(!project.contains_key("version"));
        assert!(project.contains_key("dynamic"));

        let dynamic = project.get("dynamic").unwrap().as_array().unwrap();
        assert_eq!(dynamic.len(), 1);
        assert_eq!(dynamic.get(0).unwrap().as_str().unwrap(), "version");

        // Check build-system.requires
        let build_system = doc.get("build-system").unwrap().as_table().unwrap();
        let requires = build_system.get("requires").unwrap().as_array().unwrap();

        let requires_vec: Vec<&str> = requires.iter().map(|v| v.as_str().unwrap()).collect();

        assert!(requires_vec.contains(&"hatchling"));
        assert!(requires_vec.contains(&"hatch-vcs"));
        assert!(requires_vec.contains(&"setuptools-scm"));

        Ok(())
    }

    #[test]
    fn test_modify_pyproject_toml_with_existing_dynamic() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let test_file = temp_dir.path().join("existing_dynamic.toml");

        // Create pyproject.toml with existing dynamic field
        fs::write(
            &test_file,
            r#"
[project]
name = "test-project"
dynamic = ["version", "description"]
dependencies = ["requests"]

[build-system]
requires = ["hatchling", "hatch-vcs"]
build-backend = "hatchling.build"

[tool.hatch.version]
source = "vcs"
"#,
        )?;

        let config = UvinitConfig {
            additional_requires: vec!["setuptools-scm".to_string()],
            ..Default::default()
        };

        // Should not modify file that already has dynamic
        assert!(has_project_dynamic(&test_file)?);

        // But if we force modify it, it should handle gracefully
        modify_pyproject_toml(&test_file, &config)?;

        let modified_content = fs::read_to_string(&test_file)?;
        let doc = modified_content.parse::<DocumentMut>()?;

        // Check that dynamic field still exists
        let project = doc.get("project").unwrap().as_table().unwrap();
        assert!(project.contains_key("dynamic"));

        // Check that hatch-vcs wasn't duplicated
        let build_system = doc.get("build-system").unwrap().as_table().unwrap();
        let requires = build_system.get("requires").unwrap().as_array().unwrap();

        let hatch_vcs_count = requires
            .iter()
            .filter(|v| v.as_str() == Some("hatch-vcs"))
            .count();

        assert_eq!(hatch_vcs_count, 1);

        Ok(())
    }

    #[test]
    fn test_modify_pyproject_toml_disabled_features() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let test_file = temp_dir.path().join("disabled_features.toml");

        fs::write(
            &test_file,
            r#"
[project]
name = "test-project"
version = "0.1.0"

[build-system]
requires = ["hatchling"]
build-backend = "hatchling.build"
"#,
        )?;

        let config = UvinitConfig {
            enable_bandit: false,
            enable_pytest_asyncio: false,
            enable_dynamic_version: false,
            add_hatch_vcs: false,
            additional_requires: vec![],
            skip_dirs: vec![],
        };

        modify_pyproject_toml(&test_file, &config)?;

        let modified_content = fs::read_to_string(&test_file)?;
        let doc = modified_content.parse::<DocumentMut>()?;

        // Check that version was NOT removed since dynamic version is disabled
        let project = doc.get("project").unwrap().as_table().unwrap();
        assert!(project.contains_key("version"));
        assert!(!project.contains_key("dynamic"));

        // Check that hatch-vcs was NOT added
        let build_system = doc.get("build-system").unwrap().as_table().unwrap();
        let requires = build_system.get("requires").unwrap().as_array().unwrap();

        let requires_vec: Vec<&str> = requires.iter().map(|v| v.as_str().unwrap()).collect();

        assert!(!requires_vec.contains(&"hatch-vcs"));

        // Check that tool.hatch was NOT added
        assert!(!doc.contains_key("tool"));

        Ok(())
    }

    #[test]
    fn test_config_load_and_save() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let config_path = temp_dir.path().join("test_config.toml");

        // Create a test config
        let mut config = Config::default();
        config.uvinit.skip_dirs.push("custom_skip".to_string());
        config
            .uvinit
            .additional_requires
            .push("custom-requirement".to_string());

        // Save config
        let content = toml::to_string_pretty(&config)?;
        fs::write(&config_path, content)?;

        // Load config
        let loaded_content = fs::read_to_string(&config_path)?;
        let loaded_config: Config = toml::from_str(&loaded_content)?;

        assert_eq!(loaded_config.uvinit.skip_dirs, config.uvinit.skip_dirs);
        assert_eq!(
            loaded_config.uvinit.additional_requires,
            config.uvinit.additional_requires
        );
        assert_eq!(
            loaded_config.uvinit.enable_dynamic_version,
            config.uvinit.enable_dynamic_version
        );

        Ok(())
    }
}
