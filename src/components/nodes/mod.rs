use dioxus::prelude::*;
use serde::{Serialize, Deserialize};
use crate::components::{
    canvas::CanvasState, connections::get_port_world_pos, workflow::Workflow,
    nodes::{
        model::ModelNode,
        file::{FileImportNode, FileExportNode}
    }
};

pub mod model;
mod file;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    User,
    Assistant,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: MessageRole,
    pub content: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thinking: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProviderType {
    Ollama,
    Anthropic,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum NodeType {
    Prompt {}, 
    FileImport {
        file_path: Option<String>,
        file_name: Option<String>,
    },
    FileExport {
        folder_path: Option<String>,
        file_name: Option<String>,
        file_type: String, // "txt" or "md"
    },
    Model {
        provider: ProviderType,
        model_name: String,
        messages: Vec<ChatMessage>,
        thinking: bool
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Node {
    pub id: usize,
    pub node_type: NodeType, 
    pub position_x: f64,
    pub position_y: f64,
    pub width: f64,
    pub height: f64,
    pub title: String,
    pub input: Option<String>,
    pub output: Option<String>, 
    pub drag_offset_x: f64,
    pub drag_offset_y: f64,
    pub is_maximized: bool,
    pub needs_execution: bool,
    pub is_executing: bool
}

impl Node {
    pub fn new(id: usize, node_type: &NodeType, position_x: f64, position_y: f64) -> Self {
        let (title, width, height, specific_node_type_instance) = match node_type {
            NodeType::Prompt {} => (
                "Prompt".to_string(),
                200.0,
                200.0,
                NodeType::Prompt {},
            ),
            NodeType::FileImport { .. } => (
                "File Import".to_string(),
                200.0,
                150.0,
                NodeType::FileImport {
                    file_path: None,
                    file_name: None,
                },
            ),
            NodeType::FileExport { .. } => (
                "File Export".to_string(),
                200.0,
                150.0,
                NodeType::FileExport {
                    folder_path: None,
                    file_name: None,
                    file_type: "txt".to_string(),
                },
            ),
            NodeType::Model { provider, .. } => {
                let (title, model_name) = match provider {
                    ProviderType::Ollama => ("Ollama", ""),
                    ProviderType::Anthropic => ("Anthropic", "claude-sonnet-4-20250514"),
                };
                
                (
                    title.to_string(),
                    250.0,
                    300.0,
                    NodeType::Model {
                        provider: provider.clone(),
                        model_name: model_name.to_string(),
                        messages: Vec::new(),
                        thinking: false
                    },
                )
            },
        };

        Node {
            id,
            node_type: specific_node_type_instance,
            position_x,
            position_y,
            width,
            height,
            title,
            input: None,
            output: None,
            drag_offset_x: 0.0,
            drag_offset_y: 0.0,
            is_maximized: false,
            needs_execution: true,
            is_executing: false
        }
    }
    
    pub fn prepare_prompt(&self) -> anyhow::Result<Vec<ChatMessage>> {
        let chat_messages = match &self.node_type {
            NodeType::Model { messages, .. } => messages,
            _ => return Err(anyhow::anyhow!("prepare_prompt called on non-model node")),
        };
    
        let mut messages = Vec::new();
        if let Some(input) = &self.input {
            if !input.trim().is_empty() {
                messages.push(ChatMessage {
                    role: MessageRole::User,
                    content: input.clone(),
                    thinking: None
                });
            }
        }
        messages.extend(chat_messages.clone());
        if messages.is_empty() {
            return Err(anyhow::anyhow!("No input or messages provided to model"));
        }
        Ok(messages)
    }
    pub fn reset(&mut self) {
        self.output = None;
        self.needs_execution = true;
        match &mut self.node_type {
            NodeType::Model { messages, .. } => {
                messages.clear();
            },
            NodeType::FileImport { file_path, file_name } => {
                *file_path = None;
                *file_name = None;
            },
            NodeType::FileExport { folder_path, file_name, file_type } => {
                *folder_path = None;
                *file_name = None;
                *file_type = "txt".to_string();
            },
            _ => {}
        }
    }
}

#[component]
pub fn NodeComponent(
    node_id: usize,
    workflow_state: Signal<Workflow>,
    canvas_state: Signal<CanvasState>,
) -> Element {
    let mut node_context_menu_visible = use_signal(|| false);
    let mut node_context_menu_pos_x = use_signal(|| 0.0);
    let mut node_context_menu_pos_y = use_signal(|| 0.0);
    let current_node_opt = use_memo(move || workflow_state.read().nodes.get(&node_id).cloned());

    let node = current_node_opt().unwrap();
    
    // Event handlers
    let mut on_node_select = move |id| workflow_state.write().selected_node_id = Some(id);
    let mut on_start_drag = move |(id, event_data): (usize, Event<MouseData>)| {
        event_data.prevent_default();
        
        let (mouse_x, mouse_y) = event_data.page_coordinates().into();
        workflow_state.write().start_dragging_node(id, mouse_x, mouse_y, &canvas_state.read());
    };
    let on_mouse_down = move |event: Event<MouseData>| {
        match event.trigger_button() {
            Some(dioxus::html::input_data::MouseButton::Secondary) => {
                event.stop_propagation();
                let coords = event.element_coordinates();
                node_context_menu_pos_x.set(coords.x);
                node_context_menu_pos_y.set(coords.y);
                node_context_menu_visible.set(true);
            }
            _ => {
                event.stop_propagation();
                if node_context_menu_visible() {
                    node_context_menu_visible.set(false);
                }
                on_node_select(node.id);
                on_start_drag((node.id, event.clone()));
            }
        }
    };
    let on_start_connection = move |(id, event_data): (usize, Event<MouseData>)| {
        event_data.prevent_default();
        let mut ws_writer = workflow_state.write();
        let cs_reader = canvas_state.read();
    
        if let Some(node) = ws_writer.nodes.get(&id) {
            let (port_world_x, port_world_y) = get_port_world_pos(node, "output");
            let (port_center_page_x, port_center_page_y) = cs_reader.world_to_page_coords(port_world_x, port_world_y);
            ws_writer.start_drawing_connection(id, port_center_page_x, port_center_page_y, &cs_reader);
        }
    };
    let on_connection_redirect = move |(id, event_data): (usize, Event<MouseData>)| {
        event_data.prevent_default();
        workflow_state.write().redirect_connection(id);
    };
    let on_delete = move |_| {
        workflow_state.write().remove_node(node_id);
    };
    let on_reset = move |_| {
        if let Some(node) = workflow_state.write().nodes.get_mut(&node_id) {
            node.reset();
        }
    };
    let mut on_toggle_maximize = move |id: usize| {
        let mut ws_write = workflow_state.write();
        if let Some(node) = ws_write.nodes.get_mut(&id) {
            node.is_maximized = !node.is_maximized; 
        }
    };

    let input_order_number = use_memo(move || {
        let ws = workflow_state.read();
        // Find all nodes this node connects to as a source
        let target_node_ids: Vec<usize> = ws.connections
            .values()
            .filter(|conn| conn.from_node_id == node_id)
            .map(|conn| conn.to_node_id)
            .collect();
        
        // Check each target - if any has multiple inputs, return our order number
        for target_id in target_node_ids {
            if let Some(order) = ws.get_input_order_number(node_id, target_id) {
                return Some(order);
            }
        }
        None
    });

    let canvas_zoom = canvas_state.read().zoom;

    let (node_position_x, node_position_y, node_width, node_height) = if node.is_maximized {
        let scale_factor = 3.0;
        let new_width = node.width * scale_factor;
        let new_height = node.height * scale_factor;
        
        let centered_x = node.position_x - (new_width - node.width) / 2.0;
        
        (centered_x, node.position_y, new_width, new_height)
    }  else {
        (node.position_x, node.position_y, node.width, node.height)
    };
    
    let executing_border = if node.is_executing {
        "background-image: linear-gradient(90deg, var(--text-link) 0%, var(--text-link) 100%);
        background-size: 0% 3px;
        background-repeat: no-repeat;
        background-position: bottom left;
        animation: executing 2s ease-in-out infinite;"
    } else {
        ""
    };
    
    let node_style = format!(
        "position: absolute; left: {}px; top: {}px; width: {}px; height: {}px; \
        background-color: var(--ui); border-radius: 8px; \
        display: flex; flex-direction: column; overflow: hidden; z-index: {};",
        node_position_x, node_position_y, node_width, node_height,
        if node.is_maximized { 1000 } else { 1 }
    );

    let body = match &node.node_type {
        NodeType::Prompt {} => {
            rsx! {
                PromptNode {
                    node: node.clone(),
                    workflow_state,
                    canvas_zoom,
                    on_start_connection,
                    on_connection_redirect,
                }
            }
        }
        NodeType::FileImport { .. } => {
            rsx! {
                FileImportNode {
                    node: node.clone(),
                    workflow_state,
                    canvas_zoom,
                    on_start_connection,
                    on_connection_redirect,
                }
            }
        }
        NodeType::FileExport { .. } => {
            rsx! {
                FileExportNode {
                    node: node.clone(),
                    workflow_state,
                    canvas_zoom,
                    on_connection_redirect,
                }
            }
        }
        NodeType::Model { provider, model_name, messages, .. } => {
            rsx!{
                ModelNode { 
                    node: node.clone(),
                    workflow_state,
                    canvas_zoom,
                    on_start_connection,
                    on_connection_redirect,
                    provider: provider.clone(),
                    model_name,
                    messages: messages.clone()
                }
            }
        }
    };

    rsx! {
        div {
            class: "node",
            style: "{node_style}",
            onmousedown: on_mouse_down,
            onmouseenter: move |_| {
                let mut ws = workflow_state.write();
                if ws.drawing_connection_state.active {
                    ws.set_connection_target(node.id);
                }
            },
            onmouseleave: move |event| {
                event.prevent_default();
                event.stop_propagation();
                let mut ws = workflow_state.write();
                if ws.drawing_connection_state.target_node_id == Some(node.id) {
                    ws.clear_connection_target();
                }
            },
            div {
                class: "node-header",
                style: format!("position: relative; padding: 8px 12px; background-color: var(--ui); font-weight: bold; user-select: none; text-align: center; {}", executing_border),
                "{node.title}"
                
                // Order number badge
                if let Some(order) = input_order_number() {
                    div {
                        style: "position: absolute; top: 0px; left: 0px; color: var(--text-link); 
                               width: 24px; height: 100%; border-radius: 8px; display: flex; align-items: center; 
                               justify-content: center; font-size: 16px; font-weight: bold;",
                        "{order}"
                    }
                }
                
                div {
                    style: "position: absolute; top: 50%; right: 8px; transform: translateY(-50%); 
                            display: flex; gap: 4px;",
                   
                    // Maximize button
                    div {
                        style: "width: 20px; height: 20px; cursor: pointer; display: flex; align-items: center; 
                                justify-content: center; font-size: 14px;",
                        onmousedown: move |event| {
                            event.stop_propagation();
                            on_toggle_maximize(node.id);
                        },
                        "⤢" 
                    }
                }
            }
            div { class: "node-content", style: "flex: 1; padding: 10px; display: flex; flex-direction: column; overflow-y: auto;",
                {body}
            }
            if *node_context_menu_visible.read() {
                NodeContextMenu {
                    visible: node_context_menu_visible,
                    position_x: node_context_menu_pos_x,
                    position_y: node_context_menu_pos_y,
                    on_delete,
                    on_reset
                }
            }
        }
    }
}

#[component]
fn PromptNode(
    node: Node,
    workflow_state: Signal<Workflow>,
    canvas_zoom: f64,
    on_start_connection: EventHandler<(usize, Event<MouseData>)>,
    on_connection_redirect: EventHandler<(usize, Event<MouseData>)>,
) -> Element {
    let mut on_context_content_change = move |(target_node_id, new_content): (usize, String)| {
        workflow_state.write().update_node_output(target_node_id, new_content);
    };

    let content = node.output.as_deref().unwrap_or("").to_string();
    
    rsx! {
        NodeSocket{
            node_id: node.id,
            canvas_zoom,
            on_start_connection,
            on_connection_redirect,
            input: false,
            output: true
        }
        
        textarea {
            class: "text-box",
            value: "{content}",
            oninput: move |event| on_context_content_change((node.id, event.value())),
            onmousedown: |evt| evt.stop_propagation(),
            onwheel: |evt| evt.stop_propagation()
        }
    }
}

#[component]
pub fn NodeSocket(
    node_id: usize,
    canvas_zoom: f64,
    on_start_connection: EventHandler<(usize, Event<MouseData>)>,
    on_connection_redirect: EventHandler<(usize, Event<MouseData>)>,
    input: bool,
    output: bool
) -> Element {
    let socket_size = (12.0 / canvas_zoom).clamp(12.0, 25.0);
    
    let socket_style = format!(
        "width: {}px; height: {}px; border-radius: 4px; position: absolute; \
        cursor: crosshair; background-color: var(--text-link); top: 50%;",
        socket_size, socket_size
    );
    
    rsx! {
        if input {
            div {
                class: "node-input-socket",
                style: "{socket_style} left: 0; transform: translate(-50%, -50%);",
                onmousedown: move |event| {
                    event.stop_propagation();
                    on_connection_redirect.call((node_id, event.clone()));
                }
            }
        }
        
        if output {
            div {
                class: "node-output-socket",
                style: "{socket_style} right: 0; transform: translate(50%, -50%);",
                onmousedown: move |event| {
                    event.stop_propagation();
                    on_start_connection.call((node_id, event.clone()));
                }
            }
        }
    }
}

#[component]
pub fn NodeContextMenu(
    visible: Signal<bool>,
    position_x: Signal<f64>,
    position_y: Signal<f64>,
    on_delete: EventHandler<()>,
    on_reset: EventHandler<()>,
) -> Element {
    let menu_item_style = "padding: 8px 15px; cursor: pointer; user-select: none; \
        display: flex; align-items: center; justify-content: space-between;";
    
    let menu_style = format!(
        "position: absolute; top: {}px; left: {}px; background-color: var(--text-primary); \
        border: 1px solid #ccc; box-shadow: 0 2px 5px rgba(0,0,0,0.2); \
        z-index: 200; padding: 5px 0; border-radius: 8px; min-width: 150px; \
        font-family: system-ui, -apple-system, sans-serif; font-size: 14px;",
        position_y(), position_x()
    );

    rsx! {
        div {
            style: "{menu_style}",
            onmousedown: move |event| {
                event.stop_propagation();
            },
            
            div {
                style: "{menu_item_style}",
                onclick: move |_| {
                    on_delete.call(());
                    visible.set(false);
                },
                span {
                    style: "color: var(--ui);",
                    "Delete"
                }
            }
            
            div {
                style: "{menu_item_style}",
                onclick: move |_| {
                    on_reset.call(());
                    visible.set(false);
                },
                span {
                    style: "color: var(--ui);",
                    "Reset"
                }
            }
        }
    }
}