mod reminders;
mod theme;
mod tui;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(about = "Apple Reminders from your terminal.")]
struct Cli {
    /// Color theme (synthwave, monochrome, ocean, sunset, forest, tokyo night moon)
    #[arg(long, default_value = "synthwave")]
    theme: String,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// List reminders
    List {
        /// Filter by list name
        #[arg(short, long)]
        list: Option<String>,
        /// Include completed reminders
        #[arg(short, long)]
        all: bool,
    },
    /// Search reminders by name
    Search {
        /// Search query
        query: String,
    },
    /// Add a new reminder
    Add {
        /// Reminder title
        name: String,
        /// List to add to
        #[arg(short, long)]
        list: Option<String>,
        /// Due date (e.g. "March 15, 2026" or "tomorrow")
        #[arg(short, long)]
        due: Option<String>,
        /// Priority (0=none, 1=high, 5=medium, 9=low)
        #[arg(short, long)]
        priority: Option<i32>,
    },
    /// Mark a reminder as completed
    Done {
        /// Reminder name (or partial match)
        name: String,
    },
    /// Delete a reminder
    Delete {
        /// Reminder name (or partial match)
        name: String,
    },
    /// List all reminder lists
    Lists,
}

fn format_reminder(r: &reminders::Reminder) -> String {
    let check = if r.completed { "x" } else { " " };
    let due = if r.due_date.is_empty() {
        String::new()
    } else {
        format!("  \x1b[2m({})\x1b[0m", r.due_date)
    };
    let pri = match r.priority {
        1 => " \x1b[31m!!!\x1b[0m",
        5 => " \x1b[33m!!\x1b[0m",
        9 => " \x1b[36m!\x1b[0m",
        _ => "",
    };
    format!(
        "  [{check}] \x1b[2m{}/\x1b[0m{}{pri}{due}",
        r.list, r.name
    )
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let theme = theme::find_theme(&cli.theme).unwrap_or_else(|| {
        eprintln!("Unknown theme '{}', using synthwave", cli.theme);
        theme::default_theme()
    });

    match cli.command {
        None => tui::run(theme),
        Some(cmd) => match cmd {
            Commands::List { list, all } => {
                let items = reminders::list_reminders(list.as_deref(), all)?;
                if items.is_empty() {
                    println!("No reminders found.");
                } else {
                    for r in items {
                        println!("{}", format_reminder(&r));
                    }
                }
                Ok(())
            }
            Commands::Search { query } => {
                let results = reminders::search_reminders(&query)?;
                if results.is_empty() {
                    println!("No reminders matching \"{}\".", query);
                } else {
                    println!("Found {} reminder(s):", results.len());
                    for r in results {
                        println!("{}", format_reminder(&r));
                    }
                }
                Ok(())
            }
            Commands::Add {
                name,
                list,
                due,
                priority,
            } => {
                reminders::add_reminder(&name, list.as_deref(), due.as_deref(), priority)?;
                println!("Added: {name}");
                Ok(())
            }
            Commands::Done { name } => {
                reminders::complete_reminder(&name)?;
                println!("Completed: {name}");
                Ok(())
            }
            Commands::Delete { name } => {
                reminders::delete_reminder(&name)?;
                println!("Deleted: {name}");
                Ok(())
            }
            Commands::Lists => {
                let lists = reminders::list_lists()?;
                for l in lists {
                    println!("  {l}");
                }
                Ok(())
            }
        },
    }
}
