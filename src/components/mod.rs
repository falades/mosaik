mod canvas;
mod nodes;
mod connections;
mod workflow;
mod settings;

pub use {
    canvas::{Canvas, CanvasState},
    workflow::{WorkflowManager, Workflow},
    nodes::{ChatMessage, MessageRole, NodeType, model::execute_model_node},
    settings::SettingsPopup
};