use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize, Deserialize)]
pub struct ProjectSection {
    pub name: String,
    pub engine_version: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PathsSection {
    pub assets: PathBuf,
    pub scenes: PathBuf,
    pub default_scene: PathBuf,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct BuildSection {
    pub profile: Option<String>,
    pub features: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProjectConfig {
    pub project: ProjectSection,
    pub paths: PathsSection,
    #[serde(default)]
    pub build: BuildSection,
}

#[derive(Debug)]
pub struct Project {
    pub root: PathBuf,
    pub config: ProjectConfig,
}

impl Project {
    pub fn create_scaffold(
        root: impl AsRef<Path>,
        name: &str,
        engine_version: &str,
    ) -> std::io::Result<Self> {
        use std::fs;

        let root = root.as_ref().to_path_buf();
        fs::create_dir_all(&root)?;
        fs::create_dir_all(root.join("src"))?;
        fs::create_dir_all(root.join("assets"))?;
        fs::create_dir_all(root.join("scenes"))?;

        let config = ProjectConfig {
            project: ProjectSection {
                name: name.to_string(),
                engine_version: engine_version.to_string(),
            },
            paths: PathsSection {
                assets: PathBuf::from("assets"),
                scenes: PathBuf::from("scenes"),
                default_scene: PathBuf::from("scenes/main.scene.json"),
            },
            build: BuildSection {
                profile: Some("release".into()),
                features: Vec::new(),
            },
        };

        let toml_str = toml::to_string_pretty(&config).expect("serialize project config");
        fs::write(root.join("minima.project.toml"), toml_str)?;
        let cargo_toml = format!(
            r#"[package]
name = "{name_kebab}"
version = "0.1.0"
edition = "2024"

[dependencies]
minima-runtime = {{ path = "../../crates/minima-runtime" }}
minima-3d      = {{ path = "../../crates/minima-3d" }}
minima-camera  = {{ path = "../../crates/minima-camera" }}
minima-gltf    = {{ path = "../../crates/minima-gltf" }}
minima-scene   = {{ path = "../../crates/minima-scene" }}
"#,
            name_kebab = name.to_lowercase().replace(' ', "_"),
        );
        fs::write(root.join("Cargo.toml"), cargo_toml)?;

        let main_rs = r#"use minima_runtime::run_game;

fn main() {
    run_game();
}
"#;
        fs::write(root.join("src/main.rs"), main_rs)?;

        let default_scene = r#"{
  "objects": []
}
"#;
        fs::write(root.join("scenes/main.scene.json"), default_scene)?;

        Ok(Project { root, config })
    }
}
