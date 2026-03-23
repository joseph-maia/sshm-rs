use super::{
    app::{App, ContextMenu, PanelFocus},
    sftp::SftpBrowser,
    terminal::TerminalPanelWidget,
    transfer::{TransferDirection, TransferManager},
};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
};
use super::styles as styles;

fn file_icon(name: &str, is_dir: bool) -> (&'static str, Color) {
    if name == ".." {
        return ("󰜱 ", Color::Yellow);
    }
    if is_dir {
        return (" ", Color::Rgb(0x00, 0xbc, 0xd4));
    }

    let name_lower = name.to_lowercase();
    let by_name: Option<(&str, Color)> = match name_lower.as_str() {
        ".gitignore" | ".gitconfig" | ".gitmodules" | ".gitattributes" => {
            Some((" ", Color::Rgb(0xf5, 0x4d, 0x27)))
        }
        ".env" => Some((" ", Color::Rgb(0xfa, 0xf7, 0x43))),
        ".dockerignore"
        | "dockerfile"
        | "docker-compose.yml"
        | "docker-compose.yaml"
        | "compose.yml"
        | "compose.yaml" => Some(("󰡨 ", Color::Rgb(0x45, 0x8e, 0xe6))),
        ".bashrc" | ".bash_profile" | ".zshrc" | ".zshenv" | ".zprofile" => {
            Some((" ", Color::Rgb(0x89, 0xe0, 0x51)))
        }
        "makefile" => Some((" ", Color::Rgb(0x6d, 0x80, 0x86))),
        "cargo.toml" | "cargo.lock" => Some((" ", Color::Rgb(0xde, 0xa5, 0x84))),
        "license" | "licence" | "copying" => Some((" ", Color::Rgb(0xcb, 0xcb, 0x41))),
        "readme.md" | "readme" => Some((" ", Color::Rgb(0xdd, 0xdd, 0xdd))),
        _ => None,
    };
    if let Some(result) = by_name {
        return result;
    }

    let ext = name.rsplit('.').next().unwrap_or("").to_lowercase();
    match ext.as_str() {
        "rs" => (" ", Color::Rgb(0xde, 0xa5, 0x84)),
        "toml" => (" ", Color::Rgb(0x9c, 0x42, 0x21)),
        "yml" | "yaml" => (" ", Color::Rgb(0x6d, 0x80, 0x86)),
        "json" => (" ", Color::Rgb(0xcb, 0xcb, 0x41)),
        "xml" => ("󰗀 ", Color::Rgb(0xe3, 0x79, 0x33)),
        "ini" | "cfg" | "conf" => (" ", Color::Rgb(0x6d, 0x80, 0x86)),
        "md" => (" ", Color::Rgb(0xdd, 0xdd, 0xdd)),
        "txt" => ("󰈙 ", Color::Rgb(0x89, 0xe0, 0x51)),
        "pdf" => (" ", Color::Rgb(0xb3, 0x0b, 0x00)),
        "doc" | "docx" => ("󰈬 ", Color::Rgb(0x18, 0x5a, 0xbd)),
        "xls" | "xlsx" => ("󰈛 ", Color::Rgb(0x20, 0x72, 0x45)),
        "ppt" | "pptx" => ("󰈧 ", Color::Rgb(0xcb, 0x4a, 0x32)),
        "csv" => (" ", Color::Rgb(0x89, 0xe0, 0x51)),
        "html" => (" ", Color::Rgb(0xe4, 0x4d, 0x26)),
        "css" => (" ", Color::Rgb(0x66, 0x33, 0x99)),
        "js" => (" ", Color::Rgb(0xcb, 0xcb, 0x41)),
        "jsx" => (" ", Color::Rgb(0x20, 0xc2, 0xe3)),
        "ts" => (" ", Color::Rgb(0x51, 0x9a, 0xba)),
        "tsx" => (" ", Color::Rgb(0x13, 0x54, 0xbf)),
        "py" => (" ", Color::Rgb(0xff, 0xbc, 0x03)),
        "go" => (" ", Color::Rgb(0x00, 0xad, 0xd8)),
        "c" => (" ", Color::Rgb(0x59, 0x9e, 0xff)),
        "cpp" | "cc" | "cxx" => (" ", Color::Rgb(0x51, 0x9a, 0xba)),
        "h" | "hpp" => (" ", Color::Rgb(0xa0, 0x74, 0xc4)),
        "java" => (" ", Color::Rgb(0xcc, 0x3e, 0x44)),
        "rb" => (" ", Color::Rgb(0x70, 0x15, 0x16)),
        "lua" => (" ", Color::Rgb(0x51, 0xa0, 0xcf)),
        "sql" => (" ", Color::Rgb(0xda, 0xd8, 0xd8)),
        "vim" => (" ", Color::Rgb(0x01, 0x98, 0x33)),
        "wasm" => (" ", Color::Rgb(0x5c, 0x4c, 0xdb)),
        "sh" => (" ", Color::Rgb(0x4d, 0x5a, 0x5e)),
        "bash" => (" ", Color::Rgb(0x89, 0xe0, 0x51)),
        "zsh" => (" ", Color::Rgb(0x89, 0xe0, 0x51)),
        "fish" => (" ", Color::Rgb(0x4d, 0x5a, 0x5e)),
        "tar" | "gz" | "tgz" | "bz2" | "xz" | "7z" | "rar" | "zip" => {
            (" ", Color::Rgb(0xec, 0xa5, 0x17))
        }
        "jpg" | "jpeg" | "png" | "gif" | "bmp" | "webp" => (" ", Color::Rgb(0xa0, 0x74, 0xc4)),
        "svg" => ("󰜡 ", Color::Rgb(0xff, 0xb1, 0x3b)),
        "ico" => (" ", Color::Rgb(0xcb, 0xcb, 0x41)),
        "mp3" | "wav" | "flac" | "ogg" | "aac" => (" ", Color::Rgb(0x00, 0xaf, 0xff)),
        "mp4" | "avi" | "mkv" | "mov" | "webm" => (" ", Color::Rgb(0xfd, 0x97, 0x1f)),
        "exe" | "bin" => (" ", Color::Rgb(0x9f, 0x05, 0x00)),
        "so" | "o" => (" ", Color::Rgb(0xdc, 0xdd, 0xd6)),
        "dll" => (" ", Color::Rgb(0x4d, 0x2c, 0x0b)),
        "deb" | "rpm" => (" ", Color::Rgb(0xd0, 0xbe, 0xc8)),
        "dmg" | "iso" | "img" => (" ", Color::Rgb(0xd0, 0xbe, 0xc8)),
        "log" => ("󰌱 ", Color::Rgb(0xdd, 0xdd, 0xdd)),
        "lock" => (" ", Color::Rgb(0xbb, 0xbb, 0xbb)),
        "env" => (" ", Color::Rgb(0xfa, 0xf7, 0x43)),
        "pem" | "key" | "crt" => ("󰌆 ", Color::Rgb(0xe3, 0xc5, 0x8e)),
        "pub" => ("󰷖 ", Color::Rgb(0xe3, 0xc5, 0x8e)),
        "ttf" | "otf" | "woff" | "woff2" => (" ", Color::Rgb(0xec, 0xec, 0xec)),
        _ if name.starts_with('.') => (" ", Color::Rgb(0x6d, 0x80, 0x86)),
        _ => (" ", Color::Rgb(0x6d, 0x80, 0x86)),
    }
}

fn filename_color(name: &str, is_dir: bool, permissions: u32) -> Style {
    if is_dir {
        return Style::default()
            .fg(styles::primary())
            .add_modifier(Modifier::BOLD);
    }
    let ext = name.rsplit('.').next().unwrap_or("").to_lowercase();
    match ext.as_str() {
        "tar" | "gz" | "tgz" | "bz2" | "xz" | "7z" | "rar" | "zip" => {
            return Style::default().fg(Color::Rgb(0xec, 0xa5, 0x17));
        }
        _ => {}
    }
    if name.starts_with('.') {
        return Style::default().fg(styles::muted());
    }
    if permissions & 0o111 != 0 {
        return Style::default().fg(styles::green());
    }
    Style::default()
}

pub fn draw(frame: &mut Frame, app: &mut App) {
    let area = frame.area();
    app.frame_area = area;

    // Expand status bar by 1 row per active transfer (max 3 visible), plus 1 for the keybinding bar
    let active_count = app.transfers.active_count().min(3);
    let status_height = 1 + active_count as u16;

    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(status_height)])
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
        Style::default().fg(styles::primary())
    } else {
        Style::default().fg(styles::muted())
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
        Style::default().fg(styles::primary())
    } else {
        Style::default().fg(styles::muted())
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
            Span::styled("Go to: ", Style::default().fg(styles::primary())),
            Span::styled(&app.sftp_path_input, Style::default().fg(styles::fg())),
            Span::styled("█", Style::default().fg(styles::fg())),
        ]))
        .style(Style::default().bg(styles::selection_bg()));
        frame.render_widget(input, breadcrumb_area);
    } else {
        let path_str = app.sftp.current_path.clone();
        let breadcrumb = Paragraph::new(Line::from(vec![
            Span::styled(" ", Style::default()),
            Span::styled(path_str, Style::default().fg(styles::yellow())),
        ]))
        .style(Style::default().bg(styles::muted()));
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
            Style::default().fg(styles::red()),
        )]))
        .block(block);
        frame.render_widget(error_msg, list_area);
        return;
    }

    if app.sftp.entries.is_empty() {
        let empty = Paragraph::new(Line::from(vec![Span::styled(
            "(empty directory)",
            Style::default().fg(styles::muted()),
        )]))
        .block(block);
        frame.render_widget(empty, list_area);
        return;
    }

    let inner_width = list_area.width.saturating_sub(4) as usize;
    let size_col = 9usize;
    let perm_col = 9usize;
    let owner_col = 8usize;
    let show_owner = inner_width >= 50;
    let owner_total = if show_owner { (owner_col + 1) * 2 } else { 0 };
    let name_col = inner_width.saturating_sub(size_col + perm_col + 2 + owner_total);

    let items: Vec<ListItem> = app
        .sftp
        .entries
        .iter()
        .map(|e| {
            let (icon, icon_color) = file_icon(&e.name, e.is_dir);
            let icon_style = Style::default().fg(icon_color);
            let name_style = filename_color(&e.name, e.is_dir, e.permissions);

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

            let mut spans = vec![
                Span::styled(icon, icon_style),
                Span::styled(name_truncated, name_style),
                Span::raw(" "),
                Span::styled(size_str, Style::default().fg(styles::green())),
            ];

            if show_owner {
                let owner_raw = app.sftp.resolve_owner(e.uid);
                let group_raw = app.sftp.resolve_group(e.gid);
                let owner_str = format!(
                    " {:<width$}",
                    owner_raw.chars().take(owner_col).collect::<String>(),
                    width = owner_col
                );
                let group_str = format!(
                    " {:<width$}",
                    group_raw.chars().take(owner_col).collect::<String>(),
                    width = owner_col
                );
                spans.push(Span::styled(owner_str, Style::default().fg(styles::muted())));
                spans.push(Span::styled(group_str, Style::default().fg(styles::muted())));
            }

            spans.push(Span::raw(" "));
            spans.push(Span::styled(perms_str, Style::default().fg(styles::muted())));

            let line = Line::from(spans);

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
                Style::default().bg(styles::selection_bg()).fg(styles::fg())
            } else {
                Style::default().fg(styles::fg())
            };
            ListItem::new(Span::styled(format!(" {} ", item.label), style))
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(styles::primary()))
            .style(Style::default().bg(styles::bg())),
    );

    frame.render_widget(list, menu_area);
}

fn render_snippet_overlay(
    frame: &mut Frame,
    overlay: &mut super::snippets::SnippetOverlay,
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
        super::snippets::SnippetMode::Browse => {
            render_snippet_browse(frame, overlay, overlay_rect);
        }
        super::snippets::SnippetMode::Add | super::snippets::SnippetMode::Edit => {
            render_snippet_form(frame, overlay, overlay_rect);
        }
        super::snippets::SnippetMode::ConfirmDelete => {
            render_snippet_delete(frame, overlay, overlay_rect);
        }
    }
}

fn render_snippet_browse(
    frame: &mut Frame,
    overlay: &mut super::snippets::SnippetOverlay,
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
        .border_style(Style::default().fg(styles::primary()))
        .style(Style::default().bg(styles::bg()));
    frame.render_widget(search_block, search_area);

    let list_block = Block::default()
        .borders(Borders::LEFT | Borders::RIGHT | Borders::BOTTOM)
        .border_style(Style::default().fg(styles::primary()))
        .style(Style::default().bg(styles::bg()));
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
            Style::default().fg(styles::muted()),
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
                    .fg(styles::primary())
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(styles::fg())
            };
            let cmd_style = Style::default().fg(styles::green());
            let desc_style = Style::default().fg(styles::muted());
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
        Span::styled(" Enter", Style::default().fg(styles::yellow())),
        Span::raw(":exec"),
        Span::raw("  "),
        Span::styled("a", Style::default().fg(styles::yellow())),
        Span::raw(":add"),
        Span::raw("  "),
        Span::styled("e", Style::default().fg(styles::yellow())),
        Span::raw(":edit"),
        Span::raw("  "),
        Span::styled("d", Style::default().fg(styles::yellow())),
        Span::raw(":del"),
        Span::raw("  "),
        Span::styled("Esc", Style::default().fg(styles::yellow())),
        Span::raw(":close"),
    ]))
    .style(Style::default().bg(styles::bg()));
    frame.render_widget(footer, footer_area);
}

fn render_snippet_form(
    frame: &mut Frame,
    overlay: &super::snippets::SnippetOverlay,
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
        .border_style(Style::default().fg(styles::primary()))
        .style(Style::default().bg(styles::bg()));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let fields = [
        (
            "Name:        ",
            &form.name,
            super::snippets::AddFormField::Name,
        ),
        (
            "Command:     ",
            &form.command,
            super::snippets::AddFormField::Command,
        ),
        (
            "Description: ",
            &form.description,
            super::snippets::AddFormField::Description,
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
            Style::default().fg(styles::primary())
        } else {
            Style::default().fg(styles::muted())
        };
        let cursor = if is_active { "\u{2588}" } else { "" };

        let line = Line::from(vec![
            Span::styled(*label, label_style),
            Span::styled(*value, Style::default().fg(styles::fg())),
            Span::styled(cursor, Style::default().fg(styles::fg())),
        ]);
        frame.render_widget(Paragraph::new(line), field_area);
    }

    let footer_y = inner.y + inner.height.saturating_sub(1);
    let footer_area = Rect::new(inner.x, footer_y, inner.width, 1);
    let footer = Paragraph::new(Line::from(vec![
        Span::styled(" Tab", Style::default().fg(styles::yellow())),
        Span::raw(":next field"),
        Span::raw("  "),
        Span::styled("Enter", Style::default().fg(styles::yellow())),
        Span::raw(":save"),
        Span::raw("  "),
        Span::styled("Esc", Style::default().fg(styles::yellow())),
        Span::raw(":cancel"),
    ]));
    frame.render_widget(footer, footer_area);
}

fn render_snippet_delete(
    frame: &mut Frame,
    overlay: &super::snippets::SnippetOverlay,
    area: Rect,
) {
    let name = overlay
        .selected_snippet()
        .map(|s| s.name.as_str())
        .unwrap_or("?");

    let block = Block::default()
        .title(" Delete Snippet ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(styles::red()))
        .style(Style::default().bg(styles::bg()));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let msg = Paragraph::new(vec![
        Line::from(""),
        Line::from(vec![Span::styled(
            format!("  Delete '{}'?", name),
            Style::default()
                .fg(styles::red())
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::raw("  "),
            Span::styled("y", Style::default().fg(styles::yellow())),
            Span::raw(":yes"),
            Span::raw("  "),
            Span::styled("n", Style::default().fg(styles::yellow())),
            Span::raw(":cancel"),
        ]),
    ]);
    frame.render_widget(msg, inner);
}

fn render_status(frame: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let active_count = app.transfers.active_count().min(3);

    // If there are active transfers, split the area: transfers on top rows, keybindings on bottom row
    let (transfer_area, keys_area) = if active_count > 0 {
        let splits = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(active_count as u16),
                Constraint::Length(1),
            ])
            .split(area);
        (Some(splits[0]), splits[1])
    } else {
        (None, area)
    };

    // Render transfer progress rows
    if let Some(t_area) = transfer_area {
        let active: Vec<&super::transfer::TransferInfo> = app.transfers.active_transfers();
        for (i, info) in active.iter().take(3).enumerate() {
            let row = Rect::new(t_area.x, t_area.y + i as u16, t_area.width, 1);
            render_transfer_row(frame, info, row);
        }
    }

    // Render keybindings / status line
    if app.confirm_delete.is_some() {
        let paragraph = Paragraph::new(Line::from(vec![
            Span::styled(" \u{26a0} ", Style::default().fg(styles::red())),
            Span::styled(&app.status_message, Style::default().fg(styles::red()).add_modifier(Modifier::BOLD)),
        ]))
        .style(Style::default().bg(styles::muted()));
        frame.render_widget(paragraph, keys_area);
        return;
    }

    let mut keys = vec![
        Span::styled(" Ctrl+Q", Style::default().fg(styles::yellow())),
        Span::raw(" quit"),
        Span::raw("  "),
        Span::styled("Ctrl+S", Style::default().fg(styles::yellow())),
        Span::raw(" switch panel"),
        Span::raw("  "),
        Span::styled("Ctrl+B", Style::default().fg(styles::yellow())),
        Span::raw(" toggle SFTP"),
        Span::raw("  "),
        Span::styled("Ctrl+F", Style::default().fg(styles::yellow())),
        Span::raw(" follow dir"),
        Span::raw("  "),
        Span::styled("/", Style::default().fg(styles::yellow())),
        Span::raw(" go to path"),
        Span::raw("  "),
        Span::styled("d", Style::default().fg(styles::yellow())),
        Span::raw(" download"),
        Span::raw("  "),
        Span::styled("u", Style::default().fg(styles::yellow())),
        Span::raw(" upload"),
        Span::raw("  "),
        Span::styled("Ctrl+P", Style::default().fg(styles::yellow())),
        Span::raw(" snippets"),
        Span::raw("  "),
        Span::styled(&app.status_message, Style::default().fg(styles::green())),
    ];

    if app.sftp_follow_terminal {
        keys.push(Span::raw("  "));
        keys.push(Span::styled("[Follow]", Style::default().fg(styles::primary())));
    }

    if active_count > 0 {
        keys.push(Span::raw("  "));
        keys.push(Span::styled("Ctrl+X", Style::default().fg(styles::yellow())));
        keys.push(Span::raw(" cancel transfer"));
    }

    let paragraph = Paragraph::new(Line::from(keys)).style(Style::default().bg(styles::muted()));
    frame.render_widget(paragraph, keys_area);
}

fn render_transfer_row(frame: &mut Frame, info: &super::transfer::TransferInfo, area: Rect) {
    let width = area.width as usize;
    if width < 20 {
        return;
    }

    let direction_icon = match info.direction {
        TransferDirection::Upload => "\u{2191}", // ↑
        TransferDirection::Download => "\u{2193}", // ↓
    };

    let speed = TransferManager::speed_bytes_per_sec(info);
    let speed_str = TransferManager::format_speed(speed);

    let eta_str = match TransferManager::eta_secs(info) {
        Some(s) if s < 60 => format!("ETA {}s", s),
        Some(s) => format!("ETA {}m{}s", s / 60, s % 60),
        None => String::new(),
    };

    let filename_max = 20usize;
    let filename: String = if info.filename.chars().count() > filename_max {
        let t: String = info.filename.chars().take(filename_max - 1).collect();
        format!("{}~", t)
    } else {
        info.filename.clone()
    };

    // Build progress bar (16 chars wide)
    let bar_width = 16usize;
    let pct = if info.total_bytes > 0 {
        (info.transferred_bytes as f64 / info.total_bytes as f64).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let filled = (pct * bar_width as f64) as usize;
    let bar_filled: String = "\u{2588}".repeat(filled);
    let bar_empty: String = "\u{2591}".repeat(bar_width - filled);
    let pct_str = if info.total_bytes > 0 {
        format!("{:3.0}%", pct * 100.0)
    } else {
        "  ??".to_string()
    };

    let mut spans = vec![
        Span::styled(format!(" {} ", direction_icon), Style::default().fg(styles::primary())),
        Span::styled(format!("{:<width$} ", filename, width = filename_max), Style::default().fg(styles::fg())),
        Span::styled(bar_filled, Style::default().fg(styles::primary())),
        Span::styled(bar_empty, Style::default().fg(styles::muted())),
        Span::raw(format!(" {} ", pct_str)),
        Span::styled(speed_str, Style::default().fg(styles::green())),
    ];

    if !eta_str.is_empty() {
        spans.push(Span::raw(format!("  {}", eta_str)));
    }

    let paragraph = Paragraph::new(Line::from(spans))
        .style(Style::default().bg(styles::bg()));
    frame.render_widget(paragraph, area);
}
