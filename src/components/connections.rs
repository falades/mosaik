use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::components::nodes::Node;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Connection {
    pub id: usize,
    pub from_node_id: usize,
    pub to_node_id: usize,
}

#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
pub struct ConnectionDrawingState {
    pub active: bool,
    pub source_node_id: usize,
    pub source_port_world_pos: (f64, f64),
    pub current_mouse_world_pos: (f64, f64),
    pub target_node_id: Option<usize>,
}

impl ConnectionDrawingState {
    fn build_path(from_x: f64, from_y: f64, to_x: f64, to_y: f64) -> String {
        format!("M {},{} L {},{}", from_x + 5000.0, from_y + 5000.0, to_x + 5000.0, to_y + 5000.0 )
    }
    
    fn get_drawing_line_path(&self) -> String {
        let (from_x, from_y) = self.source_port_world_pos;
        let (to_x, to_y) = self.current_mouse_world_pos;
        Self::build_path(from_x, from_y, to_x, to_y)
    }
}

#[component]
pub fn ConnectionsRenderer(
    nodes: HashMap<usize, Node>,
    connections: HashMap<usize, Connection>,
    drawing_state: ConnectionDrawingState,
) -> Element {
    let connection_lines = connections.values().filter_map(|conn| {
        let from_node = nodes.get(&conn.from_node_id)?;
        let to_node = nodes.get(&conn.to_node_id)?;

        let (from_x, from_y) = get_port_world_pos(from_node, "output");
        let (to_x, to_y) = get_port_world_pos(to_node, "input");
        
        let path_data = ConnectionDrawingState::build_path(from_x, from_y, to_x, to_y);

        Some(rsx! {
            path {
                key: "conn-{conn.id}",
                d: "{path_data}",
                stroke: "var(--text-link)",
                "stroke-width": "2",
                fill: "none",
            }
        })
    });

    let drawing_line = drawing_state.active.then(|| {
        let path_data = drawing_state.get_drawing_line_path();
        
        rsx! {
            path {
                d: "{path_data}",
                stroke: "var(--text-link)",
                "stroke-width": "2.5",
                fill: "none",
            }
        }
    });

    rsx! {
        svg {
            style: "position: absolute; top: -5000px; left: -5000px; width: 10000px; height: 10000px; pointer-events: none; z-index: 5;",
            xmlns: "http://www.w3.org/2000/svg",
            {connection_lines}
            {drawing_line}
        }
    }
}

pub fn get_port_world_pos(node: &Node, port_type: &str) -> (f64, f64) {
    match port_type {
        "input" => (
            node.position_x,
            node.position_y + node.height / 2.0,
        ),
        "output" => (
            node.position_x + node.width,
            node.position_y + node.height / 2.0,
        ),
        _ => (node.position_x, node.position_y),
    }
}