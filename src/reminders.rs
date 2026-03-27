use anyhow::{Context, Result, bail};
use std::path::PathBuf;
use std::process::Command;

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct Reminder {
    pub id: String,
    pub name: String,
    pub list: String,
    pub due_date: String,
    pub completed: bool,
    pub priority: i32,
}

fn bridge_path() -> PathBuf {
    let exe = std::env::current_exe().unwrap_or_default();
    let dir = exe.parent().unwrap_or(std::path::Path::new("."));
    dir.join("nudge-bridge")
}

fn run_bridge(args: &[&str]) -> Result<String> {
    let path = bridge_path();
    let output = Command::new(&path)
        .args(args)
        .output()
        .with_context(|| format!("Failed to run nudge-bridge at {:?}", path))?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        bail!("{}", err.trim());
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn parse_reminders(output: &str) -> Vec<Reminder> {
    if output.is_empty() {
        return vec![];
    }
    output
        .lines()
        .filter_map(|line| {
            let parts: Vec<&str> = line.splitn(6, "|||").collect();
            if parts.len() >= 6 {
                Some(Reminder {
                    list: parts[0].trim().to_string(),
                    id: parts[1].trim().to_string(),
                    name: parts[2].trim().to_string(),
                    due_date: parts[3].trim().to_string(),
                    completed: parts[4].trim() == "true",
                    priority: parts[5].trim().parse().unwrap_or(0),
                })
            } else {
                None
            }
        })
        .collect()
}

pub fn list_reminders(list: Option<&str>, show_completed: bool) -> Result<Vec<Reminder>> {
    let mut args = vec!["list"];
    if let Some(l) = list {
        args.push("--list");
        args.push(l);
    }
    if show_completed {
        args.push("--all");
    }
    let output = run_bridge(&args)?;
    Ok(parse_reminders(&output))
}

pub fn search_reminders(query: &str) -> Result<Vec<Reminder>> {
    let output = run_bridge(&["search", query])?;
    Ok(parse_reminders(&output))
}

pub fn add_reminder(
    name: &str,
    list: Option<&str>,
    due_date: Option<&str>,
    priority: Option<i32>,
) -> Result<()> {
    let mut args = vec!["add", name];
    if let Some(l) = list {
        args.push("--list");
        args.push(l);
    }
    let due_str;
    if let Some(d) = due_date {
        args.push("--due");
        due_str = d.to_string();
        args.push(&due_str);
    }
    let pri_str;
    if let Some(p) = priority {
        args.push("--priority");
        pri_str = p.to_string();
        args.push(&pri_str);
    }
    run_bridge(&args)?;
    Ok(())
}

pub fn complete_reminder(name: &str) -> Result<()> {
    run_bridge(&["complete", name])?;
    Ok(())
}

pub fn delete_reminder(name: &str) -> Result<()> {
    run_bridge(&["delete", name])?;
    Ok(())
}

pub fn move_reminder(id: &str, to_list: &str) -> Result<()> {
    run_bridge(&["move", id, "--list", to_list])?;
    Ok(())
}

pub fn uncomplete_reminder(id: &str) -> Result<()> {
    run_bridge(&["uncomplete", id])?;
    Ok(())
}

pub fn edit_reminder(id: &str, new_name: &str) -> Result<()> {
    run_bridge(&["edit", id, new_name])?;
    Ok(())
}

pub fn create_list(name: &str) -> Result<()> {
    run_bridge(&["create-list", name])?;
    Ok(())
}

pub fn rename_list(old_name: &str, new_name: &str) -> Result<()> {
    run_bridge(&["rename-list", old_name, new_name])?;
    Ok(())
}

pub fn delete_list(name: &str) -> Result<()> {
    run_bridge(&["delete-list", name])?;
    Ok(())
}

pub fn list_lists() -> Result<Vec<String>> {
    let output = run_bridge(&["lists"])?;
    if output.is_empty() {
        return Ok(vec![]);
    }
    Ok(output.lines().map(|l| l.trim().to_string()).collect())
}
