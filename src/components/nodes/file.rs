use dioxus::prelude::*;
use crate::components::{
    nodes::{Node, NodeType, NodeSocket},
    Workflow
};

#[component]
pub fn FileImportNode(
    node: Node,
    workflow_state: Signal<Workflow>,
    canvas_zoom: f64,
    on_start_connection: EventHandler<(usize, Event<MouseData>)>,
    on_connection_redirect: EventHandler<(usize, Event<MouseData>)>,
) -> Element {
    let file_name = use_memo(move || {
        let ws = workflow_state.read();
        if let Some(node) = ws.nodes.get(&node.id) {
            if let NodeType::FileImport { file_name, .. } = &node.node_type {
                return file_name.clone();
            }
        }
        None
    });
    let on_file_select = move |_| {
        let mut ws_clone = workflow_state;
        let node_id = node.id;
        
        spawn(async move {
            if let Some(file_path) = rfd::AsyncFileDialog::new()
                .add_filter("Text Files", &["txt", "md"])
                .pick_file()
                .await
            {
                let path_str = file_path.path().to_string_lossy().to_string();
                let file_name = file_path.file_name();
                
                match std::fs::read_to_string(&path_str) {
                    Ok(content) => {
                        let mut ws = ws_clone.write();
                        if let Some(node) = ws.nodes.get_mut(&node_id) {
                            if let NodeType::FileImport { file_path: fp, file_name: fn_field } = &mut node.node_type {
                                *fp = Some(path_str);
                                *fn_field = Some(file_name);
                                node.output = Some(content);
                                node.needs_execution = false;
                            }
                        }
                    },
                    Err(_) => {
                        // Handle error - could show a message
                    }
                }
            }
        });
    };

    rsx! {
        NodeSocket{
            node_id: node.id,
            canvas_zoom,
            on_start_connection,
            on_connection_redirect,
            input: false,
            output: true,
        }
        
        div {
            style: "display: flex; flex-direction: column; gap: 6px; height: 100%;",
            if let Some(name) = file_name() {
                div {
                    style: "font-size: 12px; color: var(--text-primary); word-break: break-all; 
                            font-weight: bold; padding: 4px 0;",
                    "{name}"
                }
            } else {
                button {
                    style: "background-color: var(--bg-alt); color: var(--text-primary); border: none; 
                            padding: 8px 12px; border-radius: 4px; cursor: pointer; font-weight: bold;",
                    onclick: on_file_select,
                    onmousedown: |evt| evt.stop_propagation(),
                    "Select File"
                }
            }
            
            if let Some(content) = &node.output {
                div { 
                    style: "flex-grow: 1; display: flex; flex-direction: column; overflow: hidden;", 
                    div {  
                        class: "text-box",
                        onwheel: move |event| event.stop_propagation(),
                        "{content}"
                    }
                }
            }
        }
    }
}

#[component]
pub fn FileExportNode(
    node: Node,
    workflow_state: Signal<Workflow>,
    canvas_zoom: f64,
    on_connection_redirect: EventHandler<(usize, Event<MouseData>)>,
) -> Element {
    let mut error_message = use_signal(|| None::<String>);
    let (folder_path, file_name, file_type) = use_memo(move || {
        let ws = workflow_state.read();
        if let Some(node) = ws.nodes.get(&node.id) {
            if let NodeType::FileExport { folder_path, file_name, file_type } = &node.node_type {
                return (folder_path.clone(), file_name.clone(), file_type.clone());
            }
        }
        (None, None, "txt".to_string())
    })();

    let mut update_folder = move |new_folder: String| {
        if let Some(node) = workflow_state.write().nodes.get_mut(&node.id) {
            if let NodeType::FileExport { folder_path, .. } = &mut node.node_type {
                *folder_path = if new_folder.is_empty() { None } else { Some(new_folder) };
                node.needs_execution = true;
            }
        }
    };

    let mut update_filename = move |new_filename: String| {
        if let Some(node) = workflow_state.write().nodes.get_mut(&node.id) {
            if let NodeType::FileExport { file_name, .. } = &mut node.node_type {
                *file_name = if new_filename.is_empty() { None } else { Some(new_filename) };
                node.needs_execution = true;
            }
        }
    };

    let mut update_file_type = move |new_type: String| {
        if let Some(node) = workflow_state.write().nodes.get_mut(&node.id) {
            if let NodeType::FileExport { file_type, .. } = &mut node.node_type {
                *file_type = new_type;
                node.needs_execution = true;
            }
        }
    };
    
    let save_file = move |_| {
        error_message.set(None);
        let ws = workflow_state.read();
        let Some(node) = ws.nodes.get(&node.id) else { return };
        let NodeType::FileExport { folder_path, file_name, file_type } = &node.node_type else { return };
        let (Some(input), Some(folder), Some(filename)) = (&node.input, folder_path, file_name) else {
            error_message.set(Some("Forgot to add folder or filename?".to_string()));
            return
        };
        
        if filename.is_empty() { 
            error_message.set(Some("Forgot to add filename?".to_string()));
            return
        };
        let full_filename = format!("{}.{}", filename, file_type);
        let file_path = std::path::Path::new(folder).join(&full_filename);
        let input_content = input.clone();
        
        spawn(async move {
            match std::fs::write(&file_path, input_content) {
                Ok(_) => println!("File saved successfully to: {:?}", file_path),
                Err(e) => {
                    error_message.set(Some(format!("Failed to save file: {}", e)));
                }
            }
        });
    };
    
    let choose_folder = move |_| {
        spawn(async move {
            if let Some(folder) = rfd::AsyncFileDialog::new().pick_folder().await {
                if let Some(path) = folder.path().to_str() {
                    update_folder(path.to_string());
                }
            }
        });
    };
    
    rsx! {
        NodeSocket {
            node_id: node.id,
            canvas_zoom,
            on_start_connection: |_| {},
            on_connection_redirect,
            input: true,
            output: false
        }
        
        div {
            style: "display: flex; flex-direction: column; overflow-y: auto; gap: 6px; height: 100%;",
            onwheel: move |event| event.stop_propagation(),
            
            if let Some(path) = &folder_path {
                div {
                    style: "font-size: 12px; color: var(--text-primary); word-break: break-all; 
                            font-weight: bold; padding: 4px 0;",
                    "{path}"
                }
            } else {
                button {
                    style: "background-color: var(--bg-alt); color: var(--text-primary); border: none; 
                            padding: 8px 12px; border-radius: 4px; cursor: pointer; font-weight: bold;",
                    onclick: choose_folder,
                    onmousedown: |evt| evt.stop_propagation(),
                    "Select Folder"
                }
            }
    
            div {
                style: "display: flex; justify-content: space-between; width: 100%;",
                input {
                    r#type: "text",
                    value: file_name.unwrap_or_default(),
                    placeholder: "filename...",
                    oninput: move |event| update_filename(event.value()),
                    onmousedown: move |event| event.stop_propagation(),
                    style: "padding: 8px; border: none; border-radius: 4px; background-color: var(--bg-alt); color: var(--text-primary); width: 60%; box-sizing: border-box;"
                }
            
                div {
                    style: "position: relative; width: 35%;",   
                    select {
                        value: file_type,
                        onchange: move |event| update_file_type(event.value()),
                        onmousedown: move |event| event.stop_propagation(),
                        style: "background: var(--bg-alt); appearance: none; width: 100%; border: none; 
                                color: var(--text-primary); padding: 6px 12px; cursor: pointer; box-sizing: border-box;",
                        
                        option { value: "txt", ".txt" }
                        option { value: "md", ".md" }
                    }
                    div {
                        style: "position: absolute; right: 8px; top: 50%; transform: translateY(-50%); pointer-events: none; color: var(--text-primary); font-size: 12px;",
                        "▼"
                    }
                }
            }
    
            if let Some(input) = &node.input {
                if let Some(error) = error_message() {
                    div {
                        style: "color: var(--text-warning); font-size: 12px; padding: 4px;",
                        "{error}"
                    }
                }
                
                div {
                    style: "flex-grow: 1; display: flex; flex-direction: column;",
                    button {
                        style: "background-color: var(--bg-alt); color: var(--text-primary); border: none; 
                                padding: 8px 12px; border-radius: 4px; cursor: pointer; font-weight: bold; margin-bottom: 6px;",
                        onclick: save_file,
                        onmousedown: |evt| evt.stop_propagation(),
                        "Save"
                    }
                    div {
                        class: "text-box",
                        onwheel: move |event| event.stop_propagation(),
                        "{input}"
                    }
                }
            }
        }
    }
}