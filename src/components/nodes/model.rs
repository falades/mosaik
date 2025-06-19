use dioxus::prelude::*;
use crate::{
    components::{
        workflow::Workflow,
        nodes::{Node, NodeType, NodeSocket, ChatMessage, MessageRole, ProviderType}
    }, 
    llm::{AnthropicClient, ApiManager, LLMProvider, OllamaClient}
};

#[component]
pub fn ModelNode(
    node: Node,
    workflow_state: Signal<Workflow>,
    canvas_zoom: f64,
    on_start_connection: EventHandler<(usize, Event<MouseData>)>,
    on_connection_redirect: EventHandler<(usize, Event<MouseData>)>,
    provider: ProviderType,
    model_name: String,
    messages: Vec<ChatMessage>
) -> Element {
    let mut current_message = use_signal(|| "".to_string());
    let mut available_models = use_signal(Vec::<String>::new);
    let mut show_thoughts = use_signal(|| false);
    let mut api_key_available = use_signal(|| false);
    
    // Fetch models when component mounts or provider changes
    use_effect(move || {
        let provider_clone = provider.clone();
        spawn(async move {
            let api_manager = match ApiManager::new() {
                Ok(manager) => manager,
                Err(_) => {
                    api_key_available.set(false);
                    return;
                }
            };
            let models = match provider_clone {
                ProviderType::Ollama => {
                    let client = OllamaClient::new();
                    client.get_available_models().await.unwrap_or_else(|_| vec!["".to_string()])
                },
                ProviderType::Anthropic => {
                    if api_manager.get_anthropic_key().is_err() {
                        api_key_available.set(false);
                        return;
                    }
                    let client = AnthropicClient::new();
                    client.get_available_models().await.unwrap_or_else(|_| vec!["claude-sonnet-4-20250514".to_string()])
                }
            };
            api_key_available.set(true);
            available_models.set(models);
        });
    });
    
    let mut on_model_change = move |(target_node_id, new_model): (usize, String)| {
        let mut ws = workflow_state.write();
        if let Some(node_to_update) = ws.nodes.get_mut(&target_node_id) {
            if let NodeType::Model { model_name, .. } = &mut node_to_update.node_type {
                *model_name = new_model;
            }
        }
    };
    
    let mut on_send_message = move |(target_node_id, message_content): (usize, String)| {
        let (provider_type, model_name_clone, thinking_enabled, ollama_messages) = {
            let mut ws = workflow_state.write();
            let Some(node_to_update) = ws.nodes.get_mut(&target_node_id) else { return };
            let NodeType::Model { provider, model_name, messages, thinking } = &mut node_to_update.node_type else { return };
            
            messages.push(ChatMessage {
                role: MessageRole::User,
                content: message_content.clone(),
                thinking: None
            });
            
            let provider_type = provider.clone();
            let model_name_clone = model_name.clone();
            let thinking_enabled = *thinking;
            node_to_update.is_executing = true;
            
            match node_to_update.prepare_prompt() {
                Ok(ollama_messages) => (provider_type, model_name_clone, thinking_enabled, ollama_messages),
                Err(e) => {
                    println!("Failed to prepare prompt: {}", e);
                    node_to_update.is_executing = false;
                    return;
                }
            }
        };
        
        spawn(execute_model_node(
            workflow_state,
            target_node_id,
            ollama_messages,
            provider_type,
            Some(model_name_clone),
            Some(thinking_enabled)
        ));
    };
    
    if !api_key_available() {
        return rsx! {
            div {
                style: "color: var(--text-warning); padding: 10px; text-align: center; 
                    background-color: var(--bg-alt); border-radius: 4px;",
                "⚠️ Configure your Anthropic API key in settings to use this provider"
            }
        };
    }
    
    if node.is_maximized {
        rsx! {
            NodeSocket{
                node_id: node.id,
                canvas_zoom,
                on_start_connection,
                on_connection_redirect,
                input: true,
                output: true
            }
            
            div { 
                style: "flex: 1; overflow-y: auto; padding: 10px; margin-bottom: 10px;",
                onwheel: move |event| event.stop_propagation(),
                for message in messages {
                    div {
                        style: match message.role {
                            MessageRole::User => "display: flex; flex-direction: column; align-items: flex-end; margin-bottom: 8px;",
                            MessageRole::Assistant => "display: flex; flex-direction: column; align-items: flex-start; margin-bottom: 8px;",
                        },
                        div {
                            style: "background-color: var(--bg-alt); padding: 8px 12px; border-radius: 12px; max-width: 70%;",
                            if message.role == MessageRole::Assistant && show_thoughts() {
                                if let Some(thinking_content) = &message.thinking {
                                    "{thinking_content}"
                                } else {
                                    "No thoughts available"
                                }
                            } else {
                                "{message.content}"
                            }
                        }
                        if message.role == MessageRole::Assistant && message.thinking.is_some() {
                            button {
                                style: "padding: 2px 6px; background-color: var(--bg-alt); color: var(--text-primary); 
                                    border: 1px solid var(--text-primary); border-radius: 4px; cursor: pointer; font-size: 9px; 
                                    margin-top: 4px;",
                                onclick: move |_| {
                                    show_thoughts.set(!show_thoughts());
                                },
                                onmousedown: |evt| evt.stop_propagation(),
                                if show_thoughts() { "Hide Thoughts" } else { "Show Thoughts" }
                            }
                        }
                    }
                }
            }            
            div {
                style: "display: flex; gap: 8px;",
                textarea {
                    style: "flex: 1; min-height: 40px; resize: none; background: transparent; 
                        border: 1px solid var(--text-primary); border-radius: 4px; padding: 8px; color: var(--text-primary);",
                    placeholder: "Type...",
                    value: "{current_message}",
                    oninput: move |event| {
                        current_message.set(event.value());
                    },
                    onkeydown: move |event| {
                        if event.key() == Key::Enter && event.modifiers().ctrl() {
                            event.prevent_default();
                            event.stop_propagation();
                            let message = current_message();
                            if !message.trim().is_empty() {
                                on_send_message((node.id, message));
                                current_message.set(String::new());
                            }
                        }
                    },
                    onmousedown: |evt| evt.stop_propagation(),
                }
                button {
                    style: "padding: 8px 16px; background-color: var(--text-link); color: var(--text-primary); 
                        border: none; border-radius: 4px; cursor: pointer;
                        display: flex; flex-direction: column; align-items: center; gap: 2px;",
                    onclick: move |_| {
                        let message = current_message();
                        if !message.trim().is_empty() {
                            on_send_message((node.id, message));
                            current_message.set(String::new());
                        }
                    },
                    onmousedown: |evt| evt.stop_propagation(),
                    div { "↑" }
                    div { 
                        style: "font-size: 8px; font-weight: normal;",
                        "( Ctrl + ⏎ )"
                    }
                }
            }
        }
    } else {
        let output_text = if show_thoughts() {
            // Try to get thinking content from the last assistant message
            if let Some(last_message) = messages.last() {
                if last_message.role == MessageRole::Assistant {
                    if let Some(thinking_content) = &last_message.thinking {
                        thinking_content.as_str()
                    } else {
                        "No thoughts available"
                    }
                } else {
                    node.output.as_deref().unwrap_or("Response")
                }
            } else {
                node.output.as_deref().unwrap_or("Response")
            }
        } else {
            node.output.as_deref().unwrap_or("Response")
        };
        
        rsx! {
            NodeSocket{
                node_id: node.id,
                canvas_zoom,
                on_start_connection,
                on_connection_redirect,
                input: true,
                output: true
            }
            
            div {
                style: "display: flex; flex-direction: column; gap: 12px; height: 100%;",
                
                div {
                    style: "display: flex; flex-direction: column; gap: 8px;",
                    
                    div {
                        style: "position: relative;",   
                        select {
                            value: "{model_name}",
                            onchange: move |event| {
                                on_model_change((node.id, event.value()));
                            },
                            onmousedown: |evt| evt.stop_propagation(),
                            style: "background: var(--bg-alt); appearance: none; width: 100%; border: none; 
                                color: var(--text-primary); padding: 6px 12px; cursor: pointer;",
                            
                            for model in available_models() {
                                option { value: "{model}", "{model}" }
                            }
                        }
                        div {
                            style: "position: absolute; right: 8px; top: 50%; transform: translateY(-50%); pointer-events: none; color: var(--text-primary); font-size: 12px;",
                            "▼"
                        }
                    }
                    
                    div {
                        style: "display: flex; align-items: center; gap: 8px;",
                        input {
                            r#type: "checkbox",
                            id: "thinking-{node.id}",
                            checked: if let NodeType::Model { thinking, .. } = &node.node_type { *thinking } else { false },
                            onchange: move |event| {
                                let mut ws = workflow_state.write();
                                if let Some(node_to_update) = ws.nodes.get_mut(&node.id) {
                                    if let NodeType::Model { thinking, .. } = &mut node_to_update.node_type {
                                        *thinking = event.checked();
                                    }
                                }
                            },
                            onmousedown: |evt| evt.stop_propagation(),
                            style: "cursor: pointer;",
                        },
                        label {
                            r#for: "thinking-{node.id}",
                            style: "font-size: 12px; cursor: pointer; user-select: none; color: var(--text-primary)",
                            "Thinking"
                        }
                        if let NodeType::Model { thinking, .. } = &node.node_type {
                            if *thinking {
                                button {
                                    style: "padding: 2px 6px; background-color: var(--bg-alt); color: var(--text-primary); 
                                        border: 1px solid var(--text-primary); border-radius: 4px; cursor: pointer; font-size: 10px; margin-left: auto;",
                                    onclick: move |_| {
                                        show_thoughts.set(!show_thoughts());
                                    },
                                    onmousedown: |evt| evt.stop_propagation(),
                                    if show_thoughts() { "Hide Thoughts" } else { "Show Thoughts" }
                                }
                            }
                        }
                    }
                }
                
                div { 
                    style: "flex-grow: 1; display: flex; flex-direction: column; overflow: hidden;", 
                    div {  
                        class: "text-box",
                        onwheel: move |event| event.stop_propagation(),
                        "{output_text}"
                    }
                }
            }
        }    
    }
}


pub async fn execute_model_node(
    mut workflow_state: Signal<Workflow>,
    node_id: usize,
    messages: Vec<ChatMessage>,
    provider_type: ProviderType,
    model_name: Option<String>,
    thinking_enabled: Option<bool>
) {
    let result = match provider_type {
        ProviderType::Ollama => {
            let client = OllamaClient::new();
            client.generate(model_name, messages, thinking_enabled).await
        },
        ProviderType::Anthropic => {
            let client = AnthropicClient::new();
            client.generate(model_name, messages, thinking_enabled).await
        }
    };
    
    let mut receiver = match result {
        Ok(recv) => recv,
        Err(e) => {
            println!("Failed to execute node {}: {}", node_id, e);
            if let Some(n) = workflow_state.write().nodes.get_mut(&node_id) {
                n.is_executing = false;
            }
            return;
        }
    };
    
    let mut ws_write = workflow_state.write();
    if let Some(NodeType::Model { messages, .. }) = ws_write.nodes.get_mut(&node_id).map(|n| &mut n.node_type) {
        messages.push(ChatMessage {
            role: MessageRole::Assistant,
            content: String::new(),
            thinking: None,
        });
    }
    drop(ws_write);
    
    let mut full_response = String::new();
    while let Some(message_chunk) = receiver.recv().await {
        let mut ws_write = workflow_state.write();
        if let Some(NodeType::Model { messages, .. }) = ws_write.nodes.get_mut(&node_id).map(|n| &mut n.node_type) {
            if let Some(last_msg) = messages.last_mut() {
                if let Some(thinking_chunk) = &message_chunk.thinking {
                    match &mut last_msg.thinking {
                        Some(existing) => existing.push_str(thinking_chunk),
                        None => last_msg.thinking = Some(thinking_chunk.clone()),
                    }
                }
                
                if !message_chunk.content.is_empty() {
                    last_msg.content.push_str(&message_chunk.content);
                    full_response.push_str(&message_chunk.content);
                }
            }
        }
        ws_write.update_node_output(node_id, full_response.clone());
    }
    
    if let Some(n) = workflow_state.write().nodes.get_mut(&node_id) {
        n.is_executing = false;
    }
}