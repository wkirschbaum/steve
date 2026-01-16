use rmcp::model::{CallToolResult, Content};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct LsRequest {
    #[schemars(description = "Directory path to list (defaults to current directory)")]
    pub path: Option<String>,
}

pub fn handle_pwd() -> CallToolResult {
    match std::env::current_dir() {
        Ok(path) => CallToolResult::success(vec![Content::text(path.display().to_string())]),
        Err(e) => CallToolResult::success(vec![Content::text(format!(
            "Failed to get cwd: {}",
            e
        ))]),
    }
}

pub fn handle_ls(req: LsRequest) -> CallToolResult {
    let dir = req.path.unwrap_or_else(|| ".".to_string());
    match std::fs::read_dir(&dir) {
        Ok(entries) => {
            let files: Vec<String> = entries
                .filter_map(|e| e.ok())
                .map(|e| e.file_name().to_string_lossy().to_string())
                .collect();
            CallToolResult::success(vec![Content::text(files.join("\n"))])
        }
        Err(e) => CallToolResult::success(vec![Content::text(format!(
            "Failed to list {}: {}",
            dir, e
        ))]),
    }
}
