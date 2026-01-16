mod tools;

use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{CallToolResult, Implementation, ProtocolVersion, ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router, ErrorData as McpError, ServerHandler, ServiceExt,
};
use tokio::io::{stdin, stdout};
use tools::{
    EchoRequest, ElixirProjectsRequest, LsRequest, SpotifyRequest,
    handle_echo, handle_elixir_projects, handle_ls, handle_pwd, handle_spotify,
};

#[derive(Clone)]
pub struct Steve {
    tool_router: ToolRouter<Self>,
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
        Ok(handle_echo(req))
    }

    #[tool(description = "Get the current working directory")]
    async fn pwd(&self) -> Result<CallToolResult, McpError> {
        Ok(handle_pwd())
    }

    #[tool(description = "List files in the specified directory")]
    async fn ls(
        &self,
        Parameters(req): Parameters<LsRequest>,
    ) -> Result<CallToolResult, McpError> {
        Ok(handle_ls(req))
    }

    #[tool(description = "Control Spotify playing in Firefox via MPRIS. Actions: play, pause, play_pause, next, previous, status")]
    async fn spotify(
        &self,
        Parameters(req): Parameters<SpotifyRequest>,
    ) -> Result<CallToolResult, McpError> {
        Ok(handle_spotify(req).await)
    }

    #[tool(description = "Manage Elixir projects. Actions: list, update_deps, outdated, git_pull, git_push, git_status, refresh. Uses cached project list from ~/.cache/steve/projects. Use 'project' to filter by name.")]
    async fn elixir_projects(
        &self,
        Parameters(req): Parameters<ElixirProjectsRequest>,
    ) -> Result<CallToolResult, McpError> {
        Ok(handle_elixir_projects(req).await)
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
