use crate::reminders::{self, Reminder};
use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{self, EnterAlternateScreen, LeaveAlternateScreen};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph};
use ratatui::DefaultTerminal;
use std::io::stdout;

enum Mode {
    Browse,
    Search,
    Add,
    Help,
}

struct App {
    reminders: Vec<Reminder>,
    filtered: Vec<usize>,
    list_state: ListState,
    search_query: String,
    add_input: String,
    mode: Mode,
    should_quit: bool,
    confirm_delete: bool,
}

impl App {
    fn new(reminders: Vec<Reminder>) -> Self {
        let filtered: Vec<usize> = (0..reminders.len()).collect();
        let mut list_state = ListState::default();
        if !filtered.is_empty() {
            list_state.select(Some(0));
        }
        Self {
            reminders,
            filtered,
            list_state,
            search_query: String::new(),
            add_input: String::new(),
            mode: Mode::Browse,
            should_quit: false,
            confirm_delete: false,
        }
    }

    fn apply_filter(&mut self) {
        let query = self.search_query.to_lowercase();
        self.filtered = self
            .reminders
            .iter()
            .enumerate()
            .filter(|(_, r)| {
                query.is_empty()
                    || r.name.to_lowercase().contains(&query)
                    || r.list.to_lowercase().contains(&query)
            })
            .map(|(i, _)| i)
            .collect();

        if self.filtered.is_empty() {
            self.list_state.select(None);
        } else {
            self.list_state.select(Some(0));
        }
    }

    fn selected_reminder(&self) -> Option<&Reminder> {
        self.list_state
            .selected()
            .and_then(|i| self.filtered.get(i))
            .map(|&idx| &self.reminders[idx])
    }

    fn move_up(&mut self) {
        if let Some(selected) = self.list_state.selected() {
            if selected > 0 {
                self.list_state.select(Some(selected - 1));
            }
        }
    }

    fn move_down(&mut self) {
        if let Some(selected) = self.list_state.selected() {
            if selected + 1 < self.filtered.len() {
                self.list_state.select(Some(selected + 1));
            }
        }
    }

    fn refresh(&mut self) -> Result<()> {
        self.reminders = reminders::list_reminders(None, false)?;
        self.apply_filter();
        Ok(())
    }
}

pub fn run() -> Result<()> {
    let items = reminders::list_reminders(None, false)?;
    if items.is_empty() {
        println!("No reminders found.");
        return Ok(());
    }

    let mut app = App::new(items);

    terminal::enable_raw_mode()?;
    execute!(stdout(), EnterAlternateScreen)?;
    let mut terminal = ratatui::init();

    let result = run_loop(&mut terminal, &mut app);

    ratatui::restore();
    execute!(stdout(), LeaveAlternateScreen)?;
    terminal::disable_raw_mode()?;

    result
}

fn run_loop(terminal: &mut DefaultTerminal, app: &mut App) -> Result<()> {
    loop {
        terminal.draw(|frame| draw(frame, app))?;

        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press {
                continue;
            }

            match app.mode {
                Mode::Browse => match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => app.should_quit = true,
                    KeyCode::Char('j') | KeyCode::Down => app.move_down(),
                    KeyCode::Char('k') | KeyCode::Up => app.move_up(),
                    KeyCode::Char('/') => {
                        app.mode = Mode::Search;
                    }
                    KeyCode::Char('?') => {
                        app.mode = Mode::Help;
                    }
                    KeyCode::Char('a') => {
                        app.add_input.clear();
                        app.mode = Mode::Add;
                    }
                    KeyCode::Char('d') => {
                        if app.selected_reminder().is_some() {
                            app.confirm_delete = true;
                        }
                    }
                    KeyCode::Char('y') if app.confirm_delete => {
                        if let Some(r) = app.selected_reminder() {
                            let name = r.name.clone();
                            let _ = reminders::delete_reminder(&name);
                            let _ = app.refresh();
                        }
                        app.confirm_delete = false;
                    }
                    KeyCode::Char('n') if app.confirm_delete => {
                        app.confirm_delete = false;
                    }
                    KeyCode::Enter => {
                        if let Some(r) = app.selected_reminder() {
                            if !r.completed {
                                let name = r.name.clone();
                                let _ = reminders::complete_reminder(&name);
                                let _ = app.refresh();
                            }
                        }
                    }
                    _ => {
                        if app.confirm_delete {
                            app.confirm_delete = false;
                        }
                    }
                },
                Mode::Search => match key.code {
                    KeyCode::Esc => {
                        app.mode = Mode::Browse;
                        app.search_query.clear();
                        app.apply_filter();
                    }
                    KeyCode::Enter => {
                        app.mode = Mode::Browse;
                    }
                    KeyCode::Backspace => {
                        app.search_query.pop();
                        app.apply_filter();
                    }
                    KeyCode::Char(c) => {
                        app.search_query.push(c);
                        app.apply_filter();
                    }
                    _ => {}
                },
                Mode::Add => match key.code {
                    KeyCode::Esc => {
                        app.mode = Mode::Browse;
                        app.add_input.clear();
                    }
                    KeyCode::Enter => {
                        if !app.add_input.is_empty() {
                            let _ = reminders::add_reminder(&app.add_input, None, None, None);
                            let _ = app.refresh();
                        }
                        app.add_input.clear();
                        app.mode = Mode::Browse;
                    }
                    KeyCode::Backspace => {
                        app.add_input.pop();
                    }
                    KeyCode::Char(c) => {
                        app.add_input.push(c);
                    }
                    _ => {}
                },
                Mode::Help => match key.code {
                    KeyCode::Char('?') | KeyCode::Esc | KeyCode::Char('q') => {
                        app.mode = Mode::Browse;
                    }
                    _ => {}
                },
            }
        }

        if app.should_quit {
            break;
        }
    }

    Ok(())
}

fn draw(frame: &mut ratatui::Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(1),
            Constraint::Length(1),
        ])
        .split(frame.area());

    // Top bar — search or add input
    match app.mode {
        Mode::Add => {
            let text = if app.add_input.is_empty() {
                String::from("_")
            } else {
                format!("{}_", app.add_input)
            };
            let input = Paragraph::new(text)
                .style(Style::default().fg(Color::Green))
                .block(Block::default().borders(Borders::ALL).title(" New Reminder "));
            frame.render_widget(input, chunks[0]);
        }
        _ => {
            let search_text = if app.search_query.is_empty() {
                match app.mode {
                    Mode::Search => String::from("_"),
                    _ => String::from("Press / to search"),
                }
            } else {
                match app.mode {
                    Mode::Search => format!("{}_", app.search_query),
                    _ => app.search_query.clone(),
                }
            };

            let search_style = match app.mode {
                Mode::Search => Style::default().fg(Color::Yellow),
                _ => Style::default().fg(Color::DarkGray),
            };

            let search = Paragraph::new(search_text)
                .style(search_style)
                .block(Block::default().borders(Borders::ALL).title(" Search "));
            frame.render_widget(search, chunks[0]);
        }
    }

    // Reminder list
    let items: Vec<ListItem> = app
        .filtered
        .iter()
        .map(|&idx| {
            let r = &app.reminders[idx];
            let check = if r.completed { "x" } else { " " };
            let mut spans = vec![
                Span::styled(format!("[{check}] "), Style::default().fg(Color::DarkGray)),
                Span::styled(format!("{}/", r.list), Style::default().fg(Color::DarkGray)),
                Span::styled(&r.name, Style::default().fg(Color::White)),
            ];
            match r.priority {
                1 => spans.push(Span::styled(" !!!", Style::default().fg(Color::Red))),
                5 => spans.push(Span::styled(" !!", Style::default().fg(Color::Yellow))),
                9 => spans.push(Span::styled(" !", Style::default().fg(Color::Cyan))),
                _ => {}
            }
            if !r.due_date.is_empty() {
                spans.push(Span::styled(
                    format!("  ({})", r.due_date),
                    Style::default().fg(Color::DarkGray),
                ));
            }
            ListItem::new(Line::from(spans))
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Reminders "),
        )
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▸ ");

    frame.render_stateful_widget(list, chunks[1], &mut app.list_state);

    // Status bar
    let status = if app.confirm_delete {
        Line::from(vec![
            Span::styled(" Delete reminder? ", Style::default().fg(Color::Red)),
            Span::styled("y", Style::default().fg(Color::Cyan)),
            Span::raw(" yes  "),
            Span::styled("n", Style::default().fg(Color::Cyan)),
            Span::raw(" no"),
        ])
    } else {
        Line::from(vec![
            Span::styled(" ↑↓/jk", Style::default().fg(Color::Cyan)),
            Span::raw(" navigate  "),
            Span::styled("⏎", Style::default().fg(Color::Cyan)),
            Span::raw(" complete  "),
            Span::styled("a", Style::default().fg(Color::Cyan)),
            Span::raw(" add  "),
            Span::styled("d", Style::default().fg(Color::Cyan)),
            Span::raw(" delete  "),
            Span::styled("/", Style::default().fg(Color::Cyan)),
            Span::raw(" search  "),
            Span::styled("?", Style::default().fg(Color::Cyan)),
            Span::raw(" help  "),
            Span::styled("q", Style::default().fg(Color::Cyan)),
            Span::raw(" quit"),
        ])
    };
    frame.render_widget(Paragraph::new(status), chunks[2]);

    // Help overlay
    if matches!(app.mode, Mode::Help) {
        draw_help(frame);
    }
}

fn draw_help(frame: &mut ratatui::Frame) {
    let area = frame.area();
    let width = 44u16.min(area.width.saturating_sub(4));
    let height = 16u16.min(area.height.saturating_sub(4));
    let x = (area.width.saturating_sub(width)) / 2;
    let y = (area.height.saturating_sub(height)) / 2;
    let popup = Rect::new(x, y, width, height);

    frame.render_widget(Clear, popup);

    let help_lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("  ↑↓ / j k  ", Style::default().fg(Color::Cyan)),
            Span::raw("Navigate reminders"),
        ]),
        Line::from(vec![
            Span::styled("  ⏎ Enter   ", Style::default().fg(Color::Cyan)),
            Span::raw("Complete selected reminder"),
        ]),
        Line::from(vec![
            Span::styled("  a         ", Style::default().fg(Color::Cyan)),
            Span::raw("Add new reminder"),
        ]),
        Line::from(vec![
            Span::styled("  d         ", Style::default().fg(Color::Cyan)),
            Span::raw("Delete selected reminder"),
        ]),
        Line::from(vec![
            Span::styled("  /         ", Style::default().fg(Color::Cyan)),
            Span::raw("Search reminders"),
        ]),
        Line::from(vec![
            Span::styled("  Esc       ", Style::default().fg(Color::Cyan)),
            Span::raw("Cancel / back"),
        ]),
        Line::from(vec![
            Span::styled("  ?         ", Style::default().fg(Color::Cyan)),
            Span::raw("Toggle this help"),
        ]),
        Line::from(vec![
            Span::styled("  q         ", Style::default().fg(Color::Cyan)),
            Span::raw("Quit"),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "    Press ? or Esc to close",
            Style::default().fg(Color::DarkGray),
        )),
    ];

    let help = Paragraph::new(help_lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Help ")
            .border_style(Style::default().fg(Color::Yellow)),
    );
    frame.render_widget(help, popup);
}
