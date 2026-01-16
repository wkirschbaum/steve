# Steve MCP Server

A local MCP server for system tasks, built with Rust using the `rmcp` crate.

## Build

After making changes, always build to verify:

```bash
cargo build --release
```

The binary is located at `target/release/steve`.

## Adding Tools

Tools are defined in `src/main.rs` using the `#[tool]` macro. Each tool needs:
1. A request struct with `#[derive(Debug, Serialize, Deserialize, JsonSchema)]`
2. An async method with `#[tool(description = "...")]`

After adding or modifying tools, rebuild and restart Claude Code to pick up changes.
