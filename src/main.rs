use dioxus::prelude::*;
use dioxus::desktop::{Config, WindowBuilder};

mod llm;
mod components;
mod file_manager;

use components::{SettingsPopup, Canvas, CanvasState, 
    WorkflowManager, Workflow, NodeType, execute_model_node
};


const MAIN_CSS: Asset = asset!("/assets/main.css");

fn main() {
    #[cfg(feature = "desktop")]
    dioxus::LaunchBuilder::desktop() // From dioxus::desktop::LaunchBuilder
        .with_cfg(
            Config::new() // From dioxus::desktop::Config
                .with_window(
                    WindowBuilder::new()
                        .with_title("mosaik")
                        .with_theme(Some(dioxus::desktop::tao::window::Theme::Dark))
                )
                .with_disable_context_menu(true)
                .with_menu(None)
        )
        .launch(App);
}

#[component]
fn App() -> Element {
    let canvas_state = use_signal(|| CanvasState {
        offset_x: 0.0,
        offset_y: 0.0,
        zoom: 1.0,
        dragging: false,
        drag_start_x: 0.0,
        drag_start_y: 0.0,
        last_offset_x: 0.0,
        last_offset_y: 0.0,
    });
    
    let mut workflow_state = use_signal(|| {
        file_manager::load_default_workflow().unwrap_or_default()
    });
    
    let popup_open = use_signal(|| false);
    
    let run_workflow = move || {
        spawn(async move {
            let execution_order = workflow_state.read().execution_order();
            
            for node_id in execution_order {
                let (provider_type, model_name_clone, thinking_enabled, ollama_messages) = {
                    let mut state = workflow_state.write();
                    let Some(node_to_update) = state.nodes.get_mut(&node_id) else { continue };
                    
                    let NodeType::Model { provider, model_name, thinking, .. } = &mut node_to_update.node_type else { continue };
                    
                    let provider_type = provider.clone();
                    let model_name_clone = model_name.clone();
                    let thinking_enabled = *thinking;
                    node_to_update.is_executing = true;
                    
                    match node_to_update.prepare_prompt() {
                        Ok(ollama_messages) => (provider_type, model_name_clone, thinking_enabled, ollama_messages),
                        Err(e) => {
                            println!("Failed to prepare prompt for node {}: {}", node_id, e);
                            node_to_update.is_executing = false;
                            continue;
                        }
                    }
                };
                
                execute_model_node(
                    workflow_state,
                    node_id,
                    ollama_messages,
                    provider_type,
                    Some(model_name_clone),
                    Some(thinking_enabled)
                ).await;
            }
        });
    };
    
    rsx! {
        document::Link { rel: "stylesheet", href: MAIN_CSS }
        
        div {
            class: "app-container",
            tabindex: "0",
            onkeydown: move |event| {
                if event.key() == Key::Enter && event.modifiers().ctrl() {
                    event.prevent_default();
                    run_workflow();
                }
            },
            Toolbar {
                workflow_state,
                popup_open,
                run_workflow
            }
            
            // Canvas container with our Canvas component
            div {
                class: "canvas-container",
                Canvas {
                    canvas_state,
                    workflow_state,
                    WorkflowManager {canvas_state, workflow_state}
                }
            }
            if *popup_open.read() {
                SettingsPopup {
                    popup_open,
                }
            } 
        }
    }
}

#[component]
fn Toolbar(
    workflow_state: Signal<Workflow>,
    popup_open: Signal<bool>,
    run_workflow: EventHandler<()>,
) -> Element {
    let mut toggle_panel = move || {
        popup_open.set(!popup_open());
    };

    rsx! {
        div {
            class: "toolbar",
            style: "position: absolute; top: 10px; right: 10px; z-index: 100; display: flex; gap: 2px;",
            
            button {
                class: "run-button",
                style: "padding: 8px 16px; background: transparent; color: var(--text-primary); 
                    border: none; cursor: pointer; font-weight: bold;
                    display: flex; flex-direction: column; align-items: center; gap: 2px;",
                onclick: move |_| run_workflow(()),
                div { "Run" }
                div { 
                    style: "font-size: 10px; font-weight: normal;",
                    "( Ctrl + ⏎ )"
                }
            }
            
            button {
                class: "save-button",
                style: "padding: 8px 16px; background: transparent; color: var(--text-primary); 
                    border: none; cursor: pointer;",
                onclick: move |_| {
                    let workflow = workflow_state.read();
                    if let Err(e) = file_manager::save_default_workflow(&workflow) {
                        println!("Failed to save workflow: {}", e);
                    } else {
                        println!("Workflow saved successfully");
                    }
                },
                "Save"
            }
            
            button {
                class: "load-button", 
                style: "padding: 8px 16px; background: transparent; color: var(--text-primary); 
                    border: none; cursor: pointer;",
                "Load"
            }
            
            button {
                class: "settings-button",
                style: "padding: 8px 12px; background: transparent; color: var(--text-primary); 
                    border: none; cursor: pointer;",
                onclick: move |_| toggle_panel(),
                "⚙"
            }
        }
    }
}