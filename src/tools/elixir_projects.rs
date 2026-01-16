use rmcp::model::{CallToolResult, Content};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use tokio::process::Command;
use walkdir::WalkDir;

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ElixirProjectsRequest {
    #[schemars(
        description = "Action to perform: list, update_deps, outdated, git_pull, git_push, git_status, refresh, delete, ignore, unignore"
    )]
    pub action: String,
    #[schemars(description = "Filter to a specific project by name (e.g., 'moneyclub')")]
    pub project: Option<String>,
    #[schemars(description = "Starting directory path (defaults to ~/src/flt)")]
    pub path: Option<String>,
}

pub async fn handle_elixir_projects(req: ElixirProjectsRequest) -> CallToolResult {
    let is_refresh = req.action == "refresh";
    let mut projects = get_elixir_projects(req.path.as_deref(), is_refresh);

    // Filter by project name if specified
    if let Some(ref project_filter) = req.project {
        let filter_lower = project_filter.to_lowercase();
        projects.retain(|p| {
            p.file_name()
                .map(|n| n.to_string_lossy().to_lowercase().contains(&filter_lower))
                .unwrap_or(false)
        });
    }

    match req.action.as_str() {
        "refresh" => handle_refresh(projects),
        "list" => handle_list(projects),
        "update_deps" => handle_update_deps(projects).await,
        "outdated" => handle_outdated(projects).await,
        "git_pull" => handle_git_pull(projects).await,
        "git_push" => handle_git_push(projects).await,
        "git_status" => handle_git_status(projects).await,
        "delete" => handle_delete(projects, &req),
        "ignore" => handle_ignore(&req),
        "unignore" => handle_unignore(&req),
        _ => CallToolResult::success(vec![Content::text(format!(
            "Unknown action '{}'. Use: list, update_deps, outdated, git_pull, git_push, git_status, refresh, delete, ignore, unignore",
            req.action
        ))]),
    }
}

fn handle_refresh(projects: Vec<PathBuf>) -> CallToolResult {
    let output = format!(
        "Refreshed project cache. Found {} Elixir projects:\n{}",
        projects.len(),
        projects
            .iter()
            .map(|p| p.display().to_string())
            .collect::<Vec<_>>()
            .join("\n")
    );
    CallToolResult::success(vec![Content::text(output)])
}

fn handle_list(projects: Vec<PathBuf>) -> CallToolResult {
    if projects.is_empty() {
        CallToolResult::success(vec![Content::text("No Elixir projects found".to_string())])
    } else {
        let names: Vec<String> = projects
            .iter()
            .map(|p| {
                p.file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| p.display().to_string())
            })
            .collect();
        let output = format!("Found {} projects: {}", projects.len(), names.join(", "));
        CallToolResult::success(vec![Content::text(output)])
    }
}

async fn handle_update_deps(projects: Vec<PathBuf>) -> CallToolResult {
    if projects.is_empty() {
        return CallToolResult::success(vec![Content::text(
            "No Elixir projects found".to_string(),
        )]);
    }

    let mut results: Vec<String> = Vec::new();
    for project in &projects {
        let output = Command::new("mix")
            .args(["deps.update", "--all"])
            .current_dir(project)
            .output()
            .await;

        let status = match output {
            Ok(o) if o.status.success() => "✓".to_string(),
            Ok(o) => {
                let stderr = String::from_utf8_lossy(&o.stderr);
                format!("✗ {}", stderr.lines().next().unwrap_or("failed"))
            }
            Err(e) => format!("✗ {}", e),
        };
        results.push(format!("{} {}", status, project.display()));
    }

    CallToolResult::success(vec![Content::text(format!(
        "Updated {} projects:\n{}",
        projects.len(),
        results.join("\n")
    ))])
}

async fn handle_outdated(projects: Vec<PathBuf>) -> CallToolResult {
    if projects.is_empty() {
        return CallToolResult::success(vec![Content::text(
            "No Elixir projects found".to_string(),
        )]);
    }

    let mut results: Vec<String> = Vec::new();
    let mut projects_with_outdated = 0;

    for project in &projects {
        let output = Command::new("mix")
            .args(["hex.outdated"])
            .current_dir(project)
            .output()
            .await;

        let project_name = project
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| project.display().to_string());

        match output {
            Ok(o) => {
                let stdout = String::from_utf8_lossy(&o.stdout);
                // Check if there are outdated deps (exit code 1 means outdated)
                if !o.status.success() || stdout.contains("Newer versions") {
                    projects_with_outdated += 1;
                    // Extract just the dependency lines
                    let outdated_deps: Vec<&str> = stdout
                        .lines()
                        .filter(|line| {
                            line.contains("->")
                                || (line.starts_with("  ")
                                    && !line.trim().is_empty()
                                    && !line.contains("Dependency"))
                        })
                        .collect();

                    if !outdated_deps.is_empty() {
                        results.push(format!(
                            "\n📦 {} ({} outdated):\n  {}",
                            project_name,
                            outdated_deps.len(),
                            outdated_deps.join("\n  ")
                        ));
                    }
                }
            }
            Err(e) => {
                results.push(format!("\n✗ {} - error: {}", project_name, e));
            }
        }
    }

    let summary = if projects_with_outdated == 0 {
        format!("All {} projects are up to date!", projects.len())
    } else {
        format!(
            "{}/{} projects have outdated dependencies:{}",
            projects_with_outdated,
            projects.len(),
            results.join("")
        )
    };

    CallToolResult::success(vec![Content::text(summary)])
}

async fn handle_git_pull(projects: Vec<PathBuf>) -> CallToolResult {
    if projects.is_empty() {
        return CallToolResult::success(vec![Content::text(
            "No Elixir projects found".to_string(),
        )]);
    }

    let mut results: Vec<String> = Vec::new();
    for project in &projects {
        let output = Command::new("git")
            .args(["pull"])
            .current_dir(project)
            .output()
            .await;

        let project_name = project
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| project.display().to_string());

        let status = match output {
            Ok(o) if o.status.success() => {
                let stdout = String::from_utf8_lossy(&o.stdout);
                if stdout.contains("Already up to date") {
                    "✓ (up to date)".to_string()
                } else {
                    "✓ (updated)".to_string()
                }
            }
            Ok(o) => {
                let stderr = String::from_utf8_lossy(&o.stderr);
                format!("✗ {}", stderr.lines().next().unwrap_or("failed"))
            }
            Err(e) => format!("✗ {}", e),
        };
        results.push(format!("{} {}", project_name, status));
    }

    CallToolResult::success(vec![Content::text(format!(
        "Git pull on {} projects:\n{}",
        projects.len(),
        results.join("\n")
    ))])
}

async fn handle_git_push(projects: Vec<PathBuf>) -> CallToolResult {
    if projects.is_empty() {
        return CallToolResult::success(vec![Content::text(
            "No Elixir projects found".to_string(),
        )]);
    }

    let mut results: Vec<String> = Vec::new();
    for project in &projects {
        let output = Command::new("git")
            .args(["push"])
            .current_dir(project)
            .output()
            .await;

        let project_name = project
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| project.display().to_string());

        let status = match output {
            Ok(o) if o.status.success() => {
                let stderr = String::from_utf8_lossy(&o.stderr);
                if stderr.contains("Everything up-to-date") {
                    "✓ (up to date)".to_string()
                } else {
                    "✓ (pushed)".to_string()
                }
            }
            Ok(o) => {
                let stderr = String::from_utf8_lossy(&o.stderr);
                format!("✗ {}", stderr.lines().next().unwrap_or("failed"))
            }
            Err(e) => format!("✗ {}", e),
        };
        results.push(format!("{} {}", project_name, status));
    }

    CallToolResult::success(vec![Content::text(format!(
        "Git push on {} projects:\n{}",
        projects.len(),
        results.join("\n")
    ))])
}

async fn handle_git_status(projects: Vec<PathBuf>) -> CallToolResult {
    if projects.is_empty() {
        return CallToolResult::success(vec![Content::text(
            "No Elixir projects found".to_string(),
        )]);
    }

    let mut dirty_projects: Vec<String> = Vec::new();
    let mut ahead_projects: Vec<String> = Vec::new();
    let mut clean_count = 0;

    for project in &projects {
        let project_name = project
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| project.display().to_string());

        // Check for uncommitted changes
        let status_output = Command::new("git")
            .args(["status", "--porcelain"])
            .current_dir(project)
            .output()
            .await;

        let has_changes = match &status_output {
            Ok(o) => !o.stdout.is_empty(),
            Err(_) => false,
        };

        // Check if ahead of remote
        let ahead_output = Command::new("git")
            .args(["status", "--branch", "--porcelain=v2"])
            .current_dir(project)
            .output()
            .await;

        let is_ahead = match &ahead_output {
            Ok(o) => {
                let stdout = String::from_utf8_lossy(&o.stdout);
                stdout.contains("ahead")
            }
            Err(_) => false,
        };

        if has_changes {
            dirty_projects.push(project_name.clone());
        }
        if is_ahead {
            ahead_projects.push(project_name.clone());
        }
        if !has_changes && !is_ahead {
            clean_count += 1;
        }
    }

    let mut output = String::new();

    if !dirty_projects.is_empty() {
        output.push_str(&format!(
            "⚠️  Uncommitted changes ({}):\n  {}\n\n",
            dirty_projects.len(),
            dirty_projects.join("\n  ")
        ));
    }

    if !ahead_projects.is_empty() {
        output.push_str(&format!(
            "📤 Unpushed commits ({}):\n  {}\n\n",
            ahead_projects.len(),
            ahead_projects.join("\n  ")
        ));
    }

    if dirty_projects.is_empty() && ahead_projects.is_empty() {
        output = format!("✅ All {} projects are clean and pushed!", projects.len());
    } else {
        output.push_str(&format!("✓ {} projects clean", clean_count));
    }

    CallToolResult::success(vec![Content::text(output)])
}

fn handle_delete(projects: Vec<PathBuf>, req: &ElixirProjectsRequest) -> CallToolResult {
    if req.project.is_none() {
        return CallToolResult::success(vec![Content::text(
            "Error: 'project' filter is required for delete action".to_string(),
        )]);
    }

    if projects.is_empty() {
        return CallToolResult::success(vec![Content::text(
            "No matching projects found".to_string(),
        )]);
    }

    let mut results: Vec<String> = Vec::new();
    for project in &projects {
        let project_name = project
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| project.display().to_string());

        match fs::remove_dir_all(project) {
            Ok(_) => results.push(format!("✓ Deleted {}", project_name)),
            Err(e) => results.push(format!("✗ Failed to delete {}: {}", project_name, e)),
        }
    }

    // Refresh cache after deletion
    let _ = save_projects_to_cache(&scan_elixir_projects(None));

    CallToolResult::success(vec![Content::text(results.join("\n"))])
}

fn handle_ignore(req: &ElixirProjectsRequest) -> CallToolResult {
    if req.project.is_none() {
        // List currently ignored projects
        let ignored = load_ignored_projects();
        if ignored.is_empty() {
            return CallToolResult::success(vec![Content::text(
                "No projects are currently ignored".to_string(),
            )]);
        }
        let mut names: Vec<_> = ignored.into_iter().collect();
        names.sort();
        return CallToolResult::success(vec![Content::text(format!(
            "Ignored projects: {}",
            names.join(", ")
        ))]);
    }

    // Add projects to ignore list
    let mut ignored = load_ignored_projects();
    let mut added: Vec<String> = Vec::new();

    // Get unfiltered projects to find matches
    let all_projects = if let Some(cached) = load_projects_from_cache() {
        cached
    } else {
        scan_elixir_projects(req.path.as_deref())
    };

    let filter = req.project.as_ref().unwrap().to_lowercase();
    for project in &all_projects {
        if let Some(name) = project.file_name() {
            let name_str = name.to_string_lossy().to_string();
            if name_str.to_lowercase().contains(&filter) && !ignored.contains(&name_str) {
                ignored.insert(name_str.clone());
                added.push(name_str);
            }
        }
    }

    if added.is_empty() {
        return CallToolResult::success(vec![Content::text(
            "No matching projects found to ignore".to_string(),
        )]);
    }

    let _ = save_ignored_projects(&ignored);
    CallToolResult::success(vec![Content::text(format!("Ignored: {}", added.join(", ")))])
}

fn handle_unignore(req: &ElixirProjectsRequest) -> CallToolResult {
    if req.project.is_none() {
        return CallToolResult::success(vec![Content::text(
            "Error: 'project' filter is required for unignore action".to_string(),
        )]);
    }

    let mut ignored = load_ignored_projects();
    let filter = req.project.as_ref().unwrap().to_lowercase();
    let mut removed: Vec<String> = Vec::new();

    let to_remove: Vec<String> = ignored
        .iter()
        .filter(|name| name.to_lowercase().contains(&filter))
        .cloned()
        .collect();

    for name in to_remove {
        ignored.remove(&name);
        removed.push(name);
    }

    if removed.is_empty() {
        return CallToolResult::success(vec![Content::text(
            "No matching ignored projects found".to_string(),
        )]);
    }

    let _ = save_ignored_projects(&ignored);
    CallToolResult::success(vec![Content::text(format!(
        "Unignored: {}",
        removed.join(", ")
    ))])
}

// Helper functions

fn get_cache_path() -> PathBuf {
    dirs::home_dir()
        .map(|h| h.join(".cache/steve/projects"))
        .unwrap_or_else(|| PathBuf::from(".projects"))
}

fn get_ignore_path() -> PathBuf {
    dirs::home_dir()
        .map(|h| h.join(".cache/steve/ignored"))
        .unwrap_or_else(|| PathBuf::from(".ignored"))
}

fn load_ignored_projects() -> HashSet<String> {
    let ignore_path = get_ignore_path();
    if !ignore_path.exists() {
        return HashSet::new();
    }

    fs::File::open(&ignore_path)
        .ok()
        .map(|file| {
            BufReader::new(file)
                .lines()
                .map_while(Result::ok)
                .collect()
        })
        .unwrap_or_default()
}

fn save_ignored_projects(ignored: &HashSet<String>) -> Result<(), std::io::Error> {
    let ignore_path = get_ignore_path();
    if let Some(parent) = ignore_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut file = fs::File::create(&ignore_path)?;
    for name in ignored {
        writeln!(file, "{}", name)?;
    }
    Ok(())
}

fn load_projects_from_cache() -> Option<Vec<PathBuf>> {
    let cache_path = get_cache_path();
    if !cache_path.exists() {
        return None;
    }

    let file = fs::File::open(&cache_path).ok()?;
    let reader = BufReader::new(file);
    let mut projects: Vec<PathBuf> = Vec::new();
    let mut needs_update = false;

    for line in reader.lines().map_while(Result::ok) {
        let path = PathBuf::from(&line);
        if path.exists() && path.join("mix.exs").exists() {
            projects.push(path);
        } else {
            needs_update = true;
        }
    }

    // Update cache if we removed any stale entries
    if needs_update {
        let _ = save_projects_to_cache(&projects);
    }

    Some(projects)
}

fn save_projects_to_cache(projects: &[PathBuf]) -> Result<(), std::io::Error> {
    let cache_path = get_cache_path();
    if let Some(parent) = cache_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut file = fs::File::create(&cache_path)?;
    for project in projects {
        writeln!(file, "{}", project.display())?;
    }
    Ok(())
}

fn scan_elixir_projects(path: Option<&str>) -> Vec<PathBuf> {
    let default_path = dirs::home_dir()
        .map(|h| h.join("src/flt"))
        .unwrap_or_else(|| PathBuf::from("."));

    let start_path = path
        .map(|p| {
            if p.starts_with("~/") {
                dirs::home_dir()
                    .map(|h| h.join(&p[2..]))
                    .unwrap_or_else(|| PathBuf::from(p))
            } else {
                PathBuf::from(p)
            }
        })
        .unwrap_or(default_path);

    if !start_path.exists() {
        return Vec::new();
    }

    // Directories to skip (dependencies, build artifacts, etc.)
    let skip_dirs: HashSet<&str> = [
        "deps",
        "_build",
        ".elixir_ls",
        "node_modules",
        ".git",
        "_checkouts",
    ]
    .into_iter()
    .collect();

    let mut projects: Vec<PathBuf> = Vec::new();

    for entry in WalkDir::new(&start_path)
        .follow_links(true)
        .into_iter()
        .filter_entry(|e| {
            // Skip certain directories
            if e.file_type().is_dir() {
                if let Some(name) = e.file_name().to_str() {
                    return !skip_dirs.contains(name);
                }
            }
            true
        })
        .filter_map(|e| e.ok())
    {
        if entry.file_type().is_file() && entry.file_name() == "mix.exs" {
            if let Some(parent) = entry.path().parent() {
                projects.push(parent.to_path_buf());
            }
        }
    }

    projects.sort();
    projects
}

fn get_elixir_projects(path: Option<&str>, force_refresh: bool) -> Vec<PathBuf> {
    let ignored = load_ignored_projects();

    let filter_ignored = |projects: Vec<PathBuf>| -> Vec<PathBuf> {
        projects
            .into_iter()
            .filter(|p| {
                p.file_name()
                    .map(|n| !ignored.contains(&n.to_string_lossy().to_string()))
                    .unwrap_or(true)
            })
            .collect()
    };

    // If custom path specified, always scan (don't use cache)
    if path.is_some() {
        return filter_ignored(scan_elixir_projects(path));
    }

    // Try to load from cache unless force refresh
    if !force_refresh {
        if let Some(projects) = load_projects_from_cache() {
            return filter_ignored(projects);
        }
    }

    // Scan and cache
    let projects = scan_elixir_projects(None);
    let _ = save_projects_to_cache(&projects);
    filter_ignored(projects)
}
