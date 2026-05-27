use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub icon: String,
    #[serde(default)]
    pub builtin: bool,
    #[serde(default)]
    pub permissions: Vec<String>,
    #[serde(default, rename = "eventSubscriptions")]
    pub event_subscriptions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReactionTrigger {
    pub event: String,
    #[serde(default)]
    pub condition: Option<ReactionCondition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReactionCondition {
    pub field: String,
    #[serde(default)]
    pub contains: Option<String>,
    #[serde(default)]
    pub equals: Option<String>,
    #[serde(default)]
    pub not_contains: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReactionResponse {
    pub messages: Vec<String>,
    #[serde(default = "default_priority")]
    pub priority: u8,
    #[serde(default = "default_cooldown")]
    pub cooldown_minutes: u32,
}

fn default_priority() -> u8 { 5 }
fn default_cooldown() -> u32 { 10 }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reaction {
    pub id: String,
    pub trigger: ReactionTrigger,
    pub response: ReactionResponse,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReactionsFile {
    pub reactions: Vec<Reaction>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModuleStatus {
    Active,
    Disabled,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadedModule {
    pub manifest: ModuleManifest,
    pub reactions: Vec<Reaction>,
    pub status: ModuleStatus,
    pub error: Option<String>,
    pub path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleInfo {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub icon: String,
    pub builtin: bool,
    pub status: ModuleStatus,
    pub error: Option<String>,
    pub permissions: Vec<String>,
    pub reaction_count: usize,
}

impl From<&LoadedModule> for ModuleInfo {
    fn from(m: &LoadedModule) -> Self {
        Self {
            id: m.manifest.id.clone(),
            name: m.manifest.name.clone(),
            version: m.manifest.version.clone(),
            description: m.manifest.description.clone(),
            author: m.manifest.author.clone(),
            icon: m.manifest.icon.clone(),
            builtin: m.manifest.builtin,
            status: m.status.clone(),
            error: m.error.clone(),
            permissions: m.manifest.permissions.clone(),
            reaction_count: m.reactions.len(),
        }
    }
}

pub struct ModuleLoader {
    modules_dir: PathBuf,
    modules: Vec<LoadedModule>,
    disabled_ids: Vec<String>,
}

impl ModuleLoader {
    pub fn new(modules_dir: PathBuf) -> Self {
        Self {
            modules_dir,
            modules: Vec::new(),
            disabled_ids: Vec::new(),
        }
    }


    pub fn scan_and_load(&mut self) -> Vec<String> {
        self.modules.clear();
        let builtin_dir = self.modules_dir.clone();
        let user_dir = user_modules_dir();
        let mut errors = self.scan_one_dir(&builtin_dir);
        errors.extend(self.scan_one_dir(&user_dir));
        errors
    }

    fn scan_one_dir(&mut self, dir: &std::path::Path) -> Vec<String> {
        let mut errors = Vec::new();
        fs::create_dir_all(dir).ok();

        let entries = match fs::read_dir(dir) {
            Ok(e) => e,
            Err(e) => {
                errors.push(format!("Cannot read modules dir: {}", e));
                return errors;
            }
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            match self.load_module(&path) {
                Ok(mut module) => {
                    if self.disabled_ids.contains(&module.manifest.id) {
                        module.status = ModuleStatus::Disabled;
                    }
                    self.modules.push(module);
                }
                Err(e) => {
                    let folder_name = path.file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_default();
                    let err_msg = format!("[{}] {}", folder_name, e);
                    errors.push(err_msg.clone());
                    self.modules.push(LoadedModule {
                        manifest: ModuleManifest {
                            id: folder_name.clone(),
                            name: folder_name,
                            version: "0.0.0".to_string(),
                            description: String::new(),
                            author: String::new(),
                            icon: String::new(),
                            builtin: false,
                            permissions: Vec::new(),
                            event_subscriptions: Vec::new(),
                        },
                        reactions: Vec::new(),
                        status: ModuleStatus::Error,
                        error: Some(e),
                        path,
                    });
                }
            }
        }
        errors
    }

    fn load_module(&self, path: &PathBuf) -> Result<LoadedModule, String> {
        let manifest_path = path.join("manifest.json");
        if !manifest_path.exists() {
            return Err("Missing manifest.json".to_string());
        }

        let manifest_data = fs::read_to_string(&manifest_path)
            .map_err(|e| format!("Cannot read manifest.json: {}", e))?;

        let manifest: ModuleManifest = serde_json::from_str(&manifest_data)
            .map_err(|e| format!("Invalid manifest.json: {}", e))?;

        if manifest.id.is_empty() {
            return Err("manifest.json: 'id' is required".to_string());
        }
        if manifest.name.is_empty() {
            return Err("manifest.json: 'name' is required".to_string());
        }

        let reactions = self.load_reactions(path)?;

        Ok(LoadedModule {
            manifest,
            reactions,
            status: ModuleStatus::Active,
            error: None,
            path: path.clone(),
        })
    }

    fn load_reactions(&self, path: &PathBuf) -> Result<Vec<Reaction>, String> {
        let reactions_path = path.join("reactions.json");
        if !reactions_path.exists() {
            return Ok(Vec::new());
        }

        let data = fs::read_to_string(&reactions_path)
            .map_err(|e| format!("Cannot read reactions.json: {}", e))?;

        let file: ReactionsFile = serde_json::from_str(&data)
            .map_err(|e| format!("Invalid reactions.json: {}", e))?;

        Ok(file.reactions)
    }

    pub fn get_modules(&self) -> Vec<ModuleInfo> {
        self.modules.iter().map(ModuleInfo::from).collect()
    }

    pub fn get_active_reactions(&self) -> Vec<(&LoadedModule, &Reaction)> {
        self.modules.iter()
            .filter(|m| m.status == ModuleStatus::Active)
            .flat_map(|m| m.reactions.iter().map(move |r| (m, r)))
            .collect()
    }

    pub fn toggle_module(&mut self, id: &str, enabled: bool) -> bool {
        let mut found = false;
        for module in &mut self.modules {
            if module.manifest.id == id && module.status != ModuleStatus::Error {
                module.status = if enabled { ModuleStatus::Active } else { ModuleStatus::Disabled };
                found = true;
                break;
            }
        }
        if enabled {
            self.disabled_ids.retain(|i| i != id);
        } else if !self.disabled_ids.contains(&id.to_string()) {
            self.disabled_ids.push(id.to_string());
        }
        found
    }

}

pub fn user_modules_dir() -> PathBuf {
    crate::paths::modules_user_dir()
}

pub fn default_modules_dir() -> PathBuf {
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()));

    // In dev mode (cargo run), use the project's modules/ folder
    // In production (.app bundle), use the app's Resources or the exe's parent
    if let Some(dir) = exe_dir {
        // Check if we're in target/debug (dev mode)
        if dir.to_string_lossy().contains("target/debug") || dir.to_string_lossy().contains("target/release") {
            // Walk up to find the project root (where modules/ lives)
            let mut project_root = dir.clone();
            while project_root.pop() {
                if project_root.join("modules").is_dir() {
                    return project_root.join("modules");
                }
            }
        }
    }

    // Fallback: relative to current working directory
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    if cwd.join("modules").is_dir() {
        return cwd.join("modules");
    }

    // Final fallback: Library/Application Support
    crate::paths::modules_user_dir()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn setup_test_dir() -> TempDir {
        tempfile::tempdir().unwrap()
    }

    fn write_module(dir: &PathBuf, id: &str, manifest: &str, reactions: Option<&str>) {
        let module_dir = dir.join(id);
        fs::create_dir_all(&module_dir).unwrap();
        fs::write(module_dir.join("manifest.json"), manifest).unwrap();
        if let Some(r) = reactions {
            fs::write(module_dir.join("reactions.json"), r).unwrap();
        }
    }

    #[test]
    fn test_load_valid_module() {
        let tmp = setup_test_dir();
        let dir = tmp.path().to_path_buf();

        write_module(&dir, "test-module", r#"{
            "id": "test-module",
            "name": "Test Module",
            "version": "1.0.0",
            "permissions": ["browser_url"]
        }"#, Some(r#"{
            "reactions": [{
                "id": "github-reaction",
                "trigger": {
                    "event": "browser_url_changed",
                    "condition": { "field": "site", "contains": "github" }
                },
                "response": {
                    "messages": ["Coding on GitHub!"],
                    "priority": 5,
                    "cooldown_minutes": 30
                }
            }]
        }"#));

        let mut loader = ModuleLoader::new(dir);
        let errors = loader.scan_and_load();

        assert!(errors.is_empty());
        assert_eq!(loader.modules.len(), 1);
        assert_eq!(loader.modules[0].manifest.id, "test-module");
        assert_eq!(loader.modules[0].reactions.len(), 1);
        assert_eq!(loader.modules[0].status, ModuleStatus::Active);
    }

    #[test]
    fn test_load_module_without_reactions() {
        let tmp = setup_test_dir();
        let dir = tmp.path().to_path_buf();

        write_module(&dir, "simple", r#"{
            "id": "simple",
            "name": "Simple Module",
            "version": "0.1.0"
        }"#, None);

        let mut loader = ModuleLoader::new(dir);
        let errors = loader.scan_and_load();

        assert!(errors.is_empty());
        assert_eq!(loader.modules[0].reactions.len(), 0);
        assert_eq!(loader.modules[0].status, ModuleStatus::Active);
    }

    #[test]
    fn test_missing_manifest() {
        let tmp = setup_test_dir();
        let dir = tmp.path().to_path_buf();
        fs::create_dir_all(dir.join("broken-module")).unwrap();

        let mut loader = ModuleLoader::new(dir);
        let errors = loader.scan_and_load();

        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("Missing manifest.json"));
        assert_eq!(loader.modules[0].status, ModuleStatus::Error);
    }

    #[test]
    fn test_invalid_json_manifest() {
        let tmp = setup_test_dir();
        let dir = tmp.path().to_path_buf();

        let module_dir = dir.join("bad-json");
        fs::create_dir_all(&module_dir).unwrap();
        fs::write(module_dir.join("manifest.json"), "not valid json {{{").unwrap();

        let mut loader = ModuleLoader::new(dir);
        let errors = loader.scan_and_load();

        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("Invalid manifest.json"));
    }

    #[test]
    fn test_missing_id_in_manifest() {
        let tmp = setup_test_dir();
        let dir = tmp.path().to_path_buf();

        write_module(&dir, "no-id", r#"{
            "id": "",
            "name": "No ID",
            "version": "1.0.0"
        }"#, None);

        let mut loader = ModuleLoader::new(dir);
        let errors = loader.scan_and_load();

        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("'id' is required"));
    }

    #[test]
    fn test_toggle_module() {
        let tmp = setup_test_dir();
        let dir = tmp.path().to_path_buf();

        write_module(&dir, "toggleable", r#"{
            "id": "toggleable",
            "name": "Toggle Test",
            "version": "1.0.0"
        }"#, None);

        let mut loader = ModuleLoader::new(dir);
        loader.scan_and_load();

        assert_eq!(loader.modules[0].status, ModuleStatus::Active);

        loader.toggle_module("toggleable", false);
        assert_eq!(loader.modules[0].status, ModuleStatus::Disabled);

        loader.toggle_module("toggleable", true);
        assert_eq!(loader.modules[0].status, ModuleStatus::Active);
    }

    #[test]
    fn test_cannot_toggle_error_module() {
        let tmp = setup_test_dir();
        let dir = tmp.path().to_path_buf();
        fs::create_dir_all(dir.join("broken")).unwrap();

        let mut loader = ModuleLoader::new(dir);
        loader.scan_and_load();

        let result = loader.toggle_module("broken", true);
        assert!(!result);
        assert_eq!(loader.modules[0].status, ModuleStatus::Error);
    }

    #[test]
    fn test_get_active_reactions() {
        let tmp = setup_test_dir();
        let dir = tmp.path().to_path_buf();

        write_module(&dir, "mod-a", r#"{
            "id": "mod-a",
            "name": "Module A",
            "version": "1.0.0"
        }"#, Some(r#"{
            "reactions": [
                { "id": "r1", "trigger": { "event": "user_typing" }, "response": { "messages": ["typing!"], "priority": 3, "cooldown_minutes": 5 } },
                { "id": "r2", "trigger": { "event": "active_app_changed" }, "response": { "messages": ["app changed!"], "priority": 5, "cooldown_minutes": 10 } }
            ]
        }"#));

        write_module(&dir, "mod-b", r#"{
            "id": "mod-b",
            "name": "Module B",
            "version": "1.0.0"
        }"#, Some(r#"{
            "reactions": [
                { "id": "r3", "trigger": { "event": "browser_url_changed" }, "response": { "messages": ["browsing!"], "priority": 7, "cooldown_minutes": 15 } }
            ]
        }"#));

        let mut loader = ModuleLoader::new(dir);
        loader.scan_and_load();

        let active = loader.get_active_reactions();
        assert_eq!(active.len(), 3);

        loader.toggle_module("mod-a", false);
        let active = loader.get_active_reactions();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].1.id, "r3");
    }

    #[test]
    fn test_multiple_modules_loaded() {
        let tmp = setup_test_dir();
        let dir = tmp.path().to_path_buf();

        write_module(&dir, "alpha", r#"{"id":"alpha","name":"Alpha","version":"1.0.0"}"#, None);
        write_module(&dir, "beta", r#"{"id":"beta","name":"Beta","version":"2.0.0"}"#, None);

        let mut loader = ModuleLoader::new(dir);
        loader.scan_and_load();

        let infos = loader.get_modules();
        assert_eq!(infos.len(), 2);
    }

    #[test]
    fn test_get_module_info() {
        let tmp = setup_test_dir();
        let dir = tmp.path().to_path_buf();

        write_module(&dir, "detailed", r#"{
            "id": "detailed",
            "name": "Detailed Module",
            "version": "1.2.3",
            "description": "A test module",
            "author": "Test Author",
            "permissions": ["browser_url", "active_app"]
        }"#, Some(r#"{"reactions":[{"id":"r1","trigger":{"event":"user_typing"},"response":{"messages":["hi"],"priority":5,"cooldown_minutes":10}}]}"#));

        let mut loader = ModuleLoader::new(dir);
        loader.scan_and_load();

        let infos = loader.get_modules();
        assert_eq!(infos[0].id, "detailed");
        assert_eq!(infos[0].name, "Detailed Module");
        assert_eq!(infos[0].version, "1.2.3");
        assert_eq!(infos[0].description, "A test module");
        assert_eq!(infos[0].author, "Test Author");
        assert_eq!(infos[0].permissions.len(), 2);
        assert_eq!(infos[0].reaction_count, 1);
    }

    #[test]
    fn test_disabled_persists_across_rescan() {
        let tmp = setup_test_dir();
        let dir = tmp.path().to_path_buf();

        write_module(&dir, "persistent", r#"{"id":"persistent","name":"P","version":"1.0.0"}"#, None);

        let mut loader = ModuleLoader::new(dir);
        loader.scan_and_load();
        loader.toggle_module("persistent", false);

        loader.scan_and_load();
        assert_eq!(loader.modules[0].status, ModuleStatus::Disabled);
    }

    // ── Checkpoint 3: user_modules_dir + dual-scan ───────────────────────────

    #[test]
    fn test_user_modules_dir_ends_with_modules() {
        let path = user_modules_dir();
        assert_eq!(path.file_name().unwrap().to_str().unwrap(), "modules");
    }

    #[test]
    fn test_scan_and_load_picks_up_module_from_second_dir() {
        let builtin_tmp = setup_test_dir();
        let user_tmp = setup_test_dir();

        // Put a module in the builtin dir and one in the "user" dir
        write_module(&builtin_tmp.path().to_path_buf(), "builtin-mod",
            r#"{"id":"builtin-mod","name":"Built-in","version":"1.0.0"}"#, None);
        write_module(&user_tmp.path().to_path_buf(), "user-mod",
            r#"{"id":"user-mod","name":"User","version":"1.0.0"}"#, None);

        // Loader uses builtin dir; we temporarily override user_modules_dir by
        // putting the user module in a subdir we scan manually via scan_one_dir
        let mut loader = ModuleLoader::new(builtin_tmp.path().to_path_buf());
        loader.modules.clear();
        loader.scan_one_dir(builtin_tmp.path());
        loader.scan_one_dir(user_tmp.path());

        let ids: Vec<_> = loader.modules.iter().map(|m| m.manifest.id.clone()).collect();
        assert!(ids.contains(&"builtin-mod".to_string()));
        assert!(ids.contains(&"user-mod".to_string()));
    }

    #[test]
    fn test_scan_and_load_merges_errors_from_both_dirs() {
        let builtin_tmp = setup_test_dir();
        let user_tmp = setup_test_dir();

        // Put a broken module (missing manifest) in each dir
        fs::create_dir_all(builtin_tmp.path().join("broken-builtin")).unwrap();
        fs::create_dir_all(user_tmp.path().join("broken-user")).unwrap();

        let mut loader = ModuleLoader::new(builtin_tmp.path().to_path_buf());
        loader.modules.clear();
        let mut errors = loader.scan_one_dir(builtin_tmp.path());
        errors.extend(loader.scan_one_dir(user_tmp.path()));

        assert_eq!(errors.len(), 2);
    }
}
