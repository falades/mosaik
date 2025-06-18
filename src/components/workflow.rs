use dioxus::prelude::*;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use crate::components::{
    canvas::CanvasState,
    nodes::{Node, NodeType, NodeComponent, ProviderType},
    connections::{Connection, get_port_world_pos, ConnectionDrawingState, ConnectionsRenderer}
};

#[derive(Clone, Serialize, Deserialize)]
pub struct Workflow {
    pub nodes: HashMap<usize, Node>,
    pub connections: HashMap<usize, Connection>,
    pub next_node_id: usize,
    pub next_connection_id: usize,
    pub selected_node_id: Option<usize>,
    pub dragging_node_id: Option<usize>,
    pub drawing_connection_state: ConnectionDrawingState,
}


impl Default for Workflow {
    fn default() -> Self {
        let mut state = Workflow {
            nodes: HashMap::new(),
            connections: HashMap::new(),
            next_node_id: 0,
            next_connection_id: 0,
            selected_node_id: None,
            dragging_node_id: None,
            drawing_connection_state: ConnectionDrawingState::default(),
        };

        // Add the default nodes
        let context_id = state.add_node(NodeType::Prompt {}, 50.0, 100.0);
        let model_id = state.add_node(NodeType::Model { provider: ProviderType::Ollama, model_name: "".into(), messages: Vec::new(), thinking: false }, 400.0, 100.0);
        let _ = state.add_connection(context_id, model_id);
        
        state
    }
}

impl Workflow {
    pub fn add_node(&mut self, node_type_variant: NodeType, position_x: f64, position_y: f64) -> usize {
        let id = self.next_node_id;
        // Pass a reference to node_type_variant to Node::new to determine which variant it is
        let node = Node::new(id, &node_type_variant, position_x, position_y);
        self.nodes.insert(id, node);
        self.next_node_id += 1;
        id
    }

    pub fn remove_node(&mut self, id: usize) {
        self.nodes.remove(&id);
        self.connections.retain(|_, conn| conn.from_node_id != id && conn.to_node_id != id);

        if self.selected_node_id == Some(id) {
            self.selected_node_id = None;
        }
        if self.dragging_node_id == Some(id) {
            self.dragging_node_id = None;
        }
        if self.drawing_connection_state.active && self.drawing_connection_state.source_node_id == id {
            self.cancel_drawing_connection();
        }
    }

    pub fn add_connection(&mut self, from_node_id: usize, to_node_id: usize) -> Result<usize, String> {
        if from_node_id == to_node_id {
            return Err("Cannot connect a node to itself".to_string());
        }

        let conn_id = self.next_connection_id;
        self.connections.insert(conn_id, Connection { id: conn_id, from_node_id, to_node_id });
        self.next_connection_id += 1;
        
        self.update_node_input_from_all_sources(&to_node_id);
        // Mark target node as needing execution
        if let Some(node) = self.nodes.get_mut(&to_node_id) {
            node.needs_execution = true;
        }
        Ok(conn_id)
    }

    fn remove_connection_by_target_node(&mut self, target_node_id: usize) {
        let conn_id_to_remove = self.connections.iter()
            .find_map(|(id, conn)| if conn.to_node_id == target_node_id { Some(*id) } else { None });

        if let Some(id) = conn_id_to_remove {
            self.connections.remove(&id);
            if let Some(node) = self.nodes.get_mut(&target_node_id) {
                node.input = None;
            }
        }
    }

    pub fn start_dragging_node(&mut self, node_id: usize, mouse_page_x: f64, mouse_page_y: f64, canvas: &CanvasState) {
        if let Some(node) = self.nodes.get_mut(&node_id) {
            self.dragging_node_id = Some(node_id);
            let mouse_world_pos = canvas.page_to_world_coords(mouse_page_x, mouse_page_y);
            node.drag_offset_x = mouse_world_pos.0 - node.position_x;
            node.drag_offset_y = mouse_world_pos.1 - node.position_y;
        }
    }

    pub fn drag_node(&mut self, mouse_page_x: f64, mouse_page_y: f64, canvas: &CanvasState) {
        if let Some(node_id) = self.dragging_node_id {
            if let Some(node) = self.nodes.get_mut(&node_id) {
                let mouse_world_pos = canvas.page_to_world_coords(mouse_page_x, mouse_page_y);
                node.position_x = mouse_world_pos.0 - node.drag_offset_x;
                node.position_y = mouse_world_pos.1 - node.drag_offset_y;
            }
        }
    }

    pub fn end_dragging_node(&mut self) {
        if let Some(dragged_node_id) = self.dragging_node_id {
            self.dragging_node_id = None;
            // Update inputs of nodes that receive from the dragged node
            self.propagate_output_to_connected_nodes(&dragged_node_id);
        }
    }
    
    pub fn get_input_order_number(&self, node_id: usize, target_node_id: usize) -> Option<usize> {
        // Get all source nodes for the target, sorted by visual position
        let mut source_nodes: Vec<(usize, f64, f64)> = self.connections
            .values()
            .filter(|conn| conn.to_node_id == target_node_id)
            .filter_map(|conn| {
                self.nodes.get(&conn.from_node_id).map(|node| 
                    (conn.from_node_id, node.position_y, node.position_x)
                )
            })
            .collect();
        
        // Only show numbers if there are multiple inputs
        if source_nodes.len() <= 1 {
            return None;
        }
        
        // Sort by visual position
        source_nodes.sort_by(|a, b| {
            a.1.partial_cmp(&b.1)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.2.partial_cmp(&b.2).unwrap_or(std::cmp::Ordering::Equal))
        });
        
        // Find the position of our node_id in the sorted list
        source_nodes.iter()
            .position(|(id, _, _)| *id == node_id)
            .map(|pos| pos + 1) // 1-based numbering
    }

    pub fn start_drawing_connection(&mut self, source_node_id: usize, port_page_x: f64, port_page_y: f64, canvas: &CanvasState) {
        if self.nodes.contains_key(&source_node_id) {
            let source_port_world_pos = canvas.page_to_world_coords(port_page_x, port_page_y);
            self.drawing_connection_state = ConnectionDrawingState {
                active: true,
                source_node_id,
                source_port_world_pos,
                current_mouse_world_pos: source_port_world_pos,
                target_node_id: None,
            };
        }
    }
    
    pub fn redirect_connection(&mut self, target_node_id: usize) {
        let Some((_, connection)) = self.connections.iter().find(|(_, conn)| conn.to_node_id == target_node_id) else {
            return;
        };
        let source_node_id = connection.from_node_id;
        
        let current_mouse_pos = if self.drawing_connection_state.active {
            self.drawing_connection_state.current_mouse_world_pos
        } else if let Some(target_node) = self.nodes.get(&target_node_id) {
            get_port_world_pos(target_node, "input")
        } else {
            (0.0, 0.0)
        };
        
        let Some(source_node) = self.nodes.get(&source_node_id) else {
            return;
        };
        let source_port_world_pos = get_port_world_pos(source_node, "output");
        self.drawing_connection_state = ConnectionDrawingState {
            active: true,
            source_node_id,
            source_port_world_pos,
            current_mouse_world_pos: current_mouse_pos,
            target_node_id: None,
        };
        self.remove_connection_by_target_node(target_node_id);
    }

    pub fn update_drawing_connection(&mut self, mouse_page_x: f64, mouse_page_y: f64, canvas: &CanvasState) {
        let mouse_world_pos = canvas.page_to_world_coords(mouse_page_x, mouse_page_y);
        self.drawing_connection_state.current_mouse_world_pos = mouse_world_pos;
    }
    
    pub fn update_node_output(&mut self, node_id: usize, new_output: String) {
        if let Some(node) = self.nodes.get_mut(&node_id) {
            node.output = Some(new_output);
            node.needs_execution = false;
        }
        self.propagate_output_to_connected_nodes(&node_id);
    }
    
    fn propagate_output_to_connected_nodes(&mut self, source_node_id: &usize) {
        // Find all connections where this node is the source
        let target_node_ids: Vec<usize> = self.connections
            .values()
            .filter(|conn| &conn.from_node_id == source_node_id)
            .map(|conn| conn.to_node_id)
            .collect();
        
        // Update input of all connected target nodes
        for target_id in target_node_ids {
            self.update_node_input_from_all_sources(&target_id);
            if let Some(node) = self.nodes.get_mut(&target_id) {
                node.needs_execution = true;
            }
        }
    }
    
    fn update_node_input_from_all_sources(&mut self, target_node_id: &usize) {
        // Find all source nodes connected to this target
        let mut source_data: Vec<(f64, f64, String)> = self.connections
            .values()
            .filter(|conn| &conn.to_node_id == target_node_id)
            .filter_map(|conn| {
                self.nodes.get(&conn.from_node_id)
                    .and_then(|node| node.output.as_ref()
                    .map(|output| 
                        (node.position_y, node.position_x, output.clone())
                    )
                )
            })
            .collect();
        
        // Sort by position: top-to-bottom (y), then left-to-right (x)
        source_data.sort_by(|a, b| {
            a.0.partial_cmp(&b.0)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
        });
        
        // Concatenate all inputs
        let combined_input = if source_data.is_empty() {
            None
        } else {
            Some(source_data.into_iter().map(|(_, _, output)| output).collect::<Vec<_>>().join("\n\n"))
        };
    
        // Update the target node's input
        if let Some(target_node) = self.nodes.get_mut(target_node_id) {
            target_node.input = combined_input;
        }
    }

    pub fn complete_drawing_connection(&mut self) -> Option<Result<usize, String>> {
        let target_id = match self.drawing_connection_state.target_node_id {
            Some(id) => id,
            None => {
                self.cancel_drawing_connection();
                return Some(Err("No valid target found for connection".to_string()));
            }
        };
    
        let result = if self.nodes.contains_key(&self.drawing_connection_state.source_node_id) {
            Some(self.add_connection(self.drawing_connection_state.source_node_id, target_id))
        } else {
            Some(Err("Source node for connection no longer exists".to_string()))
        };
    
        self.cancel_drawing_connection();
        result
    }

    pub fn cancel_drawing_connection(&mut self) {
        self.drawing_connection_state = ConnectionDrawingState::default();
    }
    
    pub fn set_connection_target(&mut self, target_id: usize) {
        println!("we are setting source node: {}, target id: {}", self.drawing_connection_state.source_node_id, target_id);
        if self.drawing_connection_state.source_node_id != target_id  {
            self.drawing_connection_state.target_node_id = Some(target_id);
        }
    }
    
    pub fn clear_connection_target(&mut self) {
        self.drawing_connection_state.target_node_id = None;
    }
    
    pub fn execution_order(&self) -> Vec<usize> {
        // Get all model node IDs
        let model_nodes: Vec<usize> = self.nodes
            .iter()
            .filter_map(|(id, node)| match node.node_type {
                NodeType::Model { .. } if node.needs_execution => Some(*id),
                _ => None,
            })
            .collect();
        
        // Build dependency map: node_id -> set of model nodes it depends on
        let mut dependencies: HashMap<usize, HashSet<usize>> = 
            model_nodes.iter().map(|&id| (id, HashSet::new())).collect();

        for connection in self.connections.values() {
            if model_nodes.contains(&connection.to_node_id) && model_nodes.contains(&connection.from_node_id) {
                dependencies.get_mut(&connection.to_node_id).unwrap().insert(connection.from_node_id);
            }
        }

        // Topological sort
        let mut result = Vec::new();
        let mut remaining = dependencies;

        while !remaining.is_empty() {
            // Find nodes with no dependencies
            let ready: Vec<usize> = remaining
                .iter()
                .filter(|(_, deps)| deps.is_empty())
                .map(|(&id, _)| id)
                .collect();

            if ready.is_empty() {
                break; // Circular dependency - return partial result
            }

            // Add ready nodes to result and remove from remaining
            for &node_id in &ready {
                result.push(node_id);
                remaining.remove(&node_id);
            }

            // Remove completed nodes from other dependencies
            for deps in remaining.values_mut() {
                for &completed in &ready {
                    deps.remove(&completed);
                }
            }
        }
        result
    }
}

// Main Workflow Management Component
#[component]
pub fn WorkflowManager(
    canvas_state: Signal<CanvasState>,
    workflow_state: Signal<Workflow>
) -> Element {
    // Prepare render data
    let (current_nodes, node_ids_to_render, connections_to_render, drawing_state) = {
        let ws_read = workflow_state.read();
        (
            ws_read.nodes.clone(),
            ws_read.nodes.keys().cloned().collect::<Vec<_>>(),
            ws_read.connections.clone(),
            ws_read.drawing_connection_state.clone(),
        )
    };

    rsx! {
        div {
            ConnectionsRenderer {
                nodes: current_nodes,
                connections: connections_to_render,
                drawing_state: drawing_state,
            }
            
            // Render all nodes
            for node_id in node_ids_to_render {
                NodeComponent {
                    key: "{node_id}",
                    node_id: node_id,
                    workflow_state: workflow_state,
                    canvas_state: canvas_state,
                }
            }
        }
    }
}