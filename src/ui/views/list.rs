use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph, Row, Scrollbar, ScrollbarOrientation, ScrollbarState, Table};
use ratatui::Frame;

use crate::connectivity::HostStatus;
use crate::ui::app::{App, DisplayRow, SortMode, ViewMode};
use crate::ui::styles;

const ASCII_TITLE: &str = r#"
  /$$$$$$   /$$$$$$  /$$   /$$ /$$      /$$         /$$$$$$$   /$$$$$$
 /$$__  $$ /$$__  $$| $$  | $$| $$$    /$$$        | $$__  $$ /$$__  $$
| $$  \__/| $$  \__/| $$  | $$| $$$$  /$$$$        | $$  \ $$| $$  \__/
|  $$$$$$ |  $$$$$$ | $$$$$$$$| $$ $$/$$ $$ /$$$$$$| $$$$$$$/|  $$$$$$
 \____  $$ \____  $$| $$__  $$| $$  $$$| $$|______/| $$__  $$ \____  $$
 /$$  \ $$ /$$  \ $$| $$  | $$| $$\  $ | $$        | $$  \ $$ /$$  \ $$
|  $$$$$$/|  $$$$$$/| $$  | $$| $$ \/  | $$        | $$  | $$|  $$$$$$/
 \______/  \______/ |__/  |__/|__/     |__/        |__/  |__/ \______/
"#;

pub fn draw(f: &mut Frame, app: &App) {
    let area = f.area();

    // Clear background with theme bg
    let bg_block = Block::default().style(Style::default().bg(styles::bg()));
    f.render_widget(bg_block, area);

    // Responsive layout: compact title when terminal is small
    let compact = area.height < 20;
    let title_height = if compact { 1 } else { 9 };

    let draw_content = |f: &mut Frame, content_area: Rect| {
        let chunks = Layout::vertical([
            Constraint::Length(title_height),
            Constraint::Length(3),
            Constraint::Min(3),
            Constraint::Length(1),
        ])
        .split(content_area);

        if compact {
            draw_compact_title(f, chunks[0]);
        } else {
            draw_title(f, chunks[0]);
        }
        draw_search_bar(f, app, chunks[1]);
        draw_table(f, app, chunks[2]);
        draw_status_bar(f, app, chunks[3]);
    };

    if app.show_sidebar {
        let main_chunks = Layout::horizontal([
            Constraint::Length(20),
            Constraint::Min(40),
        ])
        .split(area);

        draw_sidebar(f, app, main_chunks[0]);
        draw_content(f, main_chunks[1]);
    } else {
        draw_content(f, area);
    }

    // Overlay views
    match app.view_mode {
        ViewMode::DeleteConfirm => draw_delete_confirm(f, app, area),
        ViewMode::Info => draw_info_overlay(f, app, area),
        ViewMode::Add => draw_host_form(f, app, area, " ADD SSH HOST "),
        ViewMode::Edit => draw_host_form(f, app, area, " EDIT SSH HOST "),
        ViewMode::Password => draw_password_overlay(f, app, area),
        ViewMode::PortForward => draw_port_forward_overlay(f, app, area),
        ViewMode::Broadcast => draw_broadcast_overlay(f, app, area),
        ViewMode::Snippets => draw_snippets_overlay(f, app, area),
        ViewMode::FileTransfer => draw_file_transfer_overlay(f, app, area),
        ViewMode::GroupCreate => draw_group_create_overlay(f, app, area),
        ViewMode::GroupPicker => draw_group_picker_overlay(f, app, area),
        _ => {}
    }
}

fn draw_title(f: &mut Frame, area: Rect) {
    if area.width < 75 {
        draw_compact_title(f, area);
        return;
    }

    let lines: Vec<Line> = ASCII_TITLE
        .lines()
        .map(|line| {
            Line::from(Span::styled(
                line.to_string(),
                Style::default().fg(styles::primary()),
            ))
        })
        .collect();

    let title = Paragraph::new(lines).alignment(Alignment::Center);
    f.render_widget(title, area);
}

fn draw_compact_title(f: &mut Frame, area: Rect) {
    let title = Paragraph::new(Line::from(Span::styled(
        "sshm-rs",
        Style::default()
            .fg(styles::primary())
            .add_modifier(Modifier::BOLD),
    )))
    .alignment(Alignment::Center);
    f.render_widget(title, area);
}

fn draw_sidebar(f: &mut Frame, app: &App, area: Rect) {
    let border_style = if app.sidebar_focused {
        styles::border_focused_style()
    } else {
        styles::border_unfocused_style()
    };

    let all_hosts_style = if app.sidebar_active_tag.is_none() {
        Style::default().fg(styles::primary()).add_modifier(Modifier::BOLD)
    } else if app.sidebar_focused && app.sidebar_selected == 0 {
        Style::default().fg(styles::fg()).bg(styles::selection_bg())
    } else {
        Style::default().fg(styles::fg())
    };

    let mut items: Vec<Line> = vec![
        Line::from(Span::styled("  All Hosts", all_hosts_style)),
    ];

    for (i, tag) in app.sidebar_tags.iter().enumerate() {
        let sidebar_index = i + 1;
        let is_selected = app.sidebar_focused && sidebar_index == app.sidebar_selected;
        let is_active = app.sidebar_active_tag.as_deref() == Some(tag);

        let style = if is_active {
            Style::default().fg(styles::primary()).add_modifier(Modifier::BOLD)
        } else if is_selected {
            Style::default().fg(styles::fg()).bg(styles::selection_bg())
        } else {
            Style::default().fg(styles::purple())
        };

        let prefix = if is_active { "\u{25cf} " } else { "  " };
        items.push(Line::from(Span::styled(format!("{prefix}#{tag}"), style)));
    }

    let list = Paragraph::new(items)
        .block(
            Block::default()
                .title(" Tags ")
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(border_style)
                .style(Style::default().bg(styles::bg())),
        );
    f.render_widget(list, area);
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
        .style(Style::default().fg(styles::fg()).bg(styles::bg()))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(border_style)
                .style(Style::default().bg(styles::bg())),
        );
    f.render_widget(search, area);
}

fn draw_table(f: &mut Frame, app: &App, area: Rect) {
    let border_style = if !app.search_mode && !app.sidebar_focused {
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
    let display_source: &[DisplayRow] = &app.display_rows;
    let total_rows = if display_source.is_empty() {
        app.filtered_hosts.len()
    } else {
        display_source.len()
    };
    let end = (app.table_offset + visible).min(total_rows);
    let visible_range = app.table_offset..end;

    let rows: Vec<Row> = visible_range
        .map(|abs_idx| {
            let is_cursor = abs_idx == app.selected;

            // Determine what to render for this row
            let display_row = display_source.get(abs_idx).cloned()
                .unwrap_or(DisplayRow::HostRow(abs_idx));

            match display_row {
                DisplayRow::GroupHeader { name, host_count, collapsed } => {
                    let collapse_icon = if collapsed { "\u{25b8}" } else { "\u{25be}" };
                    let header_text = format!("{} {} ({})", collapse_icon, name, host_count);
                    let header_style = if is_cursor {
                        Style::default()
                            .fg(styles::primary())
                            .bg(styles::selection_bg())
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default()
                            .fg(styles::primary())
                            .add_modifier(Modifier::BOLD)
                    };
                    let cells = vec![
                        Span::raw(""),
                        Span::styled(header_text, header_style),
                        Span::raw(""),
                        Span::raw(""),
                        Span::raw(""),
                        Span::raw(""),
                    ];
                    Row::new(cells).style(Style::default().bg(styles::bg()))
                }
                DisplayRow::HostRow(host_idx) => {
                    let host = match app.filtered_hosts.get(host_idx) {
                        Some(h) => h,
                        None => return Row::new(vec![Span::raw(""); 6]),
                    };

                    let is_multi_selected = app.selected_hosts.contains(&host.name);

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

                    let status_span = if is_multi_selected {
                        Span::styled("\u{2713} ", Style::default().fg(styles::cyan()))
                    } else {
                        Span::styled(indicator.to_string(), status_style)
                    };

                    let name_display = if app.favorites.is_favorite(&host.name) {
                        format!("\u{2605} {}", host.name)
                    } else {
                        host.name.clone()
                    };

                    let name_style = if is_multi_selected {
                        Style::default().fg(styles::cyan())
                    } else if app.favorites.is_favorite(&host.name) {
                        Style::default().fg(styles::yellow())
                    } else {
                        Style::default().fg(styles::fg())
                    };

                    let cells = vec![
                        status_span,
                        Span::styled(name_display, name_style),
                        Span::styled(host.user.clone(), Style::default().fg(styles::fg())),
                        Span::styled(host.hostname.clone(), Style::default().fg(styles::cyan())),
                        Span::styled(port_display, Style::default().fg(styles::fg())),
                        Span::styled(tags_str, Style::default().fg(styles::purple())),
                    ];

                    let row = Row::new(cells);
                    if is_cursor {
                        row.style(styles::table_selected_style())
                    } else if is_multi_selected {
                        row.style(styles::multi_selected_style())
                    } else {
                        row.style(styles::table_row_style())
                    }
                }
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
                .border_type(BorderType::Rounded)
                .border_style(border_style)
                .style(Style::default().bg(styles::bg())),
        )
        .row_highlight_style(styles::table_selected_style());

    f.render_widget(table, area);

    // Render scrollbar when there are more rows than visible
    let visible = app.visible_rows();
    if total_rows > visible {
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("\u{25b2}"))
            .end_symbol(Some("\u{25bc}"));
        let mut scrollbar_state = ScrollbarState::new(total_rows)
            .position(app.selected);
        f.render_stateful_widget(scrollbar, area, &mut scrollbar_state);
    }
}

fn draw_status_bar(f: &mut Frame, app: &App, area: Rect) {
    // Check if there's an active (non-expired) toast message
    let show_toast = app.toast_message.is_some()
        && app
            .toast_expires
            .map(|exp| std::time::Instant::now() < exp)
            .unwrap_or(false);

    let (left_text, left_style) = if show_toast {
        let msg = format!(" {} ", app.toast_message.as_deref().unwrap_or(""));
        (msg, Style::default().fg(styles::green()))
    } else if app.search_mode {
        (
            " Type to filter | Enter: validate | Tab: switch | Esc: close search".to_string(),
            styles::help_text_style(),
        )
    } else if app.has_selection() {
        let count = app.selected_hosts.len();
        (
            format!(" {count} selected | Space: toggle | b: broadcast | d: delete | Ctrl+a: all | Esc: clear"),
            Style::default().fg(styles::cyan()),
        )
    } else {
        (
            " ? Help".to_string(),
            styles::help_text_style(),
        )
    };

    let count = app.filtered_hosts.len();
    let total = app.hosts.len();
    let sort_label = app.sort_mode.label();
    let right = format!(" [{count}/{total}] Sort: {sort_label} ");

    // Build optional warning indicator
    let warn_count = app.config_warnings.len();
    let warn_text = if warn_count > 0 {
        format!(" \u{26a0} {} ", warn_count)
    } else {
        String::new()
    };

    // Calculate padding
    let left_len = left_text.len();
    let right_len = right.len() + warn_text.len();
    let total_len = area.width as usize;
    let pad = if total_len > left_len + right_len {
        total_len - left_len - right_len
    } else {
        1
    };

    let mut spans = vec![
        Span::styled(left_text, left_style),
        Span::raw(" ".repeat(pad)),
    ];
    if warn_count > 0 {
        spans.push(Span::styled(
            warn_text,
            Style::default()
                .fg(styles::yellow())
                .add_modifier(Modifier::BOLD),
        ));
    }
    spans.push(Span::styled(right, Style::default().fg(styles::primary())));

    let bar = Line::from(spans);
    let paragraph = Paragraph::new(bar).style(Style::default().bg(styles::bg()));
    f.render_widget(paragraph, area);
}

fn draw_delete_confirm(f: &mut Frame, app: &App, area: Rect) {
    let target = app.delete_target.as_deref().unwrap_or("???");
    let is_batch = target.starts_with("__batch__:");

    let confirm_line = if is_batch {
        let count = app.selected_hosts.len();
        format!("Delete {count} selected host{}?", if count == 1 { "" } else { "s" })
    } else {
        format!("Delete host '{target}'?")
    };

    let popup_width = 54u16.min(area.width.saturating_sub(4));
    let popup_height = 7u16;
    let x = (area.width.saturating_sub(popup_width)) / 2;
    let y = (area.height.saturating_sub(popup_height)) / 2;
    let popup_area = Rect::new(x, y, popup_width, popup_height);

    f.render_widget(Clear, popup_area);

    let title = if is_batch { "DELETE SELECTED HOSTS" } else { "DELETE SSH HOST" };

    let lines = vec![
        Line::from(Span::styled(
            title,
            styles::delete_title_style(),
        )),
        Line::from(""),
        Line::from(confirm_line),
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
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(styles::red()))
                .style(Style::default().bg(styles::bg()).fg(styles::fg())),
        );
    f.render_widget(paragraph, popup_area);
}

fn draw_info_overlay(f: &mut Frame, app: &App, area: Rect) {
    let host = match app.selected_host() {
        Some(h) => h,
        None => return,
    };

    let last_login = app.format_time_ago(&host.name);
    let last_login_display = if last_login.is_empty() {
        "Never".to_string()
    } else {
        last_login
    };

    let (indicator, _) = app.get_status_indicator(&host.name);
    let port_display = if host.port.is_empty() { "22" } else { &host.port };

    let mut lines = vec![
        Line::from(Span::styled(
            format!(" Host: {} ", host.name),
            Style::default()
                .fg(styles::primary())
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
    ];

    // Always show hostname
    lines.push(Line::from(vec![
        Span::styled("  Hostname:  ", Style::default().fg(styles::muted())),
        Span::styled(&host.hostname, Style::default().fg(styles::fg())),
    ]));

    // Only show user if non-empty
    if !host.user.is_empty() {
        lines.push(Line::from(vec![
            Span::styled("  User:      ", Style::default().fg(styles::muted())),
            Span::styled(&host.user, Style::default().fg(styles::fg())),
        ]));
    }

    // Always show port (defaults to 22)
    lines.push(Line::from(vec![
        Span::styled("  Port:      ", Style::default().fg(styles::muted())),
        Span::styled(port_display, Style::default().fg(styles::fg())),
    ]));

    // Only show identity if non-empty
    if !host.identity.is_empty() {
        lines.push(Line::from(vec![
            Span::styled("  Identity:  ", Style::default().fg(styles::muted())),
            Span::styled(&host.identity, Style::default().fg(styles::fg())),
        ]));
    }

    // Only show proxy_jump if non-empty
    if !host.proxy_jump.is_empty() {
        lines.push(Line::from(vec![
            Span::styled("  ProxyJump: ", Style::default().fg(styles::muted())),
            Span::styled(&host.proxy_jump, Style::default().fg(styles::fg())),
        ]));
    }

    // Only show tags if non-empty
    if !host.tags.is_empty() {
        let tags_str = host.tags.iter().map(|t| format!("#{t}")).collect::<Vec<_>>().join(" ");
        lines.push(Line::from(vec![
            Span::styled("  Tags:      ", Style::default().fg(styles::muted())),
            Span::styled(tags_str, Style::default().fg(styles::purple())),
        ]));
    }

    lines.push(Line::from(vec![
        Span::styled("  Password:  ", Style::default().fg(styles::muted())),
        Span::styled(
            if crate::credentials::has_password(&host.name) { "Saved" } else { "None" },
            Style::default().fg(if crate::credentials::has_password(&host.name) { styles::green() } else { styles::muted() }),
        ),
    ]));

    lines.push(Line::from(vec![
        Span::styled("  Status:    ", Style::default().fg(styles::muted())),
        Span::styled(indicator, Style::default().fg(styles::fg())),
    ]));

    lines.push(Line::from(vec![
        Span::styled("  Last used: ", Style::default().fg(styles::muted())),
        Span::styled(last_login_display, Style::default().fg(styles::fg())),
    ]));

    // Collect warnings that concern this host
    let host_warnings: Vec<&String> = app
        .config_warnings
        .iter()
        .filter(|w| {
            w.contains(&format!("host name: '{}'", host.name))
                || w.starts_with(&format!("Host '{}': ", host.name))
        })
        .collect();

    if !host_warnings.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  Warnings:",
            Style::default()
                .fg(styles::yellow())
                .add_modifier(Modifier::BOLD),
        )));
        for w in host_warnings {
            lines.push(Line::from(Span::styled(
                format!("  \u{26a0} {}", w),
                Style::default().fg(styles::red()),
            )));
        }
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  Press Esc or Enter to close",
        styles::help_text_style(),
    )));

    // Dynamic height based on content
    let popup_height = (lines.len() as u16 + 2).min(area.height.saturating_sub(4));
    let popup_width = 60u16.min(area.width.saturating_sub(4));
    let x = (area.width.saturating_sub(popup_width)) / 2;
    let y = (area.height.saturating_sub(popup_height)) / 2;
    let popup_area = Rect::new(x, y, popup_width, popup_height);

    f.render_widget(Clear, popup_area);

    let paragraph = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(styles::border_focused_style())
            .style(Style::default().bg(styles::bg()).fg(styles::fg())),
    );
    f.render_widget(paragraph, popup_area);
}

fn draw_host_form(f: &mut Frame, app: &App, area: Rect, title: &str) {
    use crate::ui::app::AddField;

    let popup_width = 60u16.min(area.width.saturating_sub(4));
    let popup_height = 18u16.min(area.height.saturating_sub(4));
    let x = (area.width.saturating_sub(popup_width)) / 2;
    let y = (area.height.saturating_sub(popup_height)) / 2;
    let popup_area = Rect::new(x, y, popup_width, popup_height);

    f.render_widget(Clear, popup_area);

    let mut lines = vec![
        Line::from(Span::styled(
            title,
            Style::default()
                .fg(styles::bg())
                .bg(styles::primary())
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
    ];

    for field in AddField::ALL {
        let idx = field as usize;
        let is_focused = app.add_focused == field;
        let label = format!("  {:12} ", field.label());
        let value = &app.add_fields[idx];

        // Mask password field
        let display_value = if field.is_secret() && !value.is_empty() {
            "*".repeat(value.len())
        } else {
            value.clone()
        };
        let cursor = if is_focused { "_" } else { "" };

        let label_style = if is_focused {
            Style::default().fg(styles::primary()).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(styles::muted())
        };

        let value_style = Style::default().fg(styles::fg());
        let indicator = if is_focused { "> " } else { "  " };

        // Show lock icon for password field if it has a value
        let suffix = if field.is_secret() && !value.is_empty() && !is_focused {
            " [saved]"
        } else {
            ""
        };

        lines.push(Line::from(vec![
            Span::styled(indicator, Style::default().fg(styles::primary())),
            Span::styled(label, label_style),
            Span::styled(format!("{display_value}{cursor}"), value_style),
            Span::styled(suffix, Style::default().fg(styles::green())),
        ]));
    }

    lines.push(Line::from(""));

    if let Some(ref err) = app.add_error {
        lines.push(Line::from(Span::styled(
            format!("  Error: {err}"),
            Style::default().fg(styles::red()),
        )));
        lines.push(Line::from(""));
    }

    lines.push(Line::from(Span::styled(
        "  Tab/Arrows: navigate | Enter: save | Esc: cancel",
        styles::help_text_style(),
    )));

    let paragraph = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(styles::border_focused_style())
            .style(Style::default().bg(styles::bg()).fg(styles::fg())),
    );
    f.render_widget(paragraph, popup_area);
}

fn draw_file_transfer_overlay(f: &mut Frame, app: &App, area: Rect) {
    let host_name = app.scp_target.as_deref().unwrap_or("???");

    let popup_width = 60u16.min(area.width.saturating_sub(4));
    let popup_height = 13u16.min(area.height.saturating_sub(4));
    let x = (area.width.saturating_sub(popup_width)) / 2;
    let y = (area.height.saturating_sub(popup_height)) / 2;
    let popup_area = Rect::new(x, y, popup_width, popup_height);

    f.render_widget(Clear, popup_area);

    // Direction toggle
    let (upload_style, download_style) = if app.scp_upload {
        (
            Style::default().fg(styles::bg()).bg(styles::primary()).add_modifier(Modifier::BOLD),
            Style::default().fg(styles::muted()),
        )
    } else {
        (
            Style::default().fg(styles::muted()),
            Style::default().fg(styles::bg()).bg(styles::primary()).add_modifier(Modifier::BOLD),
        )
    };

    let direction_focused = app.scp_focused == 0;
    let local_focused = app.scp_focused == 1;
    let remote_focused = app.scp_focused == 2;

    let focus_indicator = |focused: bool| -> &'static str {
        if focused { "> " } else { "  " }
    };

    let field_label_style = |focused: bool| -> Style {
        if focused {
            Style::default().fg(styles::primary()).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(styles::muted())
        }
    };

    let local_cursor = if local_focused { "_" } else { "" };
    let remote_cursor = if remote_focused { "_" } else { "" };

    let mut lines = vec![
        Line::from(Span::styled(
            " FILE TRANSFER (SCP) ",
            Style::default()
                .fg(styles::bg())
                .bg(styles::primary())
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Host:       ", Style::default().fg(styles::muted())),
            Span::styled(host_name, Style::default().fg(styles::fg()).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(focus_indicator(direction_focused), Style::default().fg(styles::primary())),
            Span::styled("  Direction:  ", field_label_style(direction_focused)),
            Span::styled(" Upload ", upload_style),
            Span::raw("  "),
            Span::styled(" Download ", download_style),
            if direction_focused {
                Span::styled("  \u{2190}/\u{2192} to toggle", Style::default().fg(styles::muted()))
            } else {
                Span::raw("")
            },
        ]),
        Line::from(vec![
            Span::styled(focus_indicator(local_focused), Style::default().fg(styles::primary())),
            Span::styled("  Local path: ", field_label_style(local_focused)),
            Span::styled(
                format!("{}{}", app.scp_local_path, local_cursor),
                Style::default().fg(styles::fg()),
            ),
        ]),
        Line::from(vec![
            Span::styled(focus_indicator(remote_focused), Style::default().fg(styles::primary())),
            Span::styled("  Remote path:", field_label_style(remote_focused)),
            Span::styled(
                format!(" {}{}", app.scp_remote_path, remote_cursor),
                Style::default().fg(styles::fg()),
            ),
        ]),
        Line::from(""),
    ];

    if let Some(ref err) = app.scp_error {
        lines.push(Line::from(Span::styled(
            format!("  Error: {err}"),
            Style::default().fg(styles::red()),
        )));
        lines.push(Line::from(""));
    }

    lines.push(Line::from(Span::styled(
        "  Tab: navigate | Enter: transfer | Esc: cancel",
        styles::help_text_style(),
    )));

    let paragraph = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(styles::border_focused_style())
            .style(Style::default().bg(styles::bg()).fg(styles::fg())),
    );
    f.render_widget(paragraph, popup_area);
}

fn draw_password_overlay(f: &mut Frame, app: &App, area: Rect) {
    let host_name = app.password_target.as_deref().unwrap_or("???");
    let has_existing = crate::credentials::has_password(host_name);

    let popup_width = 50u16.min(area.width.saturating_sub(4));
    let popup_height = 9u16;
    let x = (area.width.saturating_sub(popup_width)) / 2;
    let y = (area.height.saturating_sub(popup_height)) / 2;
    let popup_area = Rect::new(x, y, popup_width, popup_height);

    f.render_widget(Clear, popup_area);

    let masked_input = if app.password_input.is_empty() {
        String::new()
    } else {
        "*".repeat(app.password_input.len())
    };

    let mut lines = vec![
        Line::from(Span::styled(
            " SET PASSWORD ",
            Style::default()
                .fg(styles::bg())
                .bg(styles::primary())
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Host: ", Style::default().fg(styles::muted())),
            Span::styled(host_name, Style::default().fg(styles::fg()).add_modifier(Modifier::BOLD)),
        ]),
    ];

    if has_existing {
        lines.push(Line::from(vec![
            Span::styled("  Password: ", Style::default().fg(styles::muted())),
            Span::styled("[Saved]", Style::default().fg(styles::green())),
            Span::styled(" — Enter new or Del to remove", Style::default().fg(styles::muted())),
        ]));
    }

    lines.push(Line::from(vec![
        Span::styled("  New:   ", Style::default().fg(styles::muted())),
        Span::styled(format!("{masked_input}_"), Style::default().fg(styles::fg())),
    ]));

    lines.push(Line::from(""));

    let help = if has_existing {
        "  Enter: save | Del: remove | Esc: cancel"
    } else {
        "  Enter: save | Esc: cancel"
    };
    lines.push(Line::from(Span::styled(help, styles::help_text_style())));

    let paragraph = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(styles::border_focused_style())
            .style(Style::default().bg(styles::bg()).fg(styles::fg())),
    );
    f.render_widget(paragraph, popup_area);
}

fn draw_port_forward_overlay(f: &mut Frame, app: &App, area: Rect) {
    let host_name = app.pf_target.as_deref().unwrap_or("???");
    let is_dynamic = app.pf_forward_type == 2;

    let popup_width = 60u16.min(area.width.saturating_sub(4));
    let popup_height = 16u16.min(area.height.saturating_sub(4));
    let x = (area.width.saturating_sub(popup_width)) / 2;
    let y = (area.height.saturating_sub(popup_height)) / 2;
    let popup_area = Rect::new(x, y, popup_width, popup_height);

    f.render_widget(Clear, popup_area);

    let mut lines = vec![
        Line::from(Span::styled(
            " PORT FORWARDING ",
            Style::default()
                .fg(styles::bg())
                .bg(styles::primary())
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Host: ", Style::default().fg(styles::muted())),
            Span::styled(host_name, Style::default().fg(styles::fg()).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(""),
    ];

    // Field labels and focused state
    let fields = ["Type", "Local Port", "Remote Host", "Remote Port", "Bind Address"];

    for (i, &label) in fields.iter().enumerate() {
        let is_focused = app.pf_focused == i;
        let indicator = if is_focused { "> " } else { "  " };

        let label_style = if is_focused {
            Style::default().fg(styles::primary()).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(styles::muted())
        };

        // Dim remote host/port fields when dynamic mode is selected
        let is_disabled = is_dynamic && (i == 2 || i == 3);

        match i {
            0 => {
                // Forward type selector
                let types = ["Local", "Remote", "Dynamic"];
                let mut type_spans = vec![
                    Span::styled(indicator, Style::default().fg(styles::primary())),
                    Span::styled(format!("{:14} ", label), label_style),
                ];
                for (ti, &type_label) in types.iter().enumerate() {
                    if ti == app.pf_forward_type {
                        type_spans.push(Span::styled(
                            format!("[{type_label}]"),
                            Style::default().fg(styles::green()).add_modifier(Modifier::BOLD),
                        ));
                    } else {
                        type_spans.push(Span::styled(
                            format!(" {type_label} "),
                            Style::default().fg(styles::muted()),
                        ));
                    }
                    if ti < 2 {
                        type_spans.push(Span::styled(" ", Style::default()));
                    }
                }
                lines.push(Line::from(type_spans));
            }
            1 => {
                let cursor = if is_focused { "_" } else { "" };
                lines.push(Line::from(vec![
                    Span::styled(indicator, Style::default().fg(styles::primary())),
                    Span::styled(format!("{:14} ", label), label_style),
                    Span::styled(format!("{}{cursor}", app.pf_local_port), Style::default().fg(styles::fg())),
                ]));
            }
            2 => {
                let cursor = if is_focused && !is_disabled { "_" } else { "" };
                let value_style = if is_disabled { Style::default().fg(styles::muted()) } else { Style::default().fg(styles::fg()) };
                let suffix = if is_disabled { " (N/A)" } else { "" };
                lines.push(Line::from(vec![
                    Span::styled(indicator, Style::default().fg(styles::primary())),
                    Span::styled(format!("{:14} ", label), label_style),
                    Span::styled(format!("{}{cursor}", app.pf_remote_host), value_style),
                    Span::styled(suffix, Style::default().fg(styles::muted())),
                ]));
            }
            3 => {
                let cursor = if is_focused && !is_disabled { "_" } else { "" };
                let value_style = if is_disabled { Style::default().fg(styles::muted()) } else { Style::default().fg(styles::fg()) };
                let suffix = if is_disabled { " (N/A)" } else { "" };
                lines.push(Line::from(vec![
                    Span::styled(indicator, Style::default().fg(styles::primary())),
                    Span::styled(format!("{:14} ", label), label_style),
                    Span::styled(format!("{}{cursor}", app.pf_remote_port), value_style),
                    Span::styled(suffix, Style::default().fg(styles::muted())),
                ]));
            }
            4 => {
                let cursor = if is_focused { "_" } else { "" };
                let display = if app.pf_bind_address.is_empty() && !is_focused {
                    "0.0.0.0".to_string()
                } else {
                    format!("{}{cursor}", app.pf_bind_address)
                };
                let value_style = if app.pf_bind_address.is_empty() && !is_focused {
                    Style::default().fg(styles::muted())
                } else {
                    Style::default().fg(styles::fg())
                };
                lines.push(Line::from(vec![
                    Span::styled(indicator, Style::default().fg(styles::primary())),
                    Span::styled(format!("{:14} ", label), label_style),
                    Span::styled(display, value_style),
                ]));
            }
            _ => {}
        }
    }

    lines.push(Line::from(""));

    if let Some(ref err) = app.pf_error {
        lines.push(Line::from(Span::styled(
            format!("  Error: {err}"),
            Style::default().fg(styles::red()),
        )));
        lines.push(Line::from(""));
    }

    lines.push(Line::from(Span::styled(
        "  Tab: navigate | Left/Right: type | Enter: connect | Esc: cancel",
        styles::help_text_style(),
    )));

    let paragraph = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(styles::border_focused_style())
            .style(Style::default().bg(styles::bg()).fg(styles::fg())),
    );
    f.render_widget(paragraph, popup_area);
}

fn draw_broadcast_overlay(f: &mut Frame, app: &App, area: Rect) {
    let selected_count = app.selected_hosts.len();
    let host_lines = selected_count.min(8) as u16;
    let base_height = 7u16 + host_lines;
    let popup_height = base_height.min(area.height.saturating_sub(4));
    let popup_width = 60u16.min(area.width.saturating_sub(4));
    let x = (area.width.saturating_sub(popup_width)) / 2;
    let y = (area.height.saturating_sub(popup_height)) / 2;
    let popup_area = Rect::new(x, y, popup_width, popup_height);

    f.render_widget(Clear, popup_area);

    let mut lines = vec![
        Line::from(Span::styled(
            " COMMAND BROADCAST ",
            Style::default()
                .fg(styles::bg())
                .bg(styles::cyan())
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
    ];

    // List selected hosts (up to 8, with ellipsis if more)
    let mut sorted_hosts: Vec<&String> = app.selected_hosts.iter().collect();
    sorted_hosts.sort();

    for (i, host_name) in sorted_hosts.iter().enumerate() {
        if i >= 8 {
            let remaining = selected_count - 8;
            lines.push(Line::from(Span::styled(
                format!("  ... and {remaining} more"),
                Style::default().fg(styles::muted()),
            )));
            break;
        }
        lines.push(Line::from(vec![
            Span::styled("  ", Style::default()),
            Span::styled("\u{25cf} ", Style::default().fg(styles::cyan())),
            Span::styled(host_name.as_str(), Style::default().fg(styles::fg())),
        ]));
    }

    lines.push(Line::from(""));

    // Command input field with cursor
    lines.push(Line::from(vec![
        Span::styled("  Command: ", Style::default().fg(styles::muted())),
        Span::styled(
            format!("{}_", app.broadcast_command),
            Style::default().fg(styles::fg()),
        ),
    ]));

    lines.push(Line::from(""));

    if let Some(ref err) = app.broadcast_error {
        lines.push(Line::from(Span::styled(
            format!("  Error: {err}"),
            Style::default().fg(styles::red()),
        )));
        lines.push(Line::from(""));
    }

    lines.push(Line::from(Span::styled(
        "  Enter: execute | Esc: cancel",
        styles::help_text_style(),
    )));

    let paragraph = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(styles::cyan()))
            .style(Style::default().bg(styles::bg()).fg(styles::fg())),
    );
    f.render_widget(paragraph, popup_area);
}

fn draw_snippets_overlay(f: &mut Frame, app: &App, area: Rect) {
    let popup_width = 64u16.min(area.width.saturating_sub(4));

    let content_lines = if app.snippet_adding {
        10u16
    } else {
        let snippet_count = app.snippet_manager.snippets.len() as u16;
        (6 + snippet_count.max(1)).min(24)
    };
    let popup_height = (content_lines + 2).min(area.height.saturating_sub(4));
    let x = (area.width.saturating_sub(popup_width)) / 2;
    let y = (area.height.saturating_sub(popup_height)) / 2;
    let popup_area = Rect::new(x, y, popup_width, popup_height);

    f.render_widget(Clear, popup_area);

    let mut lines = vec![
        Line::from(Span::styled(
            " COMMAND SNIPPETS ",
            Style::default()
                .fg(styles::bg())
                .bg(styles::primary())
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
    ];

    if app.snippet_adding {
        let field_labels = ["  Name        ", "  Command     ", "  Description "];
        for (i, label) in field_labels.iter().enumerate() {
            let is_focused = app.snippet_focused == i;
            let indicator = if is_focused { "> " } else { "  " };
            let label_style = if is_focused {
                Style::default().fg(styles::primary()).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(styles::muted())
            };
            let cursor = if is_focused { "_" } else { "" };
            lines.push(Line::from(vec![
                Span::styled(indicator, Style::default().fg(styles::primary())),
                Span::styled(*label, label_style),
                Span::styled(
                    format!("{}{cursor}", app.snippet_fields[i]),
                    Style::default().fg(styles::fg()),
                ),
            ]));
        }

        lines.push(Line::from(""));

        if let Some(ref err) = app.snippet_error {
            lines.push(Line::from(Span::styled(
                format!("  Error: {err}"),
                Style::default().fg(styles::red()),
            )));
            lines.push(Line::from(""));
        }

        lines.push(Line::from(Span::styled(
            "  Tab: next field | Enter: save | Esc: cancel",
            styles::help_text_style(),
        )));
    } else {
        if app.snippet_manager.snippets.is_empty() {
            lines.push(Line::from(Span::styled(
                "  No snippets saved yet. Press 'a' to add one.",
                Style::default().fg(styles::muted()),
            )));
        } else {
            for (i, snippet) in app.snippet_manager.snippets.iter().enumerate() {
                let is_selected = i == app.snippet_selected;
                let indicator = if is_selected { "> " } else { "  " };

                let name_style = if is_selected {
                    Style::default()
                        .fg(styles::primary())
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(styles::fg())
                };

                let max_cmd = (popup_width as usize).saturating_sub(20);
                let cmd_preview = if snippet.command.len() > max_cmd {
                    format!("{}...", &snippet.command[..max_cmd.saturating_sub(3)])
                } else {
                    snippet.command.clone()
                };

                let row_style = if is_selected {
                    Style::default().bg(styles::selection_bg())
                } else {
                    Style::default()
                };

                lines.push(Line::from(vec![
                    Span::styled(format!("{indicator}"), Style::default().fg(styles::primary())),
                    Span::styled(format!("{:<16} ", snippet.name), name_style),
                    Span::styled(cmd_preview, Style::default().fg(styles::cyan())),
                ]).style(row_style));

                if is_selected && !snippet.description.is_empty() {
                    lines.push(Line::from(vec![
                        Span::styled("    ", Style::default()),
                        Span::styled(&snippet.description, Style::default().fg(styles::muted())),
                    ]));
                }
            }
        }

        lines.push(Line::from(""));

        let selected_host = app.selected_host().map(|h| h.name.as_str()).unwrap_or("none");
        lines.push(Line::from(vec![
            Span::styled("  Host: ", Style::default().fg(styles::muted())),
            Span::styled(selected_host, Style::default().fg(styles::fg()).add_modifier(Modifier::BOLD)),
        ]));

        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  a: add | d: delete | Enter: run on host | Esc: close",
            styles::help_text_style(),
        )));
    }


    let paragraph = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(styles::border_focused_style())
            .style(Style::default().bg(styles::bg()).fg(styles::fg())),
    );
    f.render_widget(paragraph, popup_area);
}

fn draw_group_create_overlay(f: &mut Frame, app: &App, area: Rect) {
    let popup_width = 50u16.min(area.width.saturating_sub(4));
    let popup_height = 7u16;
    let x = (area.width.saturating_sub(popup_width)) / 2;
    let y = (area.height.saturating_sub(popup_height)) / 2;
    let popup_area = Rect::new(x, y, popup_width, popup_height);

    f.render_widget(Clear, popup_area);

    let lines = vec![
        Line::from(Span::styled(
            " CREATE GROUP ",
            Style::default()
                .fg(styles::bg())
                .bg(styles::primary())
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Name: ", Style::default().fg(styles::muted())),
            Span::styled(
                format!("{}_", app.group_input),
                Style::default().fg(styles::fg()),
            ),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  Enter: create | Esc: cancel",
            styles::help_text_style(),
        )),
    ];

    let paragraph = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(styles::border_focused_style())
            .style(Style::default().bg(styles::bg()).fg(styles::fg())),
    );
    f.render_widget(paragraph, popup_area);
}

fn draw_group_picker_overlay(f: &mut Frame, app: &App, area: Rect) {
    let item_count = app.group_picker_items.len() as u16;
    let content_height = (4 + item_count.max(1)).min(20);
    let popup_height = (content_height + 2).min(area.height.saturating_sub(4));
    let popup_width = 44u16.min(area.width.saturating_sub(4));
    let x = (area.width.saturating_sub(popup_width)) / 2;
    let y = (area.height.saturating_sub(popup_height)) / 2;
    let popup_area = Rect::new(x, y, popup_width, popup_height);

    f.render_widget(Clear, popup_area);

    let host_name = app.selected_host().map(|h| h.name.as_str()).unwrap_or("?");

    let mut lines = vec![
        Line::from(Span::styled(
            " ASSIGN TO GROUP ",
            Style::default()
                .fg(styles::bg())
                .bg(styles::primary())
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(vec![
            Span::styled("  Host: ", Style::default().fg(styles::muted())),
            Span::styled(host_name, Style::default().fg(styles::fg()).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(""),
    ];

    if app.group_picker_items.is_empty() {
        lines.push(Line::from(Span::styled(
            "  No groups yet. Press G to create one.",
            Style::default().fg(styles::muted()),
        )));
    } else {
        for (i, group_name) in app.group_picker_items.iter().enumerate() {
            let is_selected = i == app.group_picker_selected;
            let indicator = if is_selected { "> " } else { "  " };
            let is_current = app.groups.get_group_for_host(host_name) == Some(group_name.as_str())
                || (group_name == "Ungrouped" && app.groups.get_group_for_host(host_name).is_none());

            let name_style = if is_selected {
                Style::default()
                    .fg(styles::primary())
                    .add_modifier(Modifier::BOLD)
            } else if is_current {
                Style::default().fg(styles::green())
            } else {
                Style::default().fg(styles::fg())
            };

            let current_marker = if is_current { " *" } else { "" };
            let row_style = if is_selected {
                Style::default().bg(styles::selection_bg())
            } else {
                Style::default()
            };

            lines.push(Line::from(vec![
                Span::styled(indicator, Style::default().fg(styles::primary())),
                Span::styled(format!("{}{}", group_name, current_marker), name_style),
            ]).style(row_style));
        }
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  j/k: navigate | Enter: assign | Esc: cancel",
        styles::help_text_style(),
    )));

    let paragraph = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(styles::border_focused_style())
            .style(Style::default().bg(styles::bg()).fg(styles::fg())),
    );
    f.render_widget(paragraph, popup_area);
}
