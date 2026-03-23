use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState};
use ratatui::Frame;

use crate::connectivity::HostStatus;
use crate::ui::app::{App, DisplayRow, ViewMode};
use crate::ui::styles;

pub const TITLE_HEIGHT: u16 = 2;
pub const TITLE_HEIGHT_COMPACT: u16 = 1;

pub fn draw(f: &mut Frame, app: &App) {
    let area = f.area();

    // Clear background with theme bg
    let bg_block = Block::default().style(Style::default().bg(styles::bg()));
    f.render_widget(bg_block, area);

    // Responsive layout: compact title when terminal is small
    let compact = area.height < 20;
    let title_height = if compact { TITLE_HEIGHT_COMPACT } else { TITLE_HEIGHT };

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
            draw_title(f, app, chunks[0]);
        }
        draw_search_bar(f, app, chunks[1]);
        draw_host_list(f, app, chunks[2]);
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
        ViewMode::GroupCreate => draw_group_create_overlay(f, app, area),
        ViewMode::GroupPicker => draw_group_picker_overlay(f, app, area),
        ViewMode::ThemePicker => draw_theme_picker_overlay(f, app, area),
        _ => {}
    }
}

fn draw_title(f: &mut Frame, app: &App, area: Rect) {
    let group_count = app.groups.groups.len();
    let conn_count = app.hosts.len();
    let subtitle_base = format!("{} connections \u{00b7} {} groups", conn_count, group_count);

    let mut subtitle_spans = vec![Span::styled(
        subtitle_base,
        Style::default().fg(styles::muted()),
    )];

    if let Some(ref version) = app.update_available {
        subtitle_spans.push(Span::styled(
            format!(" \u{2502} v{} available \u{2014} run sshm-rs update", version),
            Style::default().fg(styles::yellow()),
        ));
    }

    let lines = vec![
        Line::from(Span::styled(
            "SSH Connection Manager",
            Style::default()
                .fg(styles::primary())
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(subtitle_spans),
    ];

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

    let icon_style = if app.search_mode {
        Style::default().fg(styles::primary())
    } else {
        Style::default().fg(styles::muted())
    };

    let spans = if app.search_query.is_empty() && !app.search_mode {
        vec![
            Span::styled("\u{25b7} ", icon_style),
            Span::styled("Search...", Style::default().fg(styles::muted())),
        ]
    } else {
        let cursor_suffix = if app.search_mode { "_" } else { "" };
        vec![
            Span::styled("\u{25b7} ", icon_style),
            Span::styled(
                format!("{}{}", app.search_query, cursor_suffix),
                Style::default().fg(styles::fg()),
            ),
        ]
    };

    let search = Paragraph::new(Line::from(spans))
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

fn draw_host_list(f: &mut Frame, app: &App, area: Rect) {
    let border_style = if !app.search_mode && !app.sidebar_focused {
        styles::border_focused_style()
    } else {
        styles::border_unfocused_style()
    };

    let inner_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(border_style)
        .style(Style::default().bg(styles::bg()));

    let inner_area = inner_block.inner(area);
    f.render_widget(inner_block, area);

    let available_width = inner_area.width as usize;

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

    // Fixed column layout: [prefix 5][name col][tags col][user col]
    // prefix = status(3) + icon(2) = 5 chars (fav star is inside name col)
    let prefix_w: usize = 5;
    let user_col_w: usize = 14; // fixed width for username
    let remaining = available_width.saturating_sub(prefix_w + user_col_w);
    // Split remaining 60/40 between name and tags
    let name_col_w = (remaining * 55) / 100;
    let tags_col_w = remaining.saturating_sub(name_col_w);

    let lines: Vec<Line> = visible_range
        .map(|abs_idx| {
            let is_cursor = abs_idx == app.selected;
            let is_hovered = app.hovered_index == Some(abs_idx) && !is_cursor;

            let display_row = display_source.get(abs_idx).cloned()
                .unwrap_or(DisplayRow::HostRow(abs_idx));

            match display_row {
                DisplayRow::GroupHeader { name, host_count, collapsed } => {
                    let collapse_icon = if collapsed { "\u{25b8}" } else { "\u{25be}" };
                    let header_text = format!("  {} {} ({})", collapse_icon, name.to_uppercase(), host_count);
                    let pad_len = available_width.saturating_sub(header_text.len());
                    let padded = format!("{}{}", header_text, " ".repeat(pad_len));
                    let row_bg = if is_cursor {
                        styles::selection_bg()
                    } else if is_hovered {
                        styles::hover_bg()
                    } else {
                        styles::bg()
                    };
                    Line::from(Span::styled(
                        padded,
                        Style::default()
                            .fg(styles::primary())
                            .bg(row_bg)
                            .add_modifier(Modifier::BOLD),
                    ))
                }
                DisplayRow::HostRow(host_idx) => {
                    let host = match app.filtered_hosts.get(host_idx) {
                        Some(h) => h,
                        None => return Line::from(""),
                    };

                    let is_multi_selected = app.selected_hosts.contains(&host.name);

                    let row_bg = if is_cursor {
                        styles::selection_bg()
                    } else if is_multi_selected {
                        ratatui::style::Color::Rgb(0x1e, 0x2a, 0x3a)
                    } else if is_hovered {
                        styles::hover_bg()
                    } else {
                        styles::bg()
                    };

                    let mut spans: Vec<Span> = Vec::new();

                    // === PREFIX column (5 chars): status dot + icon ===
                    if is_multi_selected {
                        spans.push(Span::styled(" \u{2713} ", Style::default().fg(styles::cyan()).bg(row_bg)));
                    } else {
                        let (indicator, status) = app.get_status_indicator(&host.name);
                        let status_style = match status {
                            HostStatus::Online(_) => styles::status_online_style(),
                            HostStatus::Offline(_) => styles::status_offline_style(),
                            HostStatus::Connecting => styles::status_connecting_style(),
                            HostStatus::Unknown => styles::status_unknown_style(),
                        };
                        spans.push(Span::styled(
                            format!(" {} ", indicator),
                            status_style.bg(row_bg),
                        ));
                    }
                    spans.push(Span::styled("\u{25b8} ", Style::default().fg(styles::muted()).bg(row_bg)));

                    // === NAME column (fixed width) ===
                    let is_fav = app.favorites.is_favorite(&host.name);
                    let fav_prefix = if is_fav { "\u{2605} " } else { "" };
                    let name_display = if host.name != host.hostname && !host.hostname.is_empty() {
                        format!("{}{} ({})", fav_prefix, host.name, host.hostname)
                    } else {
                        format!("{}{}", fav_prefix, host.name)
                    };
                    // Truncate or pad to fixed width
                    let name_truncated = if name_display.len() > name_col_w {
                        format!("{}\u{2026}", &name_display[..name_col_w.saturating_sub(1)])
                    } else {
                        format!("{:<width$}", name_display, width = name_col_w)
                    };
                    let hostname_style = if is_cursor {
                        Style::default().fg(styles::primary()).bg(row_bg).add_modifier(Modifier::BOLD)
                    } else if is_multi_selected {
                        Style::default().fg(styles::cyan()).bg(row_bg).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(styles::fg()).bg(row_bg).add_modifier(Modifier::BOLD)
                    };
                    if is_fav && !name_truncated.is_empty() {
                        // Color the star separately
                        let star_end = "\u{2605} ".len();
                        if name_truncated.len() >= star_end && name_truncated.starts_with('\u{2605}') {
                            spans.push(Span::styled(
                                "\u{2605} ".to_string(),
                                Style::default().fg(styles::yellow()).bg(row_bg),
                            ));
                            spans.push(Span::styled(name_truncated[star_end..].to_string(), hostname_style));
                        } else {
                            spans.push(Span::styled(name_truncated, hostname_style));
                        }
                    } else {
                        spans.push(Span::styled(name_truncated, hostname_style));
                    }

                    // === TAGS column (fixed width) ===
                    let mut tag_spans: Vec<Span> = Vec::new();
                    let mut tags_used: usize = 0;
                    for tag in &host.tags {
                        let badge = format!(" {} ", tag);
                        let badge_len = badge.len() + 1; // +1 for space after
                        if tags_used + badge_len <= tags_col_w {
                            tag_spans.push(Span::styled(badge, styles::tag_style(tag)));
                            tag_spans.push(Span::styled(" ", Style::default().bg(row_bg)));
                            tags_used += badge_len;
                        }
                    }
                    // Pad tags column to fixed width
                    if tags_used < tags_col_w {
                        tag_spans.push(Span::styled(
                            " ".repeat(tags_col_w - tags_used),
                            Style::default().bg(row_bg),
                        ));
                    }
                    spans.extend(tag_spans);

                    // === USER column (fixed width, right-aligned) ===
                    let user_text = if host.user.is_empty() {
                        " ".repeat(user_col_w)
                    } else {
                        let u = &host.user;
                        if u.len() >= user_col_w {
                            u[..user_col_w].to_string()
                        } else {
                            format!("{:>width$}", u, width = user_col_w)
                        }
                    };
                    spans.push(Span::styled(user_text, Style::default().fg(styles::muted()).bg(row_bg)));

                    Line::from(spans)
                }
            }
        })
        .collect();

    let list_paragraph = Paragraph::new(lines);
    f.render_widget(list_paragraph, inner_area);

    // Render scrollbar when there are more rows than visible
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

    let separator_style = Style::default().fg(styles::muted());
    let key_style = Style::default().fg(styles::primary()).add_modifier(Modifier::BOLD);
    let desc_style = Style::default().fg(styles::muted());

    let left_spans: Vec<Span> = if show_toast {
        let msg = format!(" {} ", app.toast_message.as_deref().unwrap_or(""));
        let color = if app.toast_is_error { styles::red() } else { styles::green() };
        vec![Span::styled(msg, Style::default().fg(color))]
    } else if app.search_mode {
        vec![
            Span::styled(" Type to filter ", desc_style),
            Span::styled("\u{2502}", separator_style),
            Span::styled(" Enter", key_style),
            Span::styled(" validate ", desc_style),
            Span::styled("\u{2502}", separator_style),
            Span::styled(" Esc", key_style),
            Span::styled(" close", desc_style),
        ]
    } else if app.has_selection() {
        let count = app.selected_hosts.len();
        vec![
            Span::styled(format!(" {count} selected "), Style::default().fg(styles::cyan())),
            Span::styled("\u{2502}", separator_style),
            Span::styled(" Space", key_style),
            Span::styled(" toggle ", desc_style),
            Span::styled("\u{2502}", separator_style),
            Span::styled(" b", key_style),
            Span::styled(" broadcast ", desc_style),
            Span::styled("\u{2502}", separator_style),
            Span::styled(" d", key_style),
            Span::styled(" delete ", desc_style),
            Span::styled("\u{2502}", separator_style),
            Span::styled(" Esc", key_style),
            Span::styled(" clear", desc_style),
        ]
    } else {
        vec![
            Span::styled(" \u{21b5}", key_style),
            Span::styled(" terminal ", desc_style),
            Span::styled("\u{2502}", separator_style),
            Span::styled(" \u{21e7}\u{21b5}", key_style),
            Span::styled(" ssh ", desc_style),
            Span::styled("\u{2502}", separator_style),
            Span::styled(" a", key_style),
            Span::styled(" add ", desc_style),
            Span::styled("\u{2502}", separator_style),
            Span::styled(" e", key_style),
            Span::styled(" edit ", desc_style),
            Span::styled("\u{2502}", separator_style),
            Span::styled(" d", key_style),
            Span::styled(" delete ", desc_style),
            Span::styled("\u{2502}", separator_style),
            Span::styled(" f", key_style),
            Span::styled(" fav ", desc_style),
            Span::styled("\u{2502}", separator_style),
            Span::styled(" ?", key_style),
            Span::styled(" help", desc_style),
        ]
    };

    let count = app.filtered_hosts.len();
    let total = app.hosts.len();
    let sort_label = app.sort_mode.label();
    let right = format!("[{count}/{total}] Sort: {sort_label} ");

    // Build optional warning indicator
    let warn_count = app.config_warnings.len();
    let warn_text = if warn_count > 0 {
        format!(" \u{26a0} {} ", warn_count)
    } else {
        String::new()
    };

    // Calculate left content length for padding
    let left_len: usize = left_spans.iter().map(|s| s.content.len()).sum();
    let right_len = right.len() + warn_text.len();
    let total_len = area.width as usize;
    let pad = if total_len > left_len + right_len {
        total_len - left_len - right_len
    } else {
        1
    };

    let mut spans = left_spans;
    spans.push(Span::raw(" ".repeat(pad)));
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
                    Span::styled(indicator.to_string(), Style::default().fg(styles::primary())),
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

fn draw_theme_picker_overlay(f: &mut Frame, app: &App, area: Rect) {
    let presets = crate::theme::Theme::presets();
    let item_count = presets.len() as u16;
    let popup_height = (item_count + 6).min(area.height.saturating_sub(4));
    let popup_width = 40u16.min(area.width.saturating_sub(4));
    let x = (area.width.saturating_sub(popup_width)) / 2;
    let y = (area.height.saturating_sub(popup_height)) / 2;
    let popup_area = Rect::new(x, y, popup_width, popup_height);

    f.render_widget(Clear, popup_area);

    let mut lines = vec![
        Line::from(Span::styled(
            " THEME PICKER ",
            Style::default()
                .fg(styles::bg())
                .bg(styles::primary())
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
    ];

    for (i, preset) in presets.iter().enumerate() {
        let is_selected = i == app.theme_picker_index;
        let indicator = if is_selected { "> " } else { "  " };
        let style = if is_selected {
            Style::default()
                .fg(styles::primary())
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(styles::fg())
        };
        lines.push(Line::from(vec![
            Span::styled(indicator, Style::default().fg(styles::primary())),
            Span::styled(preset.name.clone(), style),
        ]));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  j/k: navigate | Enter: apply | Esc: cancel",
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
