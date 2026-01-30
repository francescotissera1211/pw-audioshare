use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use crate::config::APP_ID;

/// A single connection in a preset (stored by port names, not IDs)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresetConnection {
    pub output_node: String,
    pub output_port: String,
    pub input_node: String,
    pub input_port: String,
}

/// A named preset containing a list of connections
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Preset {
    pub name: String,
    pub connections: Vec<PresetConnection>,
}

/// Collection of all saved presets
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PresetStore {
    pub presets: HashMap<String, Preset>,
    /// Name of the currently active (auto-connecting) preset, if any
    #[serde(default)]
    pub active_preset: Option<String>,
}

impl PresetStore {
    /// Get the path to the presets file
    fn presets_path() -> Option<PathBuf> {
        let config_dir = dirs::config_dir()?;
        let app_dir = config_dir.join(APP_ID);
        Some(app_dir.join("presets.json"))
    }

    /// Load presets from disk
    pub fn load() -> Self {
        let path = match Self::presets_path() {
            Some(p) => p,
            None => return Self::default(),
        };

        if !path.exists() {
            return Self::default();
        }

        match fs::read_to_string(&path) {
            Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
            Err(e) => {
                log::warn!("Failed to load presets: {}", e);
                Self::default()
            }
        }
    }

    /// Save presets to disk
    pub fn save(&self) -> Result<(), String> {
        let path = Self::presets_path().ok_or("Could not determine config directory")?;

        // Ensure directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("Failed to create config dir: {}", e))?;
        }

        let content =
            serde_json::to_string_pretty(self).map_err(|e| format!("Failed to serialize: {}", e))?;

        fs::write(&path, content).map_err(|e| format!("Failed to write presets: {}", e))?;

        Ok(())
    }

    /// Add or update a preset
    pub fn add_preset(&mut self, preset: Preset) {
        self.presets.insert(preset.name.clone(), preset);
    }

    /// Remove a preset by name
    pub fn remove_preset(&mut self, name: &str) {
        self.presets.remove(name);
    }

    /// Get a preset by name
    pub fn get_preset(&self, name: &str) -> Option<&Preset> {
        self.presets.get(name)
    }

    /// Get all preset names
    pub fn preset_names(&self) -> Vec<String> {
        let mut names: Vec<_> = self.presets.keys().cloned().collect();
        names.sort();
        names
    }

    /// Activate a preset for auto-connecting
    pub fn activate_preset(&mut self, name: &str) {
        if self.presets.contains_key(name) {
            self.active_preset = Some(name.to_string());
        }
    }

    /// Deactivate the current preset
    pub fn deactivate_preset(&mut self) {
        self.active_preset = None;
    }

    /// Get the currently active preset, if any
    pub fn get_active_preset(&self) -> Option<&Preset> {
        self.active_preset
            .as_ref()
            .and_then(|name| self.presets.get(name))
    }

    /// Check if a preset is currently active
    pub fn is_active(&self, name: &str) -> bool {
        self.active_preset.as_deref() == Some(name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_connection(out_node: &str, out_port: &str, in_node: &str, in_port: &str) -> PresetConnection {
        PresetConnection {
            output_node: out_node.to_string(),
            output_port: out_port.to_string(),
            input_node: in_node.to_string(),
            input_port: in_port.to_string(),
        }
    }

    fn make_preset(name: &str, connections: Vec<PresetConnection>) -> Preset {
        Preset {
            name: name.to_string(),
            connections,
        }
    }

    // --- CRUD ---

    #[test]
    fn test_default_store_is_empty() {
        let store = PresetStore::default();
        assert!(store.presets.is_empty());
        assert!(store.active_preset.is_none());
    }

    #[test]
    fn test_add_and_get_preset() {
        let mut store = PresetStore::default();
        store.add_preset(make_preset("studio", vec![]));
        let preset = store.get_preset("studio").unwrap();
        assert_eq!(preset.name, "studio");
    }

    #[test]
    fn test_add_preset_overwrites_existing() {
        let mut store = PresetStore::default();
        store.add_preset(make_preset("test", vec![]));
        store.add_preset(make_preset(
            "test",
            vec![make_connection("a", "b", "c", "d")],
        ));
        assert_eq!(store.get_preset("test").unwrap().connections.len(), 1);
    }

    #[test]
    fn test_get_nonexistent_preset_returns_none() {
        let store = PresetStore::default();
        assert!(store.get_preset("nope").is_none());
    }

    #[test]
    fn test_remove_preset() {
        let mut store = PresetStore::default();
        store.add_preset(make_preset("test", vec![]));
        store.remove_preset("test");
        assert!(store.get_preset("test").is_none());
    }

    #[test]
    fn test_remove_nonexistent_preset_is_noop() {
        let mut store = PresetStore::default();
        store.remove_preset("nonexistent");
        assert!(store.presets.is_empty());
    }

    #[test]
    fn test_preset_names_sorted() {
        let mut store = PresetStore::default();
        store.add_preset(make_preset("zebra", vec![]));
        store.add_preset(make_preset("alpha", vec![]));
        store.add_preset(make_preset("middle", vec![]));
        assert_eq!(store.preset_names(), vec!["alpha", "middle", "zebra"]);
    }

    // --- Activation ---

    #[test]
    fn test_activate_preset() {
        let mut store = PresetStore::default();
        store.add_preset(make_preset("studio", vec![]));
        store.activate_preset("studio");
        assert_eq!(store.active_preset, Some("studio".to_string()));
        assert!(store.is_active("studio"));
    }

    #[test]
    fn test_activate_nonexistent_preset_does_nothing() {
        let mut store = PresetStore::default();
        store.activate_preset("nonexistent");
        assert!(store.active_preset.is_none());
    }

    #[test]
    fn test_deactivate_preset() {
        let mut store = PresetStore::default();
        store.add_preset(make_preset("studio", vec![]));
        store.activate_preset("studio");
        store.deactivate_preset();
        assert!(store.active_preset.is_none());
        assert!(!store.is_active("studio"));
    }

    #[test]
    fn test_deactivate_when_none_active() {
        let mut store = PresetStore::default();
        store.deactivate_preset(); // should not panic
        assert!(store.active_preset.is_none());
    }

    #[test]
    fn test_get_active_preset_when_none() {
        let store = PresetStore::default();
        assert!(store.get_active_preset().is_none());
    }

    #[test]
    fn test_get_active_preset_returns_data() {
        let mut store = PresetStore::default();
        store.add_preset(make_preset(
            "studio",
            vec![make_connection("mic", "capture_FL", "daw", "input_FL")],
        ));
        store.activate_preset("studio");
        let active = store.get_active_preset().unwrap();
        assert_eq!(active.name, "studio");
        assert_eq!(active.connections.len(), 1);
    }

    #[test]
    fn test_is_active_returns_false_for_wrong_name() {
        let mut store = PresetStore::default();
        store.add_preset(make_preset("studio", vec![]));
        store.activate_preset("studio");
        assert!(!store.is_active("other"));
    }

    #[test]
    fn test_removing_active_preset_leaves_stale_active() {
        // Removing a preset does not auto-deactivate — caller must handle this
        let mut store = PresetStore::default();
        store.add_preset(make_preset("studio", vec![]));
        store.activate_preset("studio");
        store.remove_preset("studio");
        // active_preset still points to "studio" but get_active_preset returns None
        assert_eq!(store.active_preset, Some("studio".to_string()));
        assert!(store.get_active_preset().is_none());
    }

    // --- Serialization ---

    #[test]
    fn test_serialize_deserialize_roundtrip() {
        let mut store = PresetStore::default();
        store.add_preset(make_preset(
            "studio",
            vec![
                make_connection("mic", "capture_FL", "ardour", "input_FL"),
                make_connection("mic", "capture_FR", "ardour", "input_FR"),
            ],
        ));
        store.activate_preset("studio");

        let json = serde_json::to_string(&store).unwrap();
        let restored: PresetStore = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.preset_names(), vec!["studio"]);
        assert_eq!(restored.active_preset, Some("studio".to_string()));
        let preset = restored.get_preset("studio").unwrap();
        assert_eq!(preset.connections.len(), 2);
        assert_eq!(preset.connections[0].output_node, "mic");
    }

    #[test]
    fn test_deserialize_without_active_preset_field() {
        let json = r#"{"presets":{}}"#;
        let store: PresetStore = serde_json::from_str(json).unwrap();
        assert!(store.active_preset.is_none());
    }

    #[test]
    fn test_deserialize_empty_json_object_fails() {
        // The `presets` field is required (not #[serde(default)]),
        // so an empty JSON object is invalid
        let result: Result<PresetStore, _> = serde_json::from_str("{}");
        assert!(result.is_err());
    }

    #[test]
    fn test_deserialize_minimal_valid_json() {
        let json = r#"{"presets":{}}"#;
        let store: PresetStore = serde_json::from_str(json).unwrap();
        assert!(store.presets.is_empty());
        assert!(store.active_preset.is_none());
    }

    #[test]
    fn test_invalid_json_fails_to_parse() {
        let result: Result<PresetStore, _> = serde_json::from_str("not valid json");
        assert!(result.is_err());
    }

    #[test]
    fn test_preset_connection_roundtrip() {
        let conn = make_connection("node_a", "port_1", "node_b", "port_2");
        let json = serde_json::to_string(&conn).unwrap();
        let restored: PresetConnection = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.output_node, "node_a");
        assert_eq!(restored.output_port, "port_1");
        assert_eq!(restored.input_node, "node_b");
        assert_eq!(restored.input_port, "port_2");
    }

    #[test]
    fn test_multiple_presets_roundtrip() {
        let mut store = PresetStore::default();
        store.add_preset(make_preset("gaming", vec![
            make_connection("game", "output_FL", "headset", "input_FL"),
        ]));
        store.add_preset(make_preset("music", vec![
            make_connection("player", "output_FL", "speakers", "input_FL"),
            make_connection("player", "output_FR", "speakers", "input_FR"),
        ]));

        let json = serde_json::to_string_pretty(&store).unwrap();
        let restored: PresetStore = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.presets.len(), 2);
        assert_eq!(restored.get_preset("gaming").unwrap().connections.len(), 1);
        assert_eq!(restored.get_preset("music").unwrap().connections.len(), 2);
    }
}
