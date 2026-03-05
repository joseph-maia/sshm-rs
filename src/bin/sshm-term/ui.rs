use crate::{
    app::{App, ContextMenu, PanelFocus},
    sftp::SftpBrowser,
    terminal::TerminalPanelWidget,
};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
};

pub fn draw(frame: &mut Frame, app: &mut App) {
    let area = frame.area();
    app.frame_area = area;

    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .split(area);

    let main_area = vertical[0];
    let status_area = vertical[1];

    if app.show_sftp {
        let horizontal = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(65), Constraint::Percentage(35)])
            .split(main_area);

        render_terminal(frame, app, horizontal[0]);
        render_sftp(frame, app, horizontal[1]);
    } else {
        render_terminal(frame, app, main_area);
    }

    render_status(frame, app, status_area);

    if let Some(ref menu) = app.context_menu {
        render_context_menu(frame, menu);
    }

    if let Some(ref mut overlay) = app.snippet_overlay {
        render_snippet_overlay(frame, overlay);
    }
}

fn render_terminal(frame: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let focused = app.focus == PanelFocus::Terminal;
    let border_style = if focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let block = Block::default()
        .title(format!(" {}@{} ", app.user, app.host))
        .borders(Borders::ALL)
        .border_style(border_style);

    let inner = block.inner(area);
    frame.render_widget(block, area);
    frame.render_widget(TerminalPanelWidget::new(&app.terminal), inner);
}

fn render_sftp(frame: &mut Frame, app: &mut App, area: ratatui::layout::Rect) {
    let focused = app.focus == PanelFocus::Sftp;
    let border_style = if focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(0)])
        .split(area);

    let breadcrumb_area = vertical[0];
    let list_area = vertical[1];

    app.sftp_breadcrumb_area = Some(breadcrumb_area);
    app.sftp_list_area = Some(list_area);

    if app.sftp_editing_path {
        let input = Paragraph::new(Line::from(vec![
            Span::styled(" ", Style::default()),
            Span::styled("Go to: ", Style::default().fg(Color::Cyan)),
            Span::styled(&app.sftp_path_input, Style::default().fg(Color::White)),
            Span::styled("█", Style::default().fg(Color::White)),
        ]))
        .style(Style::default().bg(Color::Rgb(0x30, 0x30, 0x50)));
        frame.render_widget(input, breadcrumb_area);
    } else {
        let path_str = app.sftp.current_path.clone();
        let breadcrumb = Paragraph::new(Line::from(vec![
            Span::styled(" ", Style::default()),
            Span::styled(path_str, Style::default().fg(Color::Yellow)),
        ]))
        .style(Style::default().bg(Color::DarkGray));
        frame.render_widget(breadcrumb, breadcrumb_area);
    }

    let block = Block::default()
        .title(" SFTP ")
        .borders(Borders::ALL)
        .border_style(border_style);

    if app.sftp.loading {
        let loading = Paragraph::new("Loading...").block(block);
        frame.render_widget(loading, list_area);
        return;
    }

    if let Some(ref err) = app.sftp.error {
        let error_msg = Paragraph::new(Line::from(vec![Span::styled(
            format!("Error: {err}"),
            Style::default().fg(Color::Red),
        )]))
        .block(block);
        frame.render_widget(error_msg, list_area);
        return;
    }

    if app.sftp.entries.is_empty() {
        let empty = Paragraph::new(Line::from(vec![Span::styled(
            "(empty directory)",
            Style::default().fg(Color::DarkGray),
        )]))
        .block(block);
        frame.render_widget(empty, list_area);
        return;
    }

    let inner_width = list_area.width.saturating_sub(4) as usize;
    let size_col = 9usize;
    let perm_col = 9usize;
    let name_col = inner_width.saturating_sub(size_col + perm_col + 2);

    let items: Vec<ListItem> = app
        .sftp
        .entries
        .iter()
        .map(|e| {
            let (icon, icon_style) = if e.name == ".." {
                ("↩ ", Style::default().fg(Color::Yellow))
            } else if e.is_dir {
                ("d ", Style::default().fg(Color::Blue))
            } else {
                ("  ", Style::default())
            };

            let name_truncated = if e.name.chars().count() > name_col {
                let truncated: String = e.name.chars().take(name_col.saturating_sub(1)).collect();
                format!("{}~", truncated)
            } else {
                format!("{:<width$}", e.name, width = name_col)
            };

            let size_str = if e.is_dir {
                format!("{:>width$}", "-", width = size_col)
            } else {
                format!(
                    "{:>width$}",
                    SftpBrowser::format_size(e.size),
                    width = size_col
                )
            };

            let perms_str = SftpBrowser::format_permissions(e.permissions);

            let line = Line::from(vec![
                Span::styled(icon, icon_style),
                Span::raw(name_truncated),
                Span::raw(" "),
                Span::styled(size_str, Style::default().fg(Color::Green)),
                Span::raw(" "),
                Span::styled(perms_str, Style::default().fg(Color::DarkGray)),
            ]);

            ListItem::new(line)
        })
        .collect();

    let list = List::new(items)
        .block(block)
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
        .highlight_symbol(">");

    app.sftp_list_state.select(Some(app.sftp.selected_index));

    frame.render_stateful_widget(list, list_area, &mut app.sftp_list_state);
}

fn render_context_menu(frame: &mut Frame, menu: &ContextMenu) {
    let width = menu.items.iter().map(|i| i.label.len()).max().unwrap_or(10) as u16 + 4;
    let height = menu.items.len() as u16 + 2;

    let area = frame.area();
    let x = menu.x.min(area.width.saturating_sub(width));
    let y = menu.y.min(area.height.saturating_sub(height));

    let menu_area = Rect::new(x, y, width, height);

    frame.render_widget(Clear, menu_area);

    let items: Vec<ListItem> = menu
        .items
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let style = if i == menu.selected {
                Style::default().bg(Color::Rgb(0x40, 0x40, 0x60)).fg(Color::White)
            } else {
                Style::default().fg(Color::White)
            };
            ListItem::new(Span::styled(format!(" {} ", item.label), style))
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan))
            .style(Style::default().bg(Color::Rgb(0x20, 0x20, 0x30))),
    );

    frame.render_widget(list, menu_area);
}

fn render_snippet_overlay(
    frame: &mut Frame,
    overlay: &mut crate::snippets::SnippetOverlay,
) {
    let area = frame.area();
    let width = ((area.width as f32 * 0.6) as u16).max(50).min(area.width);
    let height = ((area.height as f32 * 0.7) as u16).max(10).min(area.height);
    let x = area.width.saturating_sub(width) / 2;
    let y = area.height.saturating_sub(height) / 2;
    let overlay_rect = Rect::new(x, y, width, height);

    overlay.overlay_area = Some(overlay_rect);

    frame.render_widget(Clear, overlay_rect);

    match overlay.mode {
        crate::snippets::SnippetMode::Browse => {
            render_snippet_browse(frame, overlay, overlay_rect);
        }
        crate::snippets::SnippetMode::Add | crate::snippets::SnippetMode::Edit => {
            render_snippet_form(frame, overlay, overlay_rect);
        }
        crate::snippets::SnippetMode::ConfirmDelete => {
            render_snippet_delete(frame, overlay, overlay_rect);
        }
    }
}

fn render_snippet_browse(
    frame: &mut Frame,
    overlay: &mut crate::snippets::SnippetOverlay,
    area: Rect,
) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(area);

    let search_area = layout[0];
    let list_area = layout[1];
    let footer_area = layout[2];

    overlay.list_area = Some(list_area);

    let count = overlay.filtered_indices.len();
    let title = if overlay.search_input.is_empty() {
        format!(" Snippets ({}) ", count)
    } else {
        format!(
            " Snippets / Search: \"{}\" ({}) ",
            overlay.search_input, count
        )
    };

    let search_block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .style(Style::default().bg(Color::Rgb(0x1a, 0x1b, 0x26)));
    frame.render_widget(search_block, search_area);

    let list_block = Block::default()
        .borders(Borders::LEFT | Borders::RIGHT | Borders::BOTTOM)
        .border_style(Style::default().fg(Color::Cyan))
        .style(Style::default().bg(Color::Rgb(0x1a, 0x1b, 0x26)));
    let inner = list_block.inner(list_area);
    frame.render_widget(list_block, list_area);

    if overlay.filtered_indices.is_empty() {
        let msg = if overlay.snippets.is_empty() {
            "No snippets. Press 'a' to create one."
        } else {
            "(no matches)"
        };
        let p = Paragraph::new(Span::styled(
            msg,
            Style::default().fg(Color::DarkGray),
        ));
        frame.render_widget(p, inner);
    } else {
        let visible_items = (inner.height as usize) / 3;

        if overlay.selected_index >= overlay.scroll_offset + visible_items.max(1) {
            overlay.scroll_offset = overlay
                .selected_index
                .saturating_sub(visible_items.saturating_sub(1));
        }
        if overlay.selected_index < overlay.scroll_offset {
            overlay.scroll_offset = overlay.selected_index;
        }

        let mut y_offset = 0u16;
        for (display_idx, &snippet_idx) in overlay
            .filtered_indices
            .iter()
            .enumerate()
            .skip(overlay.scroll_offset)
            .take(visible_items.max(1))
        {
            let snippet = &overlay.snippets[snippet_idx];
            let is_selected = display_idx == overlay.selected_index;

            let name_style = if is_selected {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            let cmd_style = Style::default().fg(Color::Green);
            let desc_style = Style::default().fg(Color::DarkGray);
            let marker = if is_selected { "\u{25b8} " } else { "  " };

            if y_offset < inner.height {
                let name_area = Rect::new(inner.x, inner.y + y_offset, inner.width, 1);
                let name_line = Paragraph::new(Line::from(vec![
                    Span::styled(marker, name_style),
                    Span::styled(&snippet.name, name_style),
                ]));
                frame.render_widget(name_line, name_area);
            }
            y_offset += 1;

            if y_offset < inner.height {
                let cmd_area = Rect::new(inner.x, inner.y + y_offset, inner.width, 1);
                let cmd_text: String = snippet
                    .command
                    .chars()
                    .take(inner.width.saturating_sub(4) as usize)
                    .collect();
                let cmd_line = Paragraph::new(Line::from(vec![
                    Span::raw("    "),
                    Span::styled(cmd_text, cmd_style),
                ]));
                frame.render_widget(cmd_line, cmd_area);
            }
            y_offset += 1;

            if y_offset < inner.height {
                let desc_area = Rect::new(inner.x, inner.y + y_offset, inner.width, 1);
                let desc_line = Paragraph::new(Line::from(vec![
                    Span::raw("    "),
                    Span::styled(&snippet.description, desc_style),
                ]));
                frame.render_widget(desc_line, desc_area);
            }
            y_offset += 1;
        }
    }

    let footer = Paragraph::new(Line::from(vec![
        Span::styled(" Enter", Style::default().fg(Color::Yellow)),
        Span::raw(":exec"),
        Span::raw("  "),
        Span::styled("a", Style::default().fg(Color::Yellow)),
        Span::raw(":add"),
        Span::raw("  "),
        Span::styled("e", Style::default().fg(Color::Yellow)),
        Span::raw(":edit"),
        Span::raw("  "),
        Span::styled("d", Style::default().fg(Color::Yellow)),
        Span::raw(":del"),
        Span::raw("  "),
        Span::styled("Esc", Style::default().fg(Color::Yellow)),
        Span::raw(":close"),
    ]))
    .style(Style::default().bg(Color::Rgb(0x1a, 0x1b, 0x26)));
    frame.render_widget(footer, footer_area);
}

fn render_snippet_form(
    frame: &mut Frame,
    overlay: &crate::snippets::SnippetOverlay,
    area: Rect,
) {
    let form = match &overlay.form {
        Some(f) => f,
        None => return,
    };

    let title = if form.editing_index.is_some() {
        " Edit Snippet "
    } else {
        " Add Snippet "
    };

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .style(Style::default().bg(Color::Rgb(0x1a, 0x1b, 0x26)));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let fields = [
        (
            "Name:        ",
            &form.name,
            crate::snippets::AddFormField::Name,
        ),
        (
            "Command:     ",
            &form.command,
            crate::snippets::AddFormField::Command,
        ),
        (
            "Description: ",
            &form.description,
            crate::snippets::AddFormField::Description,
        ),
    ];

    for (i, (label, value, field)) in fields.iter().enumerate() {
        if (i as u16) * 2 >= inner.height {
            break;
        }
        let y = inner.y + (i as u16) * 2;
        let field_area = Rect::new(inner.x + 1, y, inner.width.saturating_sub(2), 1);

        let is_active = form.active_field == *field;
        let label_style = if is_active {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        let cursor = if is_active { "\u{2588}" } else { "" };

        let line = Line::from(vec![
            Span::styled(*label, label_style),
            Span::styled(*value, Style::default().fg(Color::White)),
            Span::styled(cursor, Style::default().fg(Color::White)),
        ]);
        frame.render_widget(Paragraph::new(line), field_area);
    }

    let footer_y = inner.y + inner.height.saturating_sub(1);
    let footer_area = Rect::new(inner.x, footer_y, inner.width, 1);
    let footer = Paragraph::new(Line::from(vec![
        Span::styled(" Tab", Style::default().fg(Color::Yellow)),
        Span::raw(":next field"),
        Span::raw("  "),
        Span::styled("Enter", Style::default().fg(Color::Yellow)),
        Span::raw(":save"),
        Span::raw("  "),
        Span::styled("Esc", Style::default().fg(Color::Yellow)),
        Span::raw(":cancel"),
    ]));
    frame.render_widget(footer, footer_area);
}

fn render_snippet_delete(
    frame: &mut Frame,
    overlay: &crate::snippets::SnippetOverlay,
    area: Rect,
) {
    let name = overlay
        .selected_snippet()
        .map(|s| s.name.as_str())
        .unwrap_or("?");

    let block = Block::default()
        .title(" Delete Snippet ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Red))
        .style(Style::default().bg(Color::Rgb(0x1a, 0x1b, 0x26)));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let msg = Paragraph::new(vec![
        Line::from(""),
        Line::from(vec![Span::styled(
            format!("  Delete '{}'?", name),
            Style::default()
                .fg(Color::Red)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::raw("  "),
            Span::styled("y", Style::default().fg(Color::Yellow)),
            Span::raw(":yes"),
            Span::raw("  "),
            Span::styled("n", Style::default().fg(Color::Yellow)),
            Span::raw(":cancel"),
        ]),
    ]);
    frame.render_widget(msg, inner);
}

fn render_status(frame: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    if app.confirm_delete.is_some() {
        let paragraph = Paragraph::new(Line::from(vec![
            Span::styled(" \u{26a0} ", Style::default().fg(Color::Red)),
            Span::styled(&app.status_message, Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
        ]))
        .style(Style::default().bg(Color::DarkGray));
        frame.render_widget(paragraph, area);
        return;
    }

    let mut keys = vec![
        Span::styled(" Ctrl+Q", Style::default().fg(Color::Yellow)),
        Span::raw(" quit"),
        Span::raw("  "),
        Span::styled("Ctrl+S", Style::default().fg(Color::Yellow)),
        Span::raw(" switch panel"),
        Span::raw("  "),
        Span::styled("Ctrl+B", Style::default().fg(Color::Yellow)),
        Span::raw(" toggle SFTP"),
        Span::raw("  "),
        Span::styled("Ctrl+F", Style::default().fg(Color::Yellow)),
        Span::raw(" follow dir"),
        Span::raw("  "),
        Span::styled("/", Style::default().fg(Color::Yellow)),
        Span::raw(" go to path"),
        Span::raw("  "),
        Span::styled("d", Style::default().fg(Color::Yellow)),
        Span::raw(" download"),
        Span::raw("  "),
        Span::styled("u", Style::default().fg(Color::Yellow)),
        Span::raw(" upload"),
        Span::raw("  "),
        Span::styled("Ctrl+P", Style::default().fg(Color::Yellow)),
        Span::raw(" snippets"),
        Span::raw("  "),
        Span::styled(&app.status_message, Style::default().fg(Color::Green)),
    ];

    if app.sftp_follow_terminal {
        keys.push(Span::raw("  "));
        keys.push(Span::styled("[Follow]", Style::default().fg(Color::Cyan)));
    }

    let paragraph = Paragraph::new(Line::from(keys)).style(Style::default().bg(Color::DarkGray));

    frame.render_widget(paragraph, area);
}
