# Steve MCP Server

A local MCP server for system tasks, built with Rust.

## Installation

```bash
cargo build --release
```

The binary is at `target/release/steve`. Configure it in your Claude Code MCP settings.

## Tools

### spotify

Control Spotify playing in Firefox via MPRIS.

| Action | Description |
|--------|-------------|
| `play` | Start playback |
| `pause` | Pause playback |
| `play_pause` | Toggle play/pause |
| `next` | Skip to next track |
| `previous` | Previous track / restart |
| `status` | Show current track |

### elixir_projects

Manage Elixir projects in `~/src/flt`. Uses a cached project list stored in `~/.cache/steve/projects`.

**Parameters:**
- `action` (required): The action to perform
- `project` (optional): Filter to specific project(s) by name
- `path` (optional): Override the default search path

**Actions:**

| Action | Description |
|--------|-------------|
| `list` | List all projects (from cache) |
| `refresh` | Rescan and rebuild the project cache |
| `update_deps` | Run `mix deps.update --all` on projects |
| `outdated` | Check for outdated hex packages |
| `git_pull` | Pull latest changes from remote |
| `git_push` | Push commits to remote |
| `git_status` | Show uncommitted changes and unpushed commits |
| `delete` | Remove project directory (requires `project` filter) |
| `ignore` | Add project to ignore list, or list ignored projects |
| `unignore` | Remove project from ignore list |

**Examples:**

```
# List all projects
elixir_projects(action: "list")

# Update deps for a specific project
elixir_projects(action: "update_deps", project: "moneyclub")

# Check what needs pushing before going home
elixir_projects(action: "git_status")

# Ignore a project
elixir_projects(action: "ignore", project: "old_project")

# See ignored projects
elixir_projects(action: "ignore")

# Refresh the project cache
elixir_projects(action: "refresh")
```

**Cache files:**
- `~/.cache/steve/projects` - Cached list of project paths
- `~/.cache/steve/ignored` - List of ignored project names

### Other tools

- `echo` - Echo back a message
- `pwd` - Get current working directory
- `ls` - List files in a directory
