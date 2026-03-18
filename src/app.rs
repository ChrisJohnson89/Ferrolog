use crate::parser::{LogEntry, LogLevel, LogParser};
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Clear, Paragraph, Row, Table, TableState, Wrap},
    Frame,
};
use std::time::Duration;

pub struct App {
    pub entries: Vec<LogEntry>,
    pub filtered: Vec<usize>,
    pub table_state: TableState,
    pub filter_level: Option<LogLevel>,
    pub search_query: String,
    pub search_mode: bool,
    pub show_help: bool,
    pub show_detail: bool,
    pub filename: String,
    pub should_quit: bool,
    pub follow_mode: bool,
    pub filepath: Option<String>,
    pub last_line_count: usize,
    pub last_file_len: u64,
}

impl App {
    pub fn new(
        entries: Vec<LogEntry>,
        filename: String,
        follow_mode: bool,
        filepath: Option<String>,
    ) -> Self {
        let last_line_count = entries.len();
        let filtered: Vec<usize> = (0..entries.len()).collect();
        let mut table_state = TableState::default();
        if !entries.is_empty() {
            // Start at bottom in follow mode, top otherwise
            let start = if follow_mode { entries.len() - 1 } else { 0 };
            table_state.select(Some(start));
        }
        let last_file_len = filepath
            .as_ref()
            .and_then(|p| std::fs::metadata(p).ok())
            .map(|m| m.len())
            .unwrap_or(0);
        Self {
            entries,
            filtered,
            table_state,
            filter_level: None,
            search_query: String::new(),
            search_mode: false,
            show_help: false,
            show_detail: false,
            filename,
            should_quit: false,
            follow_mode,
            filepath,
            last_line_count,
            last_file_len,
        }
    }

    pub fn apply_filters(&mut self) {
        // Clone filter state to avoid borrow conflicts inside the closure
        let filter_level = self.filter_level.clone();
        let search_query = self.search_query.to_lowercase();

        self.filtered = self
            .entries
            .iter()
            .enumerate()
            .filter(|(_, e)| {
                if let Some(ref lvl) = filter_level {
                    if &e.level != lvl {
                        return false;
                    }
                }
                if !search_query.is_empty()
                    && !e.message.to_lowercase().contains(&search_query)
                    && !e.raw.to_lowercase().contains(&search_query)
                {
                    return false;
                }
                true
            })
            .map(|(i, _)| i)
            .collect();

        if self.filtered.is_empty() {
            self.table_state.select(None);
        } else {
            self.table_state.select(Some(0));
        }
    }

    /// Poll the file for new lines when follow mode is active.
    pub fn check_follow(&mut self) -> std::io::Result<()> {
        if !self.follow_mode {
            return Ok(());
        }
        let path = match &self.filepath {
            Some(p) => p.clone(),
            None => return Ok(()),
        };

        // Quick size check to avoid redundant reads
        let file_len = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
        if file_len == self.last_file_len {
            return Ok(());
        }
        self.last_file_len = file_len;

        // Use read_to_string lossy equivalent: read bytes then convert
        let bytes = match std::fs::read(&path) {
            Ok(b) => b,
            Err(_) => return Ok(()), // transient error, skip this poll
        };
        let content = String::from_utf8_lossy(&bytes).into_owned();
        let all_lines: Vec<&str> = content.lines().collect();

        // Handle log rotation / truncation: reset if file has fewer lines than we've seen
        if all_lines.len() < self.last_line_count {
            self.last_line_count = 0;
        }

        if all_lines.len() > self.last_line_count {
            let parser = LogParser::new();
            for i in self.last_line_count..all_lines.len() {
                let entry = parser.parse_line(i + 1, all_lines[i]);
                self.entries.push(entry);
            }
            self.last_line_count = all_lines.len();

            // Rebuild filtered without resetting scroll
            let filter_level = self.filter_level.clone();
            let search_query = self.search_query.to_lowercase();
            self.filtered = self
                .entries
                .iter()
                .enumerate()
                .filter(|(_, e)| {
                    if let Some(ref lvl) = filter_level {
                        if &e.level != lvl {
                            return false;
                        }
                    }
                    if !search_query.is_empty()
                        && !e.message.to_lowercase().contains(&search_query)
                        && !e.raw.to_lowercase().contains(&search_query)
                    {
                        return false;
                    }
                    true
                })
                .map(|(i, _)| i)
                .collect();

            // Auto-scroll to bottom
            if !self.filtered.is_empty() {
                self.table_state.select(Some(self.filtered.len() - 1));
            } else {
                self.table_state.select(None);
            }
        }

        Ok(())
    }

    pub fn selected_entry(&self) -> Option<&LogEntry> {
        self.table_state
            .selected()
            .and_then(|i| self.filtered.get(i))
            .map(|&idx| &self.entries[idx])
    }

    fn move_selection(&mut self, delta: i32) {
        if self.filtered.is_empty() {
            return;
        }
        let len = self.filtered.len();
        let current = self.table_state.selected().unwrap_or(0) as i32;
        let next = (current + delta).clamp(0, len as i32 - 1) as usize;
        self.table_state.select(Some(next));
    }

    /// Jump to next search match with wrap-around.
    fn next_match(&mut self) {
        if self.filtered.is_empty() || self.search_query.is_empty() {
            return;
        }
        let len = self.filtered.len();
        let current = self.table_state.selected().unwrap_or(0);
        self.table_state.select(Some((current + 1) % len));
    }

    /// Jump to previous search match with wrap-around.
    fn prev_match(&mut self) {
        if self.filtered.is_empty() || self.search_query.is_empty() {
            return;
        }
        let len = self.filtered.len();
        let current = self.table_state.selected().unwrap_or(0);
        let prev = if current == 0 { len - 1 } else { current - 1 };
        self.table_state.select(Some(prev));
    }

    pub fn handle_events(&mut self) -> std::io::Result<()> {
        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    return Ok(());
                }

                if self.search_mode {
                    match key.code {
                        KeyCode::Esc => {
                            self.search_mode = false;
                        }
                        KeyCode::Enter => {
                            self.search_mode = false;
                            self.apply_filters();
                        }
                        KeyCode::Backspace => {
                            self.search_query.pop();
                            self.apply_filters();
                        }
                        KeyCode::Char(c) => {
                            self.search_query.push(c);
                            self.apply_filters();
                        }
                        _ => {}
                    }
                    return Ok(());
                }

                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => self.should_quit = true,
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        self.should_quit = true
                    }
                    KeyCode::Char('j') | KeyCode::Down => self.move_selection(1),
                    KeyCode::Char('k') | KeyCode::Up => self.move_selection(-1),
                    KeyCode::Char('g') | KeyCode::Home => {
                        if !self.filtered.is_empty() {
                            self.table_state.select(Some(0));
                        }
                    }
                    KeyCode::Char('G') | KeyCode::End => {
                        if !self.filtered.is_empty() {
                            self.table_state.select(Some(self.filtered.len() - 1));
                        }
                    }
                    KeyCode::PageDown => self.move_selection(20),
                    KeyCode::PageUp => self.move_selection(-20),
                    KeyCode::Char('/') => {
                        self.search_mode = true;
                    }
                    KeyCode::Char('n') => self.next_match(),
                    KeyCode::Char('N') => self.prev_match(),
                    KeyCode::Char('c') => {
                        self.search_query.clear();
                        self.filter_level = None;
                        self.apply_filters();
                    }
                    KeyCode::Char('f') => {
                        if self.filepath.is_some() {
                            self.follow_mode = !self.follow_mode;
                            if self.follow_mode && !self.filtered.is_empty() {
                                self.table_state.select(Some(self.filtered.len() - 1));
                            }
                        }
                    }
                    KeyCode::Char('1') => self.toggle_level_filter(LogLevel::Trace),
                    KeyCode::Char('2') => self.toggle_level_filter(LogLevel::Debug),
                    KeyCode::Char('3') => self.toggle_level_filter(LogLevel::Info),
                    KeyCode::Char('4') => self.toggle_level_filter(LogLevel::Warn),
                    KeyCode::Char('5') => self.toggle_level_filter(LogLevel::Error),
                    KeyCode::Char('6') => self.toggle_level_filter(LogLevel::Fatal),
                    KeyCode::Enter => {
                        self.show_detail = !self.show_detail;
                    }
                    KeyCode::Char('?') => {
                        self.show_help = !self.show_help;
                    }
                    _ => {}
                }
            }
        }
        Ok(())
    }

    fn toggle_level_filter(&mut self, level: LogLevel) {
        if self.filter_level.as_ref() == Some(&level) {
            self.filter_level = None;
        } else {
            self.filter_level = Some(level);
        }
        self.apply_filters();
    }
}

pub fn ui(frame: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // header
            Constraint::Min(5),    // table
            Constraint::Length(1), // status bar
        ])
        .split(frame.area());

    draw_header(frame, app, chunks[0]);

    if app.show_detail {
        let detail_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
            .split(chunks[1]);
        draw_table(frame, app, detail_chunks[0]);
        draw_detail(frame, app, detail_chunks[1]);
    } else {
        draw_table(frame, app, chunks[1]);
    }

    draw_status_bar(frame, app, chunks[2]);

    if app.show_help {
        draw_help_popup(frame, app);
    }
}

fn draw_header(frame: &mut Frame, app: &App, area: Rect) {
    let title = if app.search_mode {
        format!(" ferrolog  {}  Search: {}█", app.filename, app.search_query)
    } else {
        let filter_info = match &app.filter_level {
            Some(lvl) => format!("  Filter: {}", lvl),
            None => String::new(),
        };

        let count_info = if app.search_query.is_empty() {
            format!("  [{}/{}]", app.filtered.len(), app.entries.len())
        } else {
            let pos = app.table_state.selected().map(|i| i + 1).unwrap_or(0);
            format!(
                "  Search: \"{}\" [{}/{}]",
                app.search_query,
                pos,
                app.filtered.len()
            )
        };

        let follow_info = if app.follow_mode { "  [FOLLOW]" } else { "" };

        format!(
            " ferrolog  {}{}{}{}",
            app.filename, count_info, filter_info, follow_info,
        )
    };

    let header = Paragraph::new(title).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan))
            .title(" Ferrolog ")
            .title_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
    );
    frame.render_widget(header, area);
}

fn draw_table(frame: &mut Frame, app: &mut App, area: Rect) {
    let header_cells = ["#", "Timestamp", "Level", "Source", "Message"]
        .iter()
        .map(|h| {
            Cell::from(*h)
                .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
        });
    let header = Row::new(header_cells).height(1);

    let search_query = app.search_query.clone();

    let rows: Vec<Row> = app
        .filtered
        .iter()
        .map(|&idx| {
            let entry = &app.entries[idx];
            let level_style = level_color(&entry.level);
            Row::new(vec![
                Cell::from(entry.line_number.to_string())
                    .style(Style::default().fg(Color::DarkGray)),
                Cell::from(entry.timestamp.clone().unwrap_or_default())
                    .style(Style::default().fg(Color::Blue)),
                Cell::from(entry.level.to_string()).style(level_style),
                Cell::from(entry.source.clone().unwrap_or_default())
                    .style(Style::default().fg(Color::Magenta)),
                Cell::from(highlight_text(&entry.message, &search_query)),
            ])
        })
        .collect();

    let widths = [
        Constraint::Length(6),
        Constraint::Length(26),
        Constraint::Length(7),
        Constraint::Length(15),
        Constraint::Fill(1),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray)),
        )
        .row_highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">> ");

    frame.render_stateful_widget(table, area, &mut app.table_state);
}

fn draw_detail(frame: &mut Frame, app: &App, area: Rect) {
    let content = match app.selected_entry() {
        Some(entry) => {
            let mut lines = vec![
                Line::from(vec![
                    Span::styled("Line: ", Style::default().fg(Color::Yellow)),
                    Span::raw(entry.line_number.to_string()),
                ]),
                Line::from(vec![
                    Span::styled("Level: ", Style::default().fg(Color::Yellow)),
                    Span::styled(entry.level.to_string(), level_color(&entry.level)),
                ]),
            ];
            if let Some(ref ts) = entry.timestamp {
                lines.push(Line::from(vec![
                    Span::styled("Time:  ", Style::default().fg(Color::Yellow)),
                    Span::raw(ts.clone()),
                ]));
            }
            if let Some(ref src) = entry.source {
                lines.push(Line::from(vec![
                    Span::styled("Source:", Style::default().fg(Color::Yellow)),
                    Span::raw(format!(" {}", src)),
                ]));
            }
            lines.push(Line::from(""));
            lines.push(Line::from(vec![Span::styled(
                "Raw: ",
                Style::default().fg(Color::Yellow),
            )]));
            lines.push(Line::from(entry.raw.clone()));
            lines
        }
        None => vec![Line::from("No entry selected")],
    };

    let detail = Paragraph::new(content)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray))
                .title(" Detail ")
                .title_style(Style::default().fg(Color::Cyan)),
        )
        .wrap(Wrap { trim: false });
    frame.render_widget(detail, area);
}

fn draw_status_bar(frame: &mut Frame, app: &App, area: Rect) {
    let text = if app.search_mode {
        " Type to search | Enter: confirm | Esc: cancel"
    } else if !app.search_query.is_empty() {
        " j/k: navigate | n/N: next/prev match | /: new search | c: clear | Enter: detail | ?: help | q: quit"
    } else {
        " j/k: navigate | /: search | 1-6: filter level | f: follow | c: clear | Enter: detail | ?: help | q: quit"
    };
    let bar = Paragraph::new(text).style(Style::default().fg(Color::DarkGray));
    frame.render_widget(bar, area);
}

fn draw_help_popup(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let popup_area = Rect {
        x: area.width / 4,
        y: area.height / 4,
        width: area.width / 2,
        height: area.height / 2,
    };

    let follow_hint = if app.filepath.is_some() {
        "  f               Toggle follow mode"
    } else {
        "  f               Follow mode (file only)"
    };

    let help_text = vec![
        Line::from(Span::styled(
            "Keybindings",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from("  j / Down      Move down"),
        Line::from("  k / Up        Move up"),
        Line::from("  g / Home      Go to top"),
        Line::from("  G / End       Go to bottom"),
        Line::from("  PgDn/PgUp     Scroll by 20"),
        Line::from("  /             Search"),
        Line::from("  n             Next match"),
        Line::from("  N             Previous match"),
        Line::from("  1             Filter: TRACE"),
        Line::from("  2             Filter: DEBUG"),
        Line::from("  3             Filter: INFO"),
        Line::from("  4             Filter: WARN"),
        Line::from("  5             Filter: ERROR"),
        Line::from("  6             Filter: FATAL"),
        Line::from("  c             Clear filters"),
        Line::from(follow_hint),
        Line::from("  Enter         Toggle detail view"),
        Line::from("  ?             Toggle this help"),
        Line::from("  q / Esc       Quit"),
    ];

    frame.render_widget(Clear, popup_area);
    let popup = Paragraph::new(help_text).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow))
            .title(" Help ")
            .title_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
    );
    frame.render_widget(popup, popup_area);
}

fn level_color(level: &LogLevel) -> Style {
    match level {
        LogLevel::Trace => Style::default().fg(Color::DarkGray),
        LogLevel::Debug => Style::default().fg(Color::Cyan),
        LogLevel::Info => Style::default().fg(Color::Green),
        LogLevel::Warn => Style::default().fg(Color::Yellow),
        LogLevel::Error => Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        LogLevel::Fatal => Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        LogLevel::Unknown => Style::default().fg(Color::White),
    }
}

/// Split `text` into styled spans, highlighting every case-insensitive occurrence of `query`.
fn highlight_text(text: &str, query: &str) -> Line<'static> {
    if query.is_empty() {
        return Line::from(text.to_owned());
    }

    let lower_text = text.to_lowercase();
    let lower_query = query.to_lowercase();
    let qlen = lower_query.len();

    let mut spans: Vec<Span<'static>> = Vec::new();
    let mut byte_offset = 0;

    while byte_offset < lower_text.len() {
        match lower_text[byte_offset..].find(lower_query.as_str()) {
            None => {
                spans.push(Span::raw(text[byte_offset..].to_owned()));
                byte_offset = lower_text.len();
            }
            Some(rel) => {
                let match_start = byte_offset + rel;
                let match_end = match_start + qlen;

                // Guard against char boundary mismatches on non-ASCII input
                if !text.is_char_boundary(match_start) || !text.is_char_boundary(match_end) {
                    spans.push(Span::raw(text[byte_offset..].to_owned()));
                    break;
                }

                if match_start > byte_offset {
                    spans.push(Span::raw(text[byte_offset..match_start].to_owned()));
                }
                spans.push(Span::styled(
                    text[match_start..match_end].to_owned(),
                    Style::default().bg(Color::Yellow).fg(Color::Black),
                ));
                byte_offset = match_end;
            }
        }
    }

    if spans.is_empty() {
        Line::from(text.to_owned())
    } else {
        Line::from(spans)
    }
}
