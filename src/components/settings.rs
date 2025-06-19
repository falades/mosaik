use dioxus::prelude::*;
use crate::llm::ApiManager;
#[component]
pub fn SettingsPopup(
    popup_open: Signal<bool>,
) -> Element {
    let mut selected_section = use_signal(|| "api");
    
    rsx! {
        // Backdrop
        div {
            style: "position: fixed; top: 0; left: 0; width: 100%; height: 100%; 
                   background: rgba(0,0,0,0.3); z-index: 200; display: flex; 
                   justify-content: center; align-items: center;",
            onclick: move |_| popup_open.set(false),
            
            // Popup
            div {
                style: "background: var(--bg-alt); border: 1px solid var(--ui); 
                       border-radius: 8px; width: 60%; height: 80%; padding: 20px; display: flex;",
                onclick: move |e| e.stop_propagation(),
                
                // Sidebar
                div {
                    style: "width: 20%; border-right: 1px solid var(--ui); 
                           padding: 20px; display: flex; flex-direction: column; gap: 8px;",
                    
                    button {
                        style: format!("padding: 12px 16px; text-align: left; border: none; 
                                       border-radius: 6px; cursor: pointer; background: {}; 
                                       color: var(--text-primary);",
                                     if *selected_section.read() == "api" { 
                                         "var(--ui)" 
                                     } else { 
                                         "transparent" 
                                     }),
                        onclick: move |_| selected_section.set("api"),
                        "API Keys"
                    }
                }
                
                // Content area
                div {
                    style: "flex: 1; display: flex; flex-direction: column;",
                    
                    if *selected_section.read() == "api" {
                        ApiKeysSection {}
                    } 
                }
            }
        }
    }
}

#[component]
fn ApiKeysSection() -> Element {
    let mut anthropic_key = use_signal (String::new);
    // let mut openai_key = use_signal(String::new);
    // let mut google_key = use_signal(String::new);
    let mut save_status = use_signal(String::new);
    
    // Check if keys exist without loading their values
    let mut keys_exist = use_signal(|| (false, false, false));
    
    use_effect(move || {
        if let Ok(key_manager) = ApiManager::new() {
            let openai_exists = key_manager.get_openai_key().is_ok();
            let anthropic_exists = key_manager.get_anthropic_key().is_ok();
            let google_exists = key_manager.get_google_key().is_ok();
            keys_exist.set((openai_exists, anthropic_exists, google_exists));
        }
    });
    
    let save_keys = move |_| {
        let key_manager = match ApiManager::new() {
            Ok(manager) => manager,
            Err(e) => {
                save_status.set(format!("Failed to initialize key manager: {}", e));
                return;
            }
        };
    
        let mut errors = Vec::new();
        if !anthropic_key().is_empty() {
            if let Err(e) = key_manager.save_anthropic_key(&anthropic_key()) {
                errors.push(format!("Anthropic: {}", e));
            }
        }
        // if !openai_key().is_empty() {
        //     if let Err(e) = key_manager.save_openai_key(&openai_key()) {
        //         errors.push(format!("OpenAI: {}", e));
        //     }
        // }
        // if !google_key().is_empty() {
        //     if let Err(e) = key_manager.save_google_key(&google_key()) {
        //         errors.push(format!("Google: {}", e));
        //     }
        // }
        if errors.is_empty() {
            save_status.set("API keys saved successfully!".to_string());
        } else {
            save_status.set(format!("Errors: {}", errors.join(", ")));
        }
    };
    
    rsx! {
        div {
            style: "flex: 1; padding: 20px; display: flex; flex-direction: column; gap: 20px;",
            
        //     div {
        //         style: "display: flex; flex-direction: column; gap: 12px;",
                
        //         h4 {
        //             style: "margin: 0; color: var(--text-primary);",
        //             "OpenAI API Key:"
        //         }
                
        //         if keys_exist().0 {
        //             div {
        //                 style: "display: flex; align-items: center; gap: 12px;",
        //                 span {
        //                     style: "color: var(--text-secondary);",
        //                     "API key is configured"
        //                 }
        //                 button {
        //                     style: "padding: 6px 12px; background: transparent; color: var(--text-error);
        //                            border: 1px solid var(--text-error); border-radius: 4px; cursor: pointer;",
        //                     onclick: move |_| {
        //                         let (_, anthropic, google) = keys_exist();
        //                         keys_exist.set((false, anthropic, google));
        //                     },
        //                     "Replace"
        //                 }
        //             }
        //         } else {
        //             input {
        //                 r#type: "password",
        //                 placeholder: "Enter your OpenAI API key",
        //                 style: "padding: 12px; border: 1px solid var(--ui); border-radius: 6px; 
        //                        background: var(--bg-primary); color: var(--text-primary); font-size: 14px;",
        //                 value: "{openai_key}",
        //                 oninput: move |e| openai_key.set(e.value())
        //             }
        //         }
        //     }
            
            div {
                style: "display: flex; flex-direction: column; gap: 12px;",
                
                h4 {
                    style: "margin: 0; color: var(--text-primary);",
                    "Anthropic API Key:"
                }
                
                if keys_exist().1 {
                    div {
                        style: "display: flex; align-items: center; gap: 12px;",
                        span {
                            style: "color: var(--text-secondary);",
                            "API key is configured"
                        }
                        button {
                            style: "padding: 6px 12px; background: transparent; color: var(--text-error);
                                   border: 1px solid var(--text-error); border-radius: 4px; cursor: pointer;",
                            onclick: move |_| {
                                let (openai, _, google) = keys_exist();
                                keys_exist.set((openai, false, google));
                            },
                            "Replace"
                        }
                    }
                } else {
                    input {
                        r#type: "password",
                        placeholder: "Enter your Anthropic API key",
                        style: "padding: 12px; border: 1px solid var(--ui); border-radius: 6px; 
                               background: var(--bg-primary); color: var(--text-primary); font-size: 14px;",
                        value: "{anthropic_key}",
                        oninput: move |e| anthropic_key.set(e.value())
                    }
                }
            }
            
            // div {
            //     style: "display: flex; flex-direction: column; gap: 12px;",
                
            //     h4 {
            //         style: "margin: 0; color: var(--text-primary);",
            //         "Google API Key:"
            //     }
                
            //     if keys_exist().2 {
            //         div {
            //             style: "display: flex; align-items: center; gap: 12px;",
            //             span {
            //                 style: "color: var(--text-secondary);",
            //                 "API key is configured"
            //             }
            //             button {
            //                 style: "padding: 6px 12px; background: transparent; color: var(--text-error);
            //                        border: 1px solid var(--text-error); border-radius: 4px; cursor: pointer;",
            //                 onclick: move |_| {
            //                     let (openai, anthropic, _) = keys_exist();
            //                     keys_exist.set((openai, anthropic, false));
            //                 },
            //                 "Replace"
            //             }
            //         }
            //     } else {
            //         input {
            //             r#type: "password",
            //             placeholder: "Enter your Google API key",
            //             style: "padding: 12px; border: 1px solid var(--ui); border-radius: 6px; 
            //                    background: var(--bg-primary); color: var(--text-primary); font-size: 14px;",
            //             value: "{google_key}",
            //             oninput: move |e| google_key.set(e.value())
            //         }
            //     }
            // }
            
            button {
                style: "padding: 12px 24px; background: var(--ui); color: var(--text-primary);
                       border: none; border-radius: 6px; cursor: pointer; font-weight: 500; 
                       align-self: flex-start;",
                onclick: save_keys,
                "Save API Keys"
            }
            
            if !save_status().is_empty() {
                div {
                    style: "padding: 12px; border-radius: 6px; background: var(--ui); color: var(--text-primary); font-size: 14px;",
                    "{save_status}"
                }
            }
        }
    }
}