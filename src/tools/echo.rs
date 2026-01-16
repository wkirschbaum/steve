use rmcp::model::{CallToolResult, Content};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct EchoRequest {
    #[schemars(description = "The message to echo back")]
    pub message: String,
}

pub fn handle_echo(req: EchoRequest) -> CallToolResult {
    CallToolResult::success(vec![Content::text(req.message)])
}
