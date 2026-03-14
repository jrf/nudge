use crate::reminders::{self, Reminder};
use crate::theme::{self, Theme};
use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{self, EnterAlternateScreen, LeaveAlternateScreen};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph};
use ratatui::DefaultTerminal;
use std::io::stdout;

enum Mode {
    Browse,
    Search,
    Add,
    Help,
    ThemePicker,
    ListPicker,
    MovePicker,
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
    theme: Theme,
    theme_selected: usize,
    theme_before_preview: Theme,
    lists: Vec<String>,
    list_selected: usize,
    active_list: Option<String>,
    move_selected: usize,
}

impl App {
    fn new(reminders: Vec<Reminder>, theme: Theme) -> Self {
        let filtered: Vec<usize> = (0..reminders.len()).collect();
        let mut list_state = ListState::default();
        if !filtered.is_empty() {
            list_state.select(Some(0));
        }
        let theme_selected = theme::ALL_THEMES
            .iter()
            .position(|(_, t)| t.accent == theme.accent && t.border == theme.border)
            .unwrap_or(0);
        Self {
            reminders,
            filtered,
            list_state,
            search_query: String::new(),
            add_input: String::new(),
            mode: Mode::Browse,
            should_quit: false,
            confirm_delete: false,
            theme,
            theme_selected,
            theme_before_preview: theme,
            lists: vec![],
            list_selected: 0,
            active_list: None,
            move_selected: 0,
        }
    }

    fn load_lists(&mut self) {
        if let Ok(lists) = reminders::list_lists() {
            self.lists = lists;
        }
    }

    fn apply_filter(&mut self) {
        let query = self.search_query.to_lowercase();
        self.filtered = self
            .reminders
            .iter()
            .enumerate()
            .filter(|(_, r)| {
                if let Some(ref active) = self.active_list {
                    if r.list != *active {
                        return false;
                    }
                }
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

pub fn run(theme: Theme) -> Result<()> {
    let items = reminders::list_reminders(None, false)?;
    if items.is_empty() {
        println!("No reminders found.");
        return Ok(());
    }

    let mut app = App::new(items, theme);

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
                    KeyCode::Char('t') => {
                        app.theme_before_preview = app.theme;
                        app.mode = Mode::ThemePicker;
                    }
                    KeyCode::Char('m') => {
                        if app.selected_reminder().is_some() {
                            app.load_lists();
                            app.move_selected = 0;
                            app.mode = Mode::MovePicker;
                        }
                    }
                    KeyCode::Char('g') => {
                        app.load_lists();
                        app.list_selected = match &app.active_list {
                            Some(name) => app
                                .lists
                                .iter()
                                .position(|l| l == name)
                                .map(|i| i + 1)
                                .unwrap_or(0),
                            None => 0,
                        };
                        app.mode = Mode::ListPicker;
                    }
                    KeyCode::Char('r') => {
                        let _ = app.refresh();
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
                Mode::ThemePicker => match key.code {
                    KeyCode::Esc | KeyCode::Char('t') => {
                        app.theme = app.theme_before_preview;
                        app.mode = Mode::Browse;
                    }
                    KeyCode::Char('j') | KeyCode::Down => {
                        if app.theme_selected + 1 < theme::ALL_THEMES.len() {
                            app.theme_selected += 1;
                            app.theme = theme::ALL_THEMES[app.theme_selected].1;
                        }
                    }
                    KeyCode::Char('k') | KeyCode::Up => {
                        if app.theme_selected > 0 {
                            app.theme_selected -= 1;
                            app.theme = theme::ALL_THEMES[app.theme_selected].1;
                        }
                    }
                    KeyCode::Enter => {
                        app.theme = theme::ALL_THEMES[app.theme_selected].1;
                        app.mode = Mode::Browse;
                    }
                    _ => {}
                },
                Mode::ListPicker => match key.code {
                    KeyCode::Esc | KeyCode::Char('g') => {
                        app.mode = Mode::Browse;
                    }
                    KeyCode::Char('j') | KeyCode::Down => {
                        let total = app.lists.len() + 1; // +1 for "All"
                        if app.list_selected + 1 < total {
                            app.list_selected += 1;
                        }
                    }
                    KeyCode::Char('k') | KeyCode::Up => {
                        if app.list_selected > 0 {
                            app.list_selected -= 1;
                        }
                    }
                    KeyCode::Enter => {
                        if app.list_selected == 0 {
                            app.active_list = None;
                        } else {
                            app.active_list =
                                Some(app.lists[app.list_selected - 1].clone());
                        }
                        app.apply_filter();
                        app.mode = Mode::Browse;
                    }
                    _ => {}
                },
                Mode::MovePicker => match key.code {
                    KeyCode::Esc | KeyCode::Char('m') => {
                        app.mode = Mode::Browse;
                    }
                    KeyCode::Char('j') | KeyCode::Down => {
                        if app.move_selected + 1 < app.lists.len() {
                            app.move_selected += 1;
                        }
                    }
                    KeyCode::Char('k') | KeyCode::Up => {
                        if app.move_selected > 0 {
                            app.move_selected -= 1;
                        }
                    }
                    KeyCode::Enter => {
                        if let Some(r) = app.selected_reminder() {
                            let id = r.id.clone();
                            let target = app.lists[app.move_selected].clone();
                            let _ = reminders::move_reminder(&id, &target);
                            let _ = app.refresh();
                        }
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
    let t = &app.theme;

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
                .style(Style::default().fg(t.accent))
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(t.border))
                        .title(" New Reminder ")
                        .title_style(Style::default().fg(t.accent).add_modifier(Modifier::BOLD)),
                );
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
                Mode::Search => Style::default().fg(t.accent),
                _ => Style::default().fg(t.text_muted),
            };

            let search = Paragraph::new(search_text).style(search_style).block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(t.border))
                    .title(" Search ")
                    .title_style(Style::default().fg(t.accent).add_modifier(Modifier::BOLD)),
            );
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
                Span::styled(format!("[{check}] "), Style::default().fg(t.text_muted)),
                Span::styled(format!("{}/", r.list), Style::default().fg(t.text_muted)),
                Span::styled(&r.name, Style::default().fg(t.text)),
            ];
            match r.priority {
                1 => spans.push(Span::styled(" !!!", Style::default().fg(t.error))),
                5 => spans.push(Span::styled(" !!", Style::default().fg(t.accent))),
                9 => spans.push(Span::styled(" !", Style::default().fg(t.text_dim))),
                _ => {}
            }
            if !r.due_date.is_empty() {
                spans.push(Span::styled(
                    format!("  ({})", r.due_date),
                    Style::default().fg(t.text_muted),
                ));
            }
            ListItem::new(Line::from(spans))
        })
        .collect();

    let title = match &app.active_list {
        Some(name) => format!(" Reminders - {} ", name),
        None => " Reminders ".to_string(),
    };

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(t.border))
                .title(title)
                .title_style(Style::default().fg(t.accent).add_modifier(Modifier::BOLD)),
        )
        .highlight_style(
            Style::default()
                .fg(t.text_bright)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▸ ");

    frame.render_stateful_widget(list, chunks[1], &mut app.list_state);

    // Status bar
    let status = if app.confirm_delete {
        Line::from(vec![
            Span::styled(" Delete reminder? ", Style::default().fg(t.error)),
            Span::styled("y", Style::default().fg(t.accent)),
            Span::styled(" yes  ", Style::default().fg(t.text)),
            Span::styled("n", Style::default().fg(t.accent)),
            Span::styled(" no", Style::default().fg(t.text)),
        ])
    } else {
        Line::from(vec![
            Span::styled(" ?", Style::default().fg(t.accent)),
            Span::styled(" help  ", Style::default().fg(t.text_dim)),
            Span::styled("q", Style::default().fg(t.accent)),
            Span::styled(" quit", Style::default().fg(t.text_dim)),
        ])
    };
    frame.render_widget(Paragraph::new(status), chunks[2]);

    // Overlays
    match app.mode {
        Mode::Help => draw_help(frame, t),
        Mode::ThemePicker => draw_theme_picker(frame, app),
        Mode::ListPicker => draw_list_picker(frame, app),
        Mode::MovePicker => draw_move_picker(frame, app),
        _ => {}
    }
}

fn draw_help(frame: &mut ratatui::Frame, t: &Theme) {
    let area = frame.area();
    let width = 44u16.min(area.width.saturating_sub(4));
    let height = 19u16.min(area.height.saturating_sub(4));
    let x = (area.width.saturating_sub(width)) / 2;
    let y = (area.height.saturating_sub(height)) / 2;
    let popup = Rect::new(x, y, width, height);

    frame.render_widget(Clear, popup);

    let help_lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("  ↑↓ / j k  ", Style::default().fg(t.accent)),
            Span::styled("Navigate reminders", Style::default().fg(t.text)),
        ]),
        Line::from(vec![
            Span::styled("  ⏎ Enter   ", Style::default().fg(t.accent)),
            Span::styled("Complete selected reminder", Style::default().fg(t.text)),
        ]),
        Line::from(vec![
            Span::styled("  a         ", Style::default().fg(t.accent)),
            Span::styled("Add new reminder", Style::default().fg(t.text)),
        ]),
        Line::from(vec![
            Span::styled("  r         ", Style::default().fg(t.accent)),
            Span::styled("Refresh from Reminders.app", Style::default().fg(t.text)),
        ]),
        Line::from(vec![
            Span::styled("  d         ", Style::default().fg(t.accent)),
            Span::styled("Delete selected reminder", Style::default().fg(t.text)),
        ]),
        Line::from(vec![
            Span::styled("  m         ", Style::default().fg(t.accent)),
            Span::styled("Move to list", Style::default().fg(t.text)),
        ]),
        Line::from(vec![
            Span::styled("  g         ", Style::default().fg(t.accent)),
            Span::styled("Filter by list", Style::default().fg(t.text)),
        ]),
        Line::from(vec![
            Span::styled("  t         ", Style::default().fg(t.accent)),
            Span::styled("Change theme", Style::default().fg(t.text)),
        ]),
        Line::from(vec![
            Span::styled("  /         ", Style::default().fg(t.accent)),
            Span::styled("Search reminders", Style::default().fg(t.text)),
        ]),
        Line::from(vec![
            Span::styled("  Esc       ", Style::default().fg(t.accent)),
            Span::styled("Cancel / back", Style::default().fg(t.text)),
        ]),
        Line::from(vec![
            Span::styled("  ?         ", Style::default().fg(t.accent)),
            Span::styled("Toggle this help", Style::default().fg(t.text)),
        ]),
        Line::from(vec![
            Span::styled("  q         ", Style::default().fg(t.accent)),
            Span::styled("Quit", Style::default().fg(t.text)),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "    Press ? or Esc to close",
            Style::default().fg(t.text_muted),
        )),
    ];

    let help = Paragraph::new(help_lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Help ")
            .title_style(Style::default().fg(t.accent).add_modifier(Modifier::BOLD))
            .border_style(Style::default().fg(t.accent)),
    );
    frame.render_widget(help, popup);
}

fn draw_theme_picker(frame: &mut ratatui::Frame, app: &App) {
    let t = &app.theme;
    let area = frame.area();
    let height = (theme::ALL_THEMES.len() as u16 + 4).min(area.height.saturating_sub(4));
    let width = 30u16.min(area.width.saturating_sub(4));
    let x = (area.width.saturating_sub(width)) / 2;
    let y = (area.height.saturating_sub(height)) / 2;
    let popup = Rect::new(x, y, width, height);

    frame.render_widget(Clear, popup);

    let items: Vec<ListItem> = theme::ALL_THEMES
        .iter()
        .enumerate()
        .map(|(i, (name, _))| {
            let style = if i == app.theme_selected {
                Style::default()
                    .fg(t.text_bright)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(t.text)
            };
            ListItem::new(Span::styled(format!("  {name}"), style))
        })
        .collect();

    let mut state = ListState::default();
    state.select(Some(app.theme_selected));

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Theme ")
                .title_style(Style::default().fg(t.accent).add_modifier(Modifier::BOLD))
                .border_style(Style::default().fg(t.accent)),
        )
        .highlight_style(
            Style::default()
                .fg(t.text_bright)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▸ ");

    frame.render_stateful_widget(list, popup, &mut state);
}

fn draw_list_picker(frame: &mut ratatui::Frame, app: &App) {
    let t = &app.theme;
    let area = frame.area();
    let total = app.lists.len() + 1; // +1 for "All"
    let height = (total as u16 + 4).min(area.height.saturating_sub(4));
    let width = 30u16.min(area.width.saturating_sub(4));
    let x = (area.width.saturating_sub(width)) / 2;
    let y = (area.height.saturating_sub(height)) / 2;
    let popup = Rect::new(x, y, width, height);

    frame.render_widget(Clear, popup);

    let mut entries: Vec<ListItem> = vec![ListItem::new(Span::styled(
        "  All",
        Style::default().fg(t.text),
    ))];

    entries.extend(app.lists.iter().map(|name| {
        ListItem::new(Span::styled(
            format!("  {name}"),
            Style::default().fg(t.text),
        ))
    }));

    let mut state = ListState::default();
    state.select(Some(app.list_selected));

    let list = List::new(entries)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" List ")
                .title_style(Style::default().fg(t.accent).add_modifier(Modifier::BOLD))
                .border_style(Style::default().fg(t.accent)),
        )
        .highlight_style(
            Style::default()
                .fg(t.text_bright)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▸ ");

    frame.render_stateful_widget(list, popup, &mut state);
}

fn draw_move_picker(frame: &mut ratatui::Frame, app: &App) {
    let t = &app.theme;
    let area = frame.area();
    let height = (app.lists.len() as u16 + 4).min(area.height.saturating_sub(4));
    let width = 30u16.min(area.width.saturating_sub(4));
    let x = (area.width.saturating_sub(width)) / 2;
    let y = (area.height.saturating_sub(height)) / 2;
    let popup = Rect::new(x, y, width, height);

    frame.render_widget(Clear, popup);

    let items: Vec<ListItem> = app
        .lists
        .iter()
        .map(|name| {
            ListItem::new(Span::styled(
                format!("  {name}"),
                Style::default().fg(t.text),
            ))
        })
        .collect();

    let mut state = ListState::default();
    state.select(Some(app.move_selected));

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Move to ")
                .title_style(Style::default().fg(t.accent).add_modifier(Modifier::BOLD))
                .border_style(Style::default().fg(t.accent)),
        )
        .highlight_style(
            Style::default()
                .fg(t.text_bright)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▸ ");

    frame.render_stateful_widget(list, popup, &mut state);
}
