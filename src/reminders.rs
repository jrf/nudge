use anyhow::{Context, Result, bail};
use std::process::Command;

#[derive(Debug, Clone)]
pub struct Reminder {
    pub id: String,
    pub name: String,
    pub list: String,
    pub due_date: String,
    pub completed: bool,
    pub priority: i32,
}

fn run_applescript(script: &str) -> Result<String> {
    let output = Command::new("osascript")
        .arg("-e")
        .arg(script)
        .output()
        .context("Failed to run osascript")?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        bail!("AppleScript error: {}", err.trim());
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn escape_applescript(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

pub fn list_reminders(list: Option<&str>, show_completed: bool) -> Result<Vec<Reminder>> {
    let list_scope = match list {
        Some(l) => format!("{{list \"{}\"}}", escape_applescript(l)),
        None => "every list".to_string(),
    };

    let completed_filter = if show_completed {
        ""
    } else {
        "whose completed is false"
    };

    let script = format!(
        r#"tell application "Reminders"
            set output to {{}}
            repeat with l in {list_scope}
                set listName to name of l
                repeat with r in (every reminder of l {completed_filter})
                    set dueStr to ""
                    try
                        set d to due date of r
                        set dueStr to (year of d as text) & "-" & text -2 thru -1 of ("0" & (month of d as integer)) & "-" & text -2 thru -1 of ("0" & day of d)
                    end try
                    set pri to priority of r
                    set comp to completed of r
                    set end of output to listName & "|||" & (id of r) & "|||" & (name of r) & "|||" & dueStr & "|||" & (comp as text) & "|||" & (pri as text)
                end repeat
            end repeat
            set AppleScript's text item delimiters to "\n"
            return output as text
        end tell"#
    );

    let output = run_applescript(&script)?;
    if output.is_empty() {
        return Ok(vec![]);
    }

    Ok(output
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
        .collect())
}

pub fn search_reminders(query: &str) -> Result<Vec<Reminder>> {
    let escaped = escape_applescript(query);
    let script = format!(
        r#"tell application "Reminders"
            set output to {{}}
            repeat with l in every list
                set listName to name of l
                repeat with r in (every reminder of l whose name contains "{escaped}")
                    set dueStr to ""
                    try
                        set d to due date of r
                        set dueStr to (year of d as text) & "-" & text -2 thru -1 of ("0" & (month of d as integer)) & "-" & text -2 thru -1 of ("0" & day of d)
                    end try
                    set pri to priority of r
                    set comp to completed of r
                    set end of output to listName & "|||" & (id of r) & "|||" & (name of r) & "|||" & dueStr & "|||" & (comp as text) & "|||" & (pri as text)
                end repeat
            end repeat
            set AppleScript's text item delimiters to "\n"
            return output as text
        end tell"#
    );

    let output = run_applescript(&script)?;
    if output.is_empty() {
        return Ok(vec![]);
    }

    Ok(output
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
        .collect())
}

pub fn add_reminder(
    name: &str,
    list: Option<&str>,
    due_date: Option<&str>,
    priority: Option<i32>,
) -> Result<()> {
    let escaped_name = escape_applescript(name);

    let props = {
        let mut p = format!("name:\"{escaped_name}\"");
        if let Some(pri) = priority {
            p.push_str(&format!(", priority:{pri}"));
        }
        p
    };

    let due_clause = match due_date {
        Some(d) => {
            let escaped_date = escape_applescript(d);
            format!(
                r#"
                set due date of newReminder to date "{escaped_date}""#
            )
        }
        None => String::new(),
    };

    let list_target = match list {
        Some(l) => format!("list \"{}\"", escape_applescript(l)),
        None => "default list".to_string(),
    };

    let script = format!(
        r#"tell application "Reminders"
            set newReminder to make new reminder in {list_target} with properties {{{props}}}{due_clause}
        end tell"#
    );

    run_applescript(&script)?;
    Ok(())
}

pub fn complete_reminder(name: &str) -> Result<()> {
    let escaped = escape_applescript(name);
    let script = format!(
        r#"tell application "Reminders"
            repeat with l in every list
                set matched to (every reminder of l whose name contains "{escaped}" and completed is false)
                if (count of matched) > 0 then
                    set completed of item 1 of matched to true
                    return "done"
                end if
            end repeat
            error "No incomplete reminder found matching: {escaped}"
        end tell"#
    );

    run_applescript(&script)?;
    Ok(())
}

pub fn delete_reminder(name: &str) -> Result<()> {
    let escaped = escape_applescript(name);
    let script = format!(
        r#"tell application "Reminders"
            repeat with l in every list
                set matched to (every reminder of l whose name contains "{escaped}")
                if (count of matched) > 0 then
                    delete item 1 of matched
                    return "deleted"
                end if
            end repeat
            error "No reminder found matching: {escaped}"
        end tell"#
    );

    run_applescript(&script)?;
    Ok(())
}

pub fn list_lists() -> Result<Vec<String>> {
    let script = r#"tell application "Reminders"
            set listNames to name of every list
            set AppleScript's text item delimiters to "\n"
            return listNames as text
        end tell"#;

    let output = run_applescript(script)?;
    if output.is_empty() {
        return Ok(vec![]);
    }

    Ok(output.lines().map(|l| l.trim().to_string()).collect())
}
