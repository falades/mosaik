use dioxus::prelude::*;
use crate::components::nodes::ProviderType;
use crate::components::{
    workflow::Workflow,
    nodes::NodeType,
};

// Canvas state
#[derive(Default, Clone)]
pub struct CanvasState {
    pub offset_x: f64,
    pub offset_y: f64,
    pub zoom: f64,
    pub dragging: bool,
    pub drag_start_x: f64,
    pub drag_start_y: f64,
    pub last_offset_x: f64,
    pub last_offset_y: f64,
}

impl CanvasState {
    /// Converts page coordinates (e.g., mouse events) to world coordinates.
    pub fn page_to_world_coords(&self, page_x: f64, page_y: f64) -> (f64, f64) {
        (
            (page_x - self.offset_x) / self.zoom,
            (page_y - self.offset_y) / self.zoom,
        )
    }

    /// Converts world coordinates to page coordinates.
    /// (Useful for positioning elements based on world coords)
    pub fn world_to_page_coords(&self, world_x: f64, world_y: f64) -> (f64, f64) {
        (
            world_x * self.zoom + self.offset_x,
            world_y * self.zoom + self.offset_y,
        )
    }
}

#[component]
pub fn Canvas(
    canvas_state: Signal<CanvasState>,
    workflow_state: Signal<Workflow>,
    children: Element
) -> Element {
    let mut context_menu_visible = use_signal(|| false);
    let mut context_menu_pos_x = use_signal(|| 0.0);
    let mut context_menu_pos_y = use_signal(|| 0.0);
    let mut last_right_click_page_pos = use_signal(|| (0.0, 0.0));
    
    let on_add_node_from_menu = move |node_type: NodeType| {
        let cs = canvas_state.read();
        let (page_x, page_y) = *last_right_click_page_pos.read();
        let world_pos = cs.page_to_world_coords(page_x, page_y);
        workflow_state.write().add_node(node_type, world_pos.0, world_pos.1);
    };
    // Mouse event handlers for panning
    let on_mouse_down = move |event: Event<MouseData>| {
        // Match on the result of trigger_button()
        match event.trigger_button() {
            // --- Case 1: A button was pressed ---
            Some(button) => {
                match button {
                    // --- Subcase 1.1: Right Mouse Button (Secondary) ---
                    dioxus::html::input_data::MouseButton::Secondary => {
                        // Get coordinates relative to the element receiving the event
                        let coords = event.element_coordinates();

                        // Set position for the context menu
                        context_menu_pos_x.set(coords.x);
                        context_menu_pos_y.set(coords.y);
                        
                        // Make the context menu visible
                        context_menu_visible.set(true);

                        last_right_click_page_pos.set((coords.x, coords.y));
                    }
                    // --- Subcase 1.2: Left Mouse Button (Primary) ---
                    dioxus::html::input_data::MouseButton::Primary => {
                        if context_menu_visible() {
                            context_menu_visible.set(false);
                        }

                        let mut state = canvas_state.write();
                        state.dragging = true;
                        let start_coords = event.element_coordinates();
                        state.drag_start_x = start_coords.x;
                        state.drag_start_y = start_coords.y;
                        // Store the offset *before* this drag started
                        state.last_offset_x = state.offset_x;
                        state.last_offset_y = state.offset_y;
                    }
                    // --- Subcase 1.3: Other Mouse Buttons (Middle, Back, Forward) ---
                    _ => {}
                }
            }
            None => {}
        }
    };

    let on_mouse_move = move |event: Event<MouseData>| {
        let cs_read = canvas_state.read();
        let ws_read = workflow_state.read();
        
        // Handle connection drawing
        if ws_read.drawing_connection_state.active {
            let (mouse_x, mouse_y) = event.page_coordinates().into();
            drop(ws_read);
            workflow_state.write().update_drawing_connection(mouse_x, mouse_y, &cs_read);
            return;
        }
        
        // Handle node dragging
        if ws_read.dragging_node_id.is_some() {
            let (mouse_x, mouse_y) = event.page_coordinates().into();
            drop(ws_read);
            drop(cs_read);
            workflow_state.write().drag_node(mouse_x, mouse_y, &canvas_state.read());
            return;
        }
        
        // Handle canvas panning
        if cs_read.dragging {
            let dx = event.data().client_coordinates().x - cs_read.drag_start_x;
            let dy = event.data().client_coordinates().y - cs_read.drag_start_y;
            let new_x = cs_read.last_offset_x + dx;
            let new_y = cs_read.last_offset_y + dy;
            
            drop(cs_read);
            
            let mut state_write = canvas_state.write();
            state_write.offset_x = new_x;
            state_write.offset_y = new_y;
        }
    };

    let on_mouse_up = move |event: Event<MouseData>| {
        let mut ws = workflow_state.write();
        
        // If we're in connection drawing mode and not over a valid target
        if ws.drawing_connection_state.active {
            if ws.drawing_connection_state.target_node_id.is_none() {
                let coords = event.element_coordinates();
                
                // Get position for the context menu
                context_menu_pos_x.set(coords.x);
                context_menu_pos_y.set(coords.y);
                
                // Store the position for potential node creation
                last_right_click_page_pos.set((coords.x, coords.y));
                
                // Make the context menu visible
                context_menu_visible.set(true);
                
                ws.cancel_drawing_connection();
            } else if let Some(result) = ws.complete_drawing_connection() {
                    match result {
                        Ok(conn_id) => dioxus::logger::tracing::info!("Connection {} created", conn_id),
                        Err(e) => dioxus::logger::tracing::warn!("Failed to create connection: {}", e),
                }
            }
        } 
        
        // Handle node drag end
        if ws.dragging_node_id.is_some() {
            ws.end_dragging_node();
        }
        
        // Handle canvas drag end
        canvas_state.write().dragging = false;
    };

    // Wheel event handler for zooming
    let on_wheel = move |event: Event<WheelData>| {
        event.stop_propagation();
        
        let mouse_x = event.data().element_coordinates().x;
        let mouse_y = event.data().element_coordinates().y;
        
        let wheel_delta = match event.data().delta() {
            dioxus::html::geometry::WheelDelta::Pixels(vector) => vector.y * -0.01,
            dioxus::html::geometry::WheelDelta::Lines(vector) => vector.y * -0.05,
            dioxus::html::geometry::WheelDelta::Pages(vector) => vector.y * -0.2,
        };
        
        let zoom_factor = 1.0 + wheel_delta;
        
        // Get current state values
        let state_read = canvas_state.read();
        let current_offset_x = state_read.offset_x;
        let current_offset_y = state_read.offset_y;
        let old_zoom = state_read.zoom;
        drop(state_read);
        
        // Calculate new values
        let new_zoom = (old_zoom * zoom_factor).clamp(0.1, 10.0);
        
        let world_mouse_x = (mouse_x - current_offset_x) / old_zoom;
        let world_mouse_y = (mouse_y - current_offset_y) / old_zoom;
        
        let new_offset_x = mouse_x - world_mouse_x * new_zoom;
        let new_offset_y = mouse_y - world_mouse_y * new_zoom;
        
        // Update state with new values
        let mut state_write = canvas_state.write();
        state_write.offset_x = new_offset_x;
        state_write.offset_y = new_offset_y;
        state_write.zoom = new_zoom;
    };

    // Get render values from state
    let state = canvas_state.read();
    let transform_style = format!(
        "transform: translate({}px, {}px) scale({}); transform-origin: 0 0;",
        state.offset_x, state.offset_y, state.zoom
    );
    drop(state);
    
    // Render the canvas with event handlers
    rsx! {
        div {
            class: "canvas-background",
            style: "width: 100%; height: 100%; position: absolute; overflow: hidden;",
            onmousedown: on_mouse_down,
            onmousemove: on_mouse_move,
            onmouseup: on_mouse_up,
            onmouseleave: on_mouse_up, 
            onwheel: on_wheel,
            
            div {
                class: "canvas-elements",
                style: "position: absolute; {transform_style}",
                {children} 
            }
            
            if *context_menu_visible.read() {
                // Context menu
                CanvasContextMenu {
                    visible: context_menu_visible,
                    position_x: context_menu_pos_x,
                    position_y: context_menu_pos_y,
                    on_add_node: on_add_node_from_menu
                }
            }
        }
    }
}

// Context Menu Component for adding nodes
#[component]
pub fn CanvasContextMenu(
    visible: Signal<bool>,
    position_x: Signal<f64>,
    position_y: Signal<f64>,
    on_add_node: EventHandler<NodeType>,
) -> Element {
    let mut show_models_submenu = use_signal(|| false);
    let mut show_file_submenu = use_signal(|| false);
    let mut show_prompt_submenu = use_signal(|| false);
    
    let menu_item_style = "padding: 8px 15px; cursor: pointer; user-select: none; \
        display: flex; align-items: center; justify-content: space-between;";
    
    let menu_style = format!(
        "position: absolute; top: {}px; left: {}px; background-color: var(--text-primary); \
        border: 1px solid #ccc; box-shadow: 0 2px 5px rgba(0,0,0,0.2); \
        z-index: 200; padding: 5px 0; border-radius: 8px; min-width: 150px; \
        font-family: system-ui, -apple-system, sans-serif; font-size: 14px;",
        position_y(), position_x()
    );

    let models_submenu_style = format!(
        "position: absolute; top: 32; left: 100%; background-color: var(--text-primary); \
        border: 1px solid #ccc; box-shadow: 0 2px 5px rgba(0,0,0,0.2); \
        z-index: 201; padding: 5px 0; border-radius: 8px; min-width: 170px; \
        display: {};",
        if show_models_submenu() { "block" } else { "none" }
    );

    let file_submenu_style = format!(
        "position: absolute; top: 64px; left: 100%; background-color: var(--text-primary); \
        border: 1px solid #ccc; box-shadow: 0 2px 5px rgba(0,0,0,0.2); \
        z-index: 201; padding: 5px 0; border-radius: 8px; min-width: 170px; \
        display: {};",
        if show_file_submenu() { "block" } else { "none" }
    );

    let prompt_submenu_style = format!(
        "position: absolute; top: 0px; left: 100%; background-color: var(--text-primary); \
        border: 1px solid #ccc; box-shadow: 0 2px 5px rgba(0,0,0,0.2); \
        z-index: 201; padding: 5px 0; border-radius: 8px; min-width: 170px; \
        display: {};",
        if show_prompt_submenu() { "block" } else { "none" }
    );

    rsx! {
        div {
            style: "{menu_style}",
            onmousedown: move |event| {
                event.stop_propagation();
            },
            // Prompt menu
            div {
                style: "{menu_item_style}",
                onmouseenter: move |_| show_prompt_submenu.set(true),
                onmouseleave: move |_| show_prompt_submenu.set(false),
                span { 
                    style: "color: var(--ui)",
                    "Prompt" 
                }
                span {
                    style: "color: var(--ui);",
                    "➤"
                }
                div {
                    style: "{prompt_submenu_style}",
                    onmouseenter: move |_| show_prompt_submenu.set(true),
                    div {
                        style: "{menu_item_style}",
                        onclick: move |_| {
                            on_add_node.call(NodeType::Prompt {});
                            visible.set(false);
                        },
                        span {
                            style: "color: var(--ui);",
                            "Prompt"
                        }
                    }
                }
            }
            // Models menu
            div {
                style: "{menu_item_style}",
                onmouseenter: move |_| show_models_submenu.set(true),
                onmouseleave: move |_| show_models_submenu.set(false),
                span { 
                    style: "color: var(--ui)",
                    "Models" 
                }
                span {
                    style: "color: var(--ui);",
                    "➤"
                }
                div {
                    style: "{models_submenu_style}",
                    onmouseenter: move |_| show_models_submenu.set(true),
                    div {
                        style: "{menu_item_style}",
                        onclick: move |_| {
                            on_add_node.call(NodeType::Model { provider: ProviderType::Ollama, model_name: "".to_string(), messages: Vec::new(), thinking: false });
                            visible.set(false);
                        },
                        span {
                            style: "color: var(--ui);",
                            "Ollama"
                        }
                    }
                    div {
                        style: "{menu_item_style}",
                        onclick: move |_| {
                            on_add_node.call(NodeType::Model { provider: ProviderType::Anthropic, model_name: "".to_string(), messages: Vec::new(), thinking: false });
                            visible.set(false);
                        },
                        span {
                            style: "color: var(--ui);",
                            "Anthropic"
                        }
                    }
                }
            }
            // File menu
            div {
                style: "{menu_item_style}",
                onmouseenter: move |_| show_file_submenu.set(true),
                onmouseleave: move |_| show_file_submenu.set(false),
                span { 
                    style: "color: var(--ui)",
                    "File" 
                }
                span {
                    style: "color: var(--ui);",
                    "➤"
                }
                div {
                    style: "{file_submenu_style}",
                    onmouseenter: move |_| show_file_submenu.set(true),
                    div {
                        style: "{menu_item_style}",
                        onclick: move |_| {
                            on_add_node.call(NodeType::FileImport { file_path: None, file_name: None });
                            visible.set(false);
                        },
                        span {
                            style: "color: var(--ui);",
                            "Import File"
                        }
                    }
                    div {
                        style: "{menu_item_style}",
                        onclick: move |_| {
                            on_add_node.call(NodeType::FileExport { folder_path: None, file_name: None, file_type: "txt".to_string() });
                            visible.set(false);
                        },
                        span {
                            style: "color: var(--ui);",
                            "Export File"
                        }
                    }
                }
            }
        }
    }
}