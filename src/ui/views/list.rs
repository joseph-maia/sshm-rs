use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Row, Table};
use ratatui::Frame;

use crate::connectivity::HostStatus;
use crate::ui::app::{App, SortMode, ViewMode};
use crate::ui::styles;

const ASCII_TITLE: &str = r#"
         _
 ___ ___| |_ _____    ___ ___
|_ -|_ -|   |     |__|  _|_ -|
|___|___|_|_|_|_|_|__|_| |___|
"#;

pub fn draw(f: &mut Frame, app: &App) {
    let area = f.area();

    // Clear background with Tokyo Night bg
    let bg_block = Block::default().style(Style::default().bg(styles::BG));
    f.render_widget(bg_block, area);

    // Main layout: title, search, table, status bar
    let chunks = Layout::vertical([
        Constraint::Length(5),  // ASCII title
        Constraint::Length(3),  // Search bar
        Constraint::Min(3),    // Table
        Constraint::Length(1), // Status bar
    ])
    .split(area);

    draw_title(f, chunks[0]);
    draw_search_bar(f, app, chunks[1]);
    draw_table(f, app, chunks[2]);
    draw_status_bar(f, app, chunks[3]);

    // Overlay views
    match app.view_mode {
        ViewMode::DeleteConfirm => draw_delete_confirm(f, app, area),
        ViewMode::Info => draw_info_overlay(f, app, area),
        _ => {}
    }
}

fn draw_title(f: &mut Frame, area: Rect) {
    let title = Paragraph::new(ASCII_TITLE)
        .style(styles::header_style())
        .alignment(Alignment::Center);
    f.render_widget(title, area);
}

fn draw_search_bar(f: &mut Frame, app: &App, area: Rect) {
    let border_style = if app.search_mode {
        styles::border_focused_style()
    } else {
        styles::border_unfocused_style()
    };

    let label = if app.search_mode {
        "Search (Esc to unfocus): "
    } else {
        "Search (/ to focus): "
    };

    let content = format!("{}{}", label, app.search_query);
    let cursor_suffix = if app.search_mode { "_" } else { "" };
    let display = format!("{content}{cursor_suffix}");

    let search = Paragraph::new(display)
        .style(Style::default().fg(styles::FG).bg(styles::BG))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(border_style)
                .style(Style::default().bg(styles::BG)),
        );
    f.render_widget(search, area);
}

fn draw_table(f: &mut Frame, app: &App, area: Rect) {
    let border_style = if !app.search_mode {
        styles::border_focused_style()
    } else {
        styles::border_unfocused_style()
    };

    // Calculate column widths dynamically
    let available_width = if area.width > 4 { area.width - 4 } else { area.width } as usize;

    // Column proportions: Status(3) + Name(flex) + User(flex) + Hostname(flex) + Port(6) + Tags(flex)
    let status_width: u16 = 3;
    let port_width: u16 = 6;
    let fixed = (status_width + port_width) as usize;

    let remaining = if available_width > fixed {
        available_width - fixed
    } else {
        40
    };

    // Distribute remaining: Name 25%, User 15%, Hostname 30%, Tags 30%
    let name_width = ((remaining * 25) / 100).max(8) as u16;
    let user_width = ((remaining * 15) / 100).max(6) as u16;
    let hostname_width = ((remaining * 30) / 100).max(10) as u16;
    let tags_width = ((remaining * 30) / 100).max(6) as u16;

    // Build header with sort indicator
    let name_title = match app.sort_mode {
        SortMode::ByName => "Name \u{2193}",
        _ => "Name",
    };

    let header_cells = [
        "St",
        name_title,
        "User",
        "Hostname",
        "Port",
        "Tags",
    ];
    let header = Row::new(header_cells)
        .style(styles::table_header_style())
        .height(1);

    // Build rows
    let visible = app.visible_rows();
    let end = (app.table_offset + visible).min(app.filtered_hosts.len());
    let visible_hosts = &app.filtered_hosts[app.table_offset..end];

    let rows: Vec<Row> = visible_hosts
        .iter()
        .enumerate()
        .map(|(i, host)| {
            let abs_idx = app.table_offset + i;
            let is_selected = abs_idx == app.selected;

            let (indicator, status) = app.get_status_indicator(&host.name);
            let status_style = match status {
                HostStatus::Online(_) => styles::status_online_style(),
                HostStatus::Offline(_) => styles::status_offline_style(),
                HostStatus::Connecting => styles::status_connecting_style(),
                HostStatus::Unknown => styles::status_unknown_style(),
            };

            let tags_str = if host.tags.is_empty() {
                String::new()
            } else {
                host.tags.iter().map(|t| format!("#{t}")).collect::<Vec<_>>().join(" ")
            };

            let port_display = if host.port.is_empty() {
                "22".to_string()
            } else {
                host.port.clone()
            };

            let cells = vec![
                Span::styled(indicator.to_string(), status_style),
                Span::styled(host.name.clone(), Style::default().fg(styles::FG)),
                Span::styled(host.user.clone(), Style::default().fg(styles::FG)),
                Span::styled(host.hostname.clone(), Style::default().fg(styles::FG)),
                Span::styled(port_display, Style::default().fg(styles::FG)),
                Span::styled(tags_str, Style::default().fg(styles::MUTED)),
            ];

            let row = Row::new(cells.into_iter().map(|s| Line::from(s)));
            if is_selected {
                row.style(styles::table_selected_style())
            } else {
                row.style(styles::table_row_style())
            }
        })
        .collect();

    let widths = [
        Constraint::Length(status_width),
        Constraint::Length(name_width),
        Constraint::Length(user_width),
        Constraint::Length(hostname_width),
        Constraint::Length(port_width),
        Constraint::Length(tags_width),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(border_style)
                .style(Style::default().bg(styles::BG)),
        )
        .row_highlight_style(styles::table_selected_style());

    f.render_widget(table, area);
}

fn draw_status_bar(f: &mut Frame, app: &App, area: Rect) {
    let help_text = if app.search_mode {
        " Type to filter | Enter: validate | Tab: switch | Esc: quit search"
    } else {
        " j/k: navigate | Enter: connect | /: search | s: sort | i: info | ?: help | q: quit"
    };

    let count = app.filtered_hosts.len();
    let total = app.hosts.len();
    let sort_label = app.sort_mode.label();
    let right = format!(" [{count}/{total}] Sort: {sort_label} ");

    // Calculate padding
    let left_len = help_text.len();
    let right_len = right.len();
    let total_len = area.width as usize;
    let pad = if total_len > left_len + right_len {
        total_len - left_len - right_len
    } else {
        1
    };

    let bar = Line::from(vec![
        Span::styled(help_text, styles::help_text_style()),
        Span::raw(" ".repeat(pad)),
        Span::styled(right, Style::default().fg(styles::PRIMARY)),
    ]);

    let paragraph = Paragraph::new(bar).style(Style::default().bg(styles::BG));
    f.render_widget(paragraph, area);
}

fn draw_delete_confirm(f: &mut Frame, app: &App, area: Rect) {
    let host_name = app.delete_target.as_deref().unwrap_or("???");

    let popup_width = 50u16.min(area.width.saturating_sub(4));
    let popup_height = 7u16;
    let x = (area.width.saturating_sub(popup_width)) / 2;
    let y = (area.height.saturating_sub(popup_height)) / 2;
    let popup_area = Rect::new(x, y, popup_width, popup_height);

    f.render_widget(Clear, popup_area);

    let lines = vec![
        Line::from(Span::styled(
            "DELETE SSH HOST",
            styles::delete_title_style(),
        )),
        Line::from(""),
        Line::from(format!("Delete host '{host_name}'?")),
        Line::from(""),
        Line::from(Span::styled(
            "This action cannot be undone.",
            styles::delete_warning_style(),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "Enter/y: confirm | Esc/n: cancel",
            styles::help_text_style(),
        )),
    ];

    let paragraph = Paragraph::new(lines)
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(styles::RED))
                .style(Style::default().bg(styles::BG).fg(styles::FG)),
        );
    f.render_widget(paragraph, popup_area);
}

fn draw_info_overlay(f: &mut Frame, app: &App, area: Rect) {
    let host = match app.selected_host() {
        Some(h) => h,
        None => return,
    };

    let popup_width = 60u16.min(area.width.saturating_sub(4));
    let popup_height = 16u16.min(area.height.saturating_sub(4));
    let x = (area.width.saturating_sub(popup_width)) / 2;
    let y = (area.height.saturating_sub(popup_height)) / 2;
    let popup_area = Rect::new(x, y, popup_width, popup_height);

    f.render_widget(Clear, popup_area);

    let tags_str = if host.tags.is_empty() {
        "(none)".to_string()
    } else {
        host.tags.iter().map(|t| format!("#{t}")).collect::<Vec<_>>().join(" ")
    };

    let last_login = app.format_time_ago(&host.name);
    let last_login_display = if last_login.is_empty() {
        "Never".to_string()
    } else {
        last_login
    };

    let (indicator, _) = app.get_status_indicator(&host.name);
    let port_display = if host.port.is_empty() { "22" } else { &host.port };

    let lines = vec![
        Line::from(Span::styled(
            format!(" Host: {} ", host.name),
            Style::default()
                .fg(styles::PRIMARY)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Hostname:  ", Style::default().fg(styles::MUTED)),
            Span::styled(&host.hostname, Style::default().fg(styles::FG)),
        ]),
        Line::from(vec![
            Span::styled("  User:      ", Style::default().fg(styles::MUTED)),
            Span::styled(&host.user, Style::default().fg(styles::FG)),
        ]),
        Line::from(vec![
            Span::styled("  Port:      ", Style::default().fg(styles::MUTED)),
            Span::styled(port_display, Style::default().fg(styles::FG)),
        ]),
        Line::from(vec![
            Span::styled("  Identity:  ", Style::default().fg(styles::MUTED)),
            Span::styled(&host.identity, Style::default().fg(styles::FG)),
        ]),
        Line::from(vec![
            Span::styled("  ProxyJump: ", Style::default().fg(styles::MUTED)),
            Span::styled(&host.proxy_jump, Style::default().fg(styles::FG)),
        ]),
        Line::from(vec![
            Span::styled("  Tags:      ", Style::default().fg(styles::MUTED)),
            Span::styled(tags_str, Style::default().fg(styles::FG)),
        ]),
        Line::from(vec![
            Span::styled("  Status:    ", Style::default().fg(styles::MUTED)),
            Span::styled(indicator, Style::default().fg(styles::FG)),
        ]),
        Line::from(vec![
            Span::styled("  Last used: ", Style::default().fg(styles::MUTED)),
            Span::styled(last_login_display, Style::default().fg(styles::FG)),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  Press Esc or Enter to close",
            styles::help_text_style(),
        )),
    ];

    let paragraph = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(styles::border_focused_style())
            .style(Style::default().bg(styles::BG).fg(styles::FG)),
    );
    f.render_widget(paragraph, popup_area);
}
