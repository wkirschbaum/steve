use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{CallToolResult, Content, Implementation, ProtocolVersion, ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router, ErrorData as McpError, ServerHandler, ServiceExt,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tokio::io::{stdin, stdout};
use tokio::process::Command;

#[derive(Clone)]
pub struct Steve {
    tool_router: ToolRouter<Self>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct EchoRequest {
    #[schemars(description = "The message to echo back")]
    message: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct LsRequest {
    #[schemars(description = "Directory path to list (defaults to current directory)")]
    path: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct SpotifyRequest {
    #[schemars(description = "Action to perform: play, pause, play_pause, next, previous, or status")]
    action: String,
}

#[tool_router]
impl Steve {
    pub fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
        }
    }

    #[tool(description = "Echo back the provided message")]
    async fn echo(
        &self,
        Parameters(req): Parameters<EchoRequest>,
    ) -> Result<CallToolResult, McpError> {
        Ok(CallToolResult::success(vec![Content::text(req.message)]))
    }

    #[tool(description = "Get the current working directory")]
    async fn pwd(&self) -> Result<CallToolResult, McpError> {
        match std::env::current_dir() {
            Ok(path) => Ok(CallToolResult::success(vec![Content::text(
                path.display().to_string(),
            )])),
            Err(e) => Ok(CallToolResult::success(vec![Content::text(format!(
                "Failed to get cwd: {}",
                e
            ))])),
        }
    }

    #[tool(description = "List files in the specified directory")]
    async fn ls(
        &self,
        Parameters(req): Parameters<LsRequest>,
    ) -> Result<CallToolResult, McpError> {
        let dir = req.path.unwrap_or_else(|| ".".to_string());
        match std::fs::read_dir(&dir) {
            Ok(entries) => {
                let files: Vec<String> = entries
                    .filter_map(|e| e.ok())
                    .map(|e| e.file_name().to_string_lossy().to_string())
                    .collect();
                Ok(CallToolResult::success(vec![Content::text(files.join("\n"))]))
            }
            Err(e) => Ok(CallToolResult::success(vec![Content::text(format!(
                "Failed to list {}: {}",
                dir, e
            ))])),
        }
    }

    #[tool(description = "Control Spotify playing in Firefox via MPRIS. Actions: play, pause, play_pause, next, previous, status")]
    async fn spotify(
        &self,
        Parameters(req): Parameters<SpotifyRequest>,
    ) -> Result<CallToolResult, McpError> {
        let player = "firefox";

        let result = match req.action.as_str() {
            "play" => run_playerctl(player, &["play"]).await,
            "pause" => run_playerctl(player, &["pause"]).await,
            "play_pause" => run_playerctl(player, &["play-pause"]).await,
            "next" => run_playerctl(player, &["next"]).await,
            "previous" => run_playerctl(player, &["previous"]).await,
            "status" => {
                let status = run_playerctl(player, &["status"]).await.unwrap_or_default();
                let metadata = run_playerctl(
                    player,
                    &["metadata", "--format", "{{ artist }} - {{ title }}"],
                )
                .await
                .unwrap_or_default();
                Ok(format!("{}\n{}", status.trim(), metadata.trim()))
            }
            _ => Err(format!(
                "Unknown action '{}'. Use: play, pause, play_pause, next, previous, status",
                req.action
            )),
        };

        match result {
            Ok(output) => Ok(CallToolResult::success(vec![Content::text(output)])),
            Err(e) => Ok(CallToolResult::success(vec![Content::text(e)])),
        }
    }
}

async fn run_playerctl(player: &str, args: &[&str]) -> Result<String, String> {
    let mut cmd_args = vec!["--player", player];
    cmd_args.extend(args);

    match Command::new("playerctl").args(&cmd_args).output().await {
        Ok(output) => {
            if output.status.success() {
                Ok(String::from_utf8_lossy(&output.stdout).to_string())
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                Err(format!("playerctl error: {}", stderr.trim()))
            }
        }
        Err(e) => Err(format!("Failed to run playerctl: {}", e)),
    }
}

#[tool_handler]
impl ServerHandler for Steve {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: Implementation::from_build_env(),
            instructions: Some("Steve - a local MCP server for system tasks".to_string()),
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let service = Steve::new();
    let transport = (stdin(), stdout());
    let server = service.serve(transport).await?;
    server.waiting().await?;
    Ok(())
}
