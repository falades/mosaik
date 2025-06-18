use anyhow::Result;
use std::path::PathBuf;
use std::fs;
use crate::components::Workflow;

const WORKFLOW_FILENAME: &str = "default.json";

/// Get the application directory, creating it if it doesn't exist
fn get_app_directory() -> Result<PathBuf> {
    let app_dir = if let Some(data_dir) = dirs::data_dir() {
        data_dir.join("mosaik")
    } else {
        // Fallback to current directory if we can't get system data dir
        std::env::current_dir()?.join(".mosaik")
    };
    
    // Create the directory if it doesn't exist
    if !app_dir.exists() {
        fs::create_dir_all(&app_dir)?;
    }
    
    Ok(app_dir)
}

/// Get the workflows directory, creating it if it doesn't exist
fn get_workflows_directory() -> Result<PathBuf> {
    let workflows_dir = get_app_directory()?.join("workflows");
    
    // Create the directory if it doesn't exist
    if !workflows_dir.exists() {
        fs::create_dir_all(&workflows_dir)?;
    }
    
    Ok(workflows_dir)
}

/// Save the workflow to the default file
pub fn save_default_workflow(workflow: &Workflow) -> Result<()> {
    let workflow_dir = get_workflows_directory()?;
    let file_path = workflow_dir.join(WORKFLOW_FILENAME);
    
    let json_content = serde_json::to_string_pretty(workflow)?;
    fs::write(file_path, json_content)?;
    
    Ok(())
}

/// Load the workflow from the default file
pub fn load_default_workflow() -> Result<Workflow> {
    let workflow_dir = get_workflows_directory()?;
    let file_path = workflow_dir.join(WORKFLOW_FILENAME);
    
    let json_content = fs::read_to_string(file_path)?;
    let workflow: Workflow = serde_json::from_str(&json_content)?;
    
    Ok(workflow)
}