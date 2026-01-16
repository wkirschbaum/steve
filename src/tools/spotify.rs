use rmcp::model::{CallToolResult, Content};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tokio::process::Command;

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct SpotifyRequest {
    #[schemars(description = "Action to perform: play, pause, play_pause, next, previous, or status")]
    pub action: String,
}

pub async fn handle_spotify(req: SpotifyRequest) -> CallToolResult {
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
        Ok(output) => CallToolResult::success(vec![Content::text(output)]),
        Err(e) => CallToolResult::success(vec![Content::text(e)]),
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
