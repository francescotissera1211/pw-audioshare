use std::collections::HashMap;

use super::messages::{LinkState, MediaType, PortDirection};
use crate::presets::PresetConnection;

/// Represents a PipeWire node (audio device, application, etc.)
#[derive(Debug, Clone)]
pub struct PwNode {
    pub id: u32,
    pub name: String,
    pub media_class: Option<String>,
    pub description: Option<String>,
    pub application_name: Option<String>,
}

impl PwNode {
    /// Returns the best display name for this node
    pub fn display_name(&self) -> &str {
        self.description
            .as_deref()
            .or(self.application_name.as_deref())
            .unwrap_or(&self.name)
    }
}

/// Represents a port on a node
#[derive(Debug, Clone)]
pub struct PwPort {
    pub id: u32,
    pub node_id: u32,
    pub name: String,
    pub alias: Option<String>,
    pub direction: PortDirection,
    pub media_type: MediaType,
    pub channel: Option<String>,
}

impl PwPort {
    /// Returns the best display name for this port
    pub fn display_name(&self) -> &str {
        self.alias.as_deref().unwrap_or(&self.name)
    }
}

/// Represents a link between two ports
#[derive(Debug, Clone)]
pub struct PwLink {
    pub id: u32,
    pub output_node_id: u32,
    pub output_port_id: u32,
    pub input_node_id: u32,
    pub input_port_id: u32,
    pub state: LinkState,
}

/// Holds the complete PipeWire state as seen by the application
#[derive(Debug, Default)]
pub struct PwState {
    pub nodes: HashMap<u32, PwNode>,
    pub ports: HashMap<u32, PwPort>,
    pub links: HashMap<u32, PwLink>,
}

impl PwState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Get the node that owns a port
    pub fn get_port_node(&self, port_id: u32) -> Option<&PwNode> {
        self.ports
            .get(&port_id)
            .and_then(|port| self.nodes.get(&port.node_id))
    }

    /// Get all ports for a node
    pub fn get_node_ports(&self, node_id: u32) -> impl Iterator<Item = &PwPort> {
        self.ports.values().filter(move |p| p.node_id == node_id)
    }

    /// Get all output ports (sources)
    pub fn output_ports(&self) -> impl Iterator<Item = &PwPort> {
        self.ports
            .values()
            .filter(|p| p.direction == PortDirection::Output)
    }

    /// Get all input ports (sinks)
    pub fn input_ports(&self) -> impl Iterator<Item = &PwPort> {
        self.ports
            .values()
            .filter(|p| p.direction == PortDirection::Input)
    }

    /// Check if a link exists between two ports
    pub fn link_exists(&self, output_port_id: u32, input_port_id: u32) -> bool {
        self.links.values().any(|link| {
            link.output_port_id == output_port_id && link.input_port_id == input_port_id
        })
    }

    /// Find link by port IDs
    pub fn find_link(&self, output_port_id: u32, input_port_id: u32) -> Option<&PwLink> {
        self.links.values().find(|link| {
            link.output_port_id == output_port_id && link.input_port_id == input_port_id
        })
    }

    /// Given preset connections (stored by node/port names), find pairs of
    /// (output_port_id, input_port_id) where both ports currently exist and
    /// no link already connects them.
    pub fn find_preset_matches(&self, connections: &[PresetConnection]) -> Vec<(u32, u32)> {
        let mut result = Vec::new();

        for conn in connections {
            let output_port = self.ports.values().find(|p| {
                p.direction == PortDirection::Output
                    && p.name == conn.output_port
                    && self
                        .nodes
                        .get(&p.node_id)
                        .map(|n| n.name == conn.output_node)
                        .unwrap_or(false)
            });

            let input_port = self.ports.values().find(|p| {
                p.direction == PortDirection::Input
                    && p.name == conn.input_port
                    && self
                        .nodes
                        .get(&p.node_id)
                        .map(|n| n.name == conn.input_node)
                        .unwrap_or(false)
            });

            if let (Some(out), Some(inp)) = (output_port, input_port) {
                if !self.link_exists(out.id, inp.id) {
                    result.push((out.id, inp.id));
                }
            }
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: create a state with two nodes and four ports (stereo output -> stereo input)
    fn setup_stereo_state() -> PwState {
        let mut state = PwState::new();

        state.nodes.insert(
            1,
            PwNode {
                id: 1,
                name: "firefox".to_string(),
                media_class: Some("Stream/Output/Audio".to_string()),
                description: Some("Firefox".to_string()),
                application_name: Some("Firefox Browser".to_string()),
            },
        );
        state.nodes.insert(
            2,
            PwNode {
                id: 2,
                name: "alsa_output.pci".to_string(),
                media_class: Some("Audio/Sink".to_string()),
                description: Some("Speakers".to_string()),
                application_name: None,
            },
        );

        state.ports.insert(
            10,
            PwPort {
                id: 10,
                node_id: 1,
                name: "output_FL".to_string(),
                alias: Some("Front Left".to_string()),
                direction: PortDirection::Output,
                media_type: MediaType::Audio,
                channel: Some("FL".to_string()),
            },
        );
        state.ports.insert(
            11,
            PwPort {
                id: 11,
                node_id: 1,
                name: "output_FR".to_string(),
                alias: None,
                direction: PortDirection::Output,
                media_type: MediaType::Audio,
                channel: Some("FR".to_string()),
            },
        );
        state.ports.insert(
            20,
            PwPort {
                id: 20,
                node_id: 2,
                name: "input_FL".to_string(),
                alias: None,
                direction: PortDirection::Input,
                media_type: MediaType::Audio,
                channel: Some("FL".to_string()),
            },
        );
        state.ports.insert(
            21,
            PwPort {
                id: 21,
                node_id: 2,
                name: "input_FR".to_string(),
                alias: None,
                direction: PortDirection::Input,
                media_type: MediaType::Audio,
                channel: Some("FR".to_string()),
            },
        );

        state
    }

    // --- Basic state operations ---

    #[test]
    fn test_new_state_is_empty() {
        let state = PwState::new();
        assert!(state.nodes.is_empty());
        assert!(state.ports.is_empty());
        assert!(state.links.is_empty());
    }

    #[test]
    fn test_add_and_retrieve_node() {
        let mut state = PwState::new();
        state.nodes.insert(
            1,
            PwNode {
                id: 1,
                name: "test_node".to_string(),
                media_class: None,
                description: None,
                application_name: None,
            },
        );
        assert_eq!(state.nodes.len(), 1);
        assert_eq!(state.nodes.get(&1).unwrap().name, "test_node");
    }

    #[test]
    fn test_remove_node() {
        let mut state = PwState::new();
        state.nodes.insert(
            1,
            PwNode {
                id: 1,
                name: "test".to_string(),
                media_class: None,
                description: None,
                application_name: None,
            },
        );
        state.nodes.remove(&1);
        assert!(state.nodes.is_empty());
    }

    // --- Display names ---

    #[test]
    fn test_node_display_name_prefers_description() {
        let node = PwNode {
            id: 1,
            name: "internal".to_string(),
            media_class: None,
            description: Some("Friendly Name".to_string()),
            application_name: Some("App".to_string()),
        };
        assert_eq!(node.display_name(), "Friendly Name");
    }

    #[test]
    fn test_node_display_name_falls_back_to_application_name() {
        let node = PwNode {
            id: 1,
            name: "internal".to_string(),
            media_class: None,
            description: None,
            application_name: Some("App Name".to_string()),
        };
        assert_eq!(node.display_name(), "App Name");
    }

    #[test]
    fn test_node_display_name_falls_back_to_name() {
        let node = PwNode {
            id: 1,
            name: "internal_name".to_string(),
            media_class: None,
            description: None,
            application_name: None,
        };
        assert_eq!(node.display_name(), "internal_name");
    }

    #[test]
    fn test_port_display_name_prefers_alias() {
        let port = PwPort {
            id: 1,
            node_id: 1,
            name: "output_FL".to_string(),
            alias: Some("Front Left".to_string()),
            direction: PortDirection::Output,
            media_type: MediaType::Audio,
            channel: None,
        };
        assert_eq!(port.display_name(), "Front Left");
    }

    #[test]
    fn test_port_display_name_falls_back_to_name() {
        let port = PwPort {
            id: 1,
            node_id: 1,
            name: "output_FL".to_string(),
            alias: None,
            direction: PortDirection::Output,
            media_type: MediaType::Audio,
            channel: None,
        };
        assert_eq!(port.display_name(), "output_FL");
    }

    // --- Query methods ---

    #[test]
    fn test_get_port_node() {
        let state = setup_stereo_state();
        let node = state.get_port_node(10).unwrap();
        assert_eq!(node.name, "firefox");
    }

    #[test]
    fn test_get_port_node_returns_none_for_missing_port() {
        let state = setup_stereo_state();
        assert!(state.get_port_node(999).is_none());
    }

    #[test]
    fn test_get_node_ports() {
        let state = setup_stereo_state();
        let ports: Vec<_> = state.get_node_ports(1).collect();
        assert_eq!(ports.len(), 2);
        assert!(ports.iter().all(|p| p.node_id == 1));
    }

    #[test]
    fn test_get_node_ports_empty_for_missing_node() {
        let state = setup_stereo_state();
        let ports: Vec<_> = state.get_node_ports(999).collect();
        assert!(ports.is_empty());
    }

    #[test]
    fn test_output_ports() {
        let state = setup_stereo_state();
        let outputs: Vec<_> = state.output_ports().collect();
        assert_eq!(outputs.len(), 2);
        assert!(outputs
            .iter()
            .all(|p| p.direction == PortDirection::Output));
    }

    #[test]
    fn test_input_ports() {
        let state = setup_stereo_state();
        let inputs: Vec<_> = state.input_ports().collect();
        assert_eq!(inputs.len(), 2);
        assert!(inputs.iter().all(|p| p.direction == PortDirection::Input));
    }

    // --- Link operations (connections / disconnections) ---

    #[test]
    fn test_add_link() {
        let mut state = setup_stereo_state();
        state.links.insert(
            100,
            PwLink {
                id: 100,
                output_node_id: 1,
                output_port_id: 10,
                input_node_id: 2,
                input_port_id: 20,
                state: LinkState::Active,
            },
        );
        assert_eq!(state.links.len(), 1);
        assert!(state.link_exists(10, 20));
    }

    #[test]
    fn test_remove_link() {
        let mut state = setup_stereo_state();
        state.links.insert(
            100,
            PwLink {
                id: 100,
                output_node_id: 1,
                output_port_id: 10,
                input_node_id: 2,
                input_port_id: 20,
                state: LinkState::Active,
            },
        );
        state.links.remove(&100);
        assert!(state.links.is_empty());
        assert!(!state.link_exists(10, 20));
    }

    #[test]
    fn test_link_exists_false_when_no_links() {
        let state = setup_stereo_state();
        assert!(!state.link_exists(10, 20));
    }

    #[test]
    fn test_find_link() {
        let mut state = setup_stereo_state();
        state.links.insert(
            100,
            PwLink {
                id: 100,
                output_node_id: 1,
                output_port_id: 10,
                input_node_id: 2,
                input_port_id: 20,
                state: LinkState::Active,
            },
        );
        let link = state.find_link(10, 20).unwrap();
        assert_eq!(link.id, 100);
        assert_eq!(link.state, LinkState::Active);
    }

    #[test]
    fn test_find_link_returns_none() {
        let state = setup_stereo_state();
        assert!(state.find_link(10, 20).is_none());
    }

    #[test]
    fn test_connection_and_disconnection_flow() {
        let mut state = setup_stereo_state();

        // Connect stereo pair
        state.links.insert(
            100,
            PwLink {
                id: 100,
                output_node_id: 1,
                output_port_id: 10,
                input_node_id: 2,
                input_port_id: 20,
                state: LinkState::Active,
            },
        );
        state.links.insert(
            101,
            PwLink {
                id: 101,
                output_node_id: 1,
                output_port_id: 11,
                input_node_id: 2,
                input_port_id: 21,
                state: LinkState::Active,
            },
        );

        assert_eq!(state.links.len(), 2);
        assert!(state.link_exists(10, 20));
        assert!(state.link_exists(11, 21));

        // Disconnect one channel
        state.links.remove(&100);
        assert_eq!(state.links.len(), 1);
        assert!(!state.link_exists(10, 20));
        assert!(state.link_exists(11, 21));

        // Disconnect remaining
        state.links.remove(&101);
        assert!(state.links.is_empty());
    }

    // --- Auto-reconnect: find_preset_matches ---

    #[test]
    fn test_find_preset_matches_both_ports_exist() {
        let state = setup_stereo_state();
        let connections = vec![PresetConnection {
            output_node: "firefox".to_string(),
            output_port: "output_FL".to_string(),
            input_node: "alsa_output.pci".to_string(),
            input_port: "input_FL".to_string(),
        }];
        let matches = state.find_preset_matches(&connections);
        assert_eq!(matches, vec![(10, 20)]);
    }

    #[test]
    fn test_find_preset_matches_stereo_pair() {
        let state = setup_stereo_state();
        let connections = vec![
            PresetConnection {
                output_node: "firefox".to_string(),
                output_port: "output_FL".to_string(),
                input_node: "alsa_output.pci".to_string(),
                input_port: "input_FL".to_string(),
            },
            PresetConnection {
                output_node: "firefox".to_string(),
                output_port: "output_FR".to_string(),
                input_node: "alsa_output.pci".to_string(),
                input_port: "input_FR".to_string(),
            },
        ];
        let mut matches = state.find_preset_matches(&connections);
        matches.sort();
        assert_eq!(matches, vec![(10, 20), (11, 21)]);
    }

    #[test]
    fn test_find_preset_matches_output_node_missing() {
        let state = setup_stereo_state();
        let connections = vec![PresetConnection {
            output_node: "nonexistent".to_string(),
            output_port: "output_FL".to_string(),
            input_node: "alsa_output.pci".to_string(),
            input_port: "input_FL".to_string(),
        }];
        assert!(state.find_preset_matches(&connections).is_empty());
    }

    #[test]
    fn test_find_preset_matches_input_node_missing() {
        let state = setup_stereo_state();
        let connections = vec![PresetConnection {
            output_node: "firefox".to_string(),
            output_port: "output_FL".to_string(),
            input_node: "nonexistent".to_string(),
            input_port: "input_FL".to_string(),
        }];
        assert!(state.find_preset_matches(&connections).is_empty());
    }

    #[test]
    fn test_find_preset_matches_port_name_wrong() {
        let state = setup_stereo_state();
        let connections = vec![PresetConnection {
            output_node: "firefox".to_string(),
            output_port: "wrong_port_name".to_string(),
            input_node: "alsa_output.pci".to_string(),
            input_port: "input_FL".to_string(),
        }];
        assert!(state.find_preset_matches(&connections).is_empty());
    }

    #[test]
    fn test_find_preset_matches_skips_existing_link() {
        let mut state = setup_stereo_state();
        state.links.insert(
            100,
            PwLink {
                id: 100,
                output_node_id: 1,
                output_port_id: 10,
                input_node_id: 2,
                input_port_id: 20,
                state: LinkState::Active,
            },
        );
        let connections = vec![PresetConnection {
            output_node: "firefox".to_string(),
            output_port: "output_FL".to_string(),
            input_node: "alsa_output.pci".to_string(),
            input_port: "input_FL".to_string(),
        }];
        assert!(state.find_preset_matches(&connections).is_empty());
    }

    #[test]
    fn test_find_preset_matches_partial_availability() {
        let state = setup_stereo_state();
        let connections = vec![
            // This one exists
            PresetConnection {
                output_node: "firefox".to_string(),
                output_port: "output_FL".to_string(),
                input_node: "alsa_output.pci".to_string(),
                input_port: "input_FL".to_string(),
            },
            // This one has a missing input node
            PresetConnection {
                output_node: "firefox".to_string(),
                output_port: "output_FR".to_string(),
                input_node: "missing_sink".to_string(),
                input_port: "input_FR".to_string(),
            },
        ];
        let matches = state.find_preset_matches(&connections);
        assert_eq!(matches, vec![(10, 20)]);
    }

    #[test]
    fn test_find_preset_matches_empty_connections() {
        let state = setup_stereo_state();
        assert!(state.find_preset_matches(&[]).is_empty());
    }

    #[test]
    fn test_find_preset_matches_empty_state() {
        let state = PwState::new();
        let connections = vec![PresetConnection {
            output_node: "firefox".to_string(),
            output_port: "output_FL".to_string(),
            input_node: "alsa_output.pci".to_string(),
            input_port: "input_FL".to_string(),
        }];
        assert!(state.find_preset_matches(&connections).is_empty());
    }

    #[test]
    fn test_find_preset_matches_disambiguates_by_node_name() {
        let mut state = setup_stereo_state();
        // Add a second output node with the same port name
        state.nodes.insert(
            3,
            PwNode {
                id: 3,
                name: "chromium".to_string(),
                media_class: None,
                description: None,
                application_name: None,
            },
        );
        state.ports.insert(
            30,
            PwPort {
                id: 30,
                node_id: 3,
                name: "output_FL".to_string(),
                alias: None,
                direction: PortDirection::Output,
                media_type: MediaType::Audio,
                channel: Some("FL".to_string()),
            },
        );

        // Preset asks for chromium's output_FL, not firefox's
        let connections = vec![PresetConnection {
            output_node: "chromium".to_string(),
            output_port: "output_FL".to_string(),
            input_node: "alsa_output.pci".to_string(),
            input_port: "input_FL".to_string(),
        }];
        let matches = state.find_preset_matches(&connections);
        assert_eq!(matches, vec![(30, 20)]);
    }

    #[test]
    fn test_find_preset_matches_wrong_direction_ignored() {
        let state = setup_stereo_state();
        // Preset refers to an input port name as the output — should not match
        let connections = vec![PresetConnection {
            output_node: "alsa_output.pci".to_string(),
            output_port: "input_FL".to_string(), // This is an input port, not output
            input_node: "firefox".to_string(),
            input_port: "output_FL".to_string(), // This is an output port, not input
        }];
        assert!(state.find_preset_matches(&connections).is_empty());
    }
}
