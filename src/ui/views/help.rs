use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph};
use ratatui::Frame;

use crate::ui::styles;

pub fn draw(f: &mut Frame, area: Rect) {
    let popup_width = 68u16.min(area.width.saturating_sub(4));
    let popup_height = 42u16.min(area.height.saturating_sub(4));
    let x = (area.width.saturating_sub(popup_width)) / 2;
    let y = (area.height.saturating_sub(popup_height)) / 2;
    let popup_area = Rect::new(x, y, popup_width, popup_height);

    f.render_widget(Clear, popup_area);

    let key_style = styles::help_key_style();
    let desc_style = styles::help_desc_style();
    let section_style = styles::help_section_style();

    let lines = vec![
        Line::from(Span::styled(
            "sshm-rs - Keybindings",
            Style::default()
                .fg(styles::primary())
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        // Navigation
        Line::from(Span::styled("Navigation & Connection", section_style)),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Enter    ", key_style),
            Span::styled("Connect to selected host", desc_style),
        ]),
        Line::from(vec![
            Span::styled("  j / Down ", key_style),
            Span::styled("Move down", desc_style),
        ]),
        Line::from(vec![
            Span::styled("  k / Up   ", key_style),
            Span::styled("Move up", desc_style),
        ]),
        Line::from(vec![
            Span::styled("  PgUp     ", key_style),
            Span::styled("Page up", desc_style),
        ]),
        Line::from(vec![
            Span::styled("  PgDn     ", key_style),
            Span::styled("Page down", desc_style),
        ]),
        Line::from(vec![
            Span::styled("  Home     ", key_style),
            Span::styled("Jump to first host", desc_style),
        ]),
        Line::from(vec![
            Span::styled("  End      ", key_style),
            Span::styled("Jump to last host", desc_style),
        ]),
        Line::from(vec![
            Span::styled("  /        ", key_style),
            Span::styled("Search hosts", desc_style),
        ]),
        Line::from(vec![
            Span::styled("  Tab      ", key_style),
            Span::styled("Switch focus (search <-> table)", desc_style),
        ]),
        Line::from(""),
        // Multi-select
        Line::from(Span::styled("Multi-Select", section_style)),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Space    ", key_style),
            Span::styled("Toggle select host, move down", desc_style),
        ]),
        Line::from(vec![
            Span::styled("  Ctrl+a   ", key_style),
            Span::styled("Select all visible hosts", desc_style),
        ]),
        Line::from(vec![
            Span::styled("  d        ", key_style),
            Span::styled("Delete all selected hosts (when selection active)", desc_style),
        ]),
        Line::from(vec![
            Span::styled("  b        ", key_style),
            Span::styled("Broadcast command to selected hosts", desc_style),
        ]),
        Line::from(vec![
            Span::styled("  Esc      ", key_style),
            Span::styled("Clear selection", desc_style),
        ]),
        Line::from(""),
        // Host management
        Line::from(Span::styled("Host Management", section_style)),
        Line::from(""),
        Line::from(vec![
            Span::styled("  a        ", key_style),
            Span::styled("Add new host", desc_style),
        ]),
        Line::from(vec![
            Span::styled("  e        ", key_style),
            Span::styled("Edit selected host", desc_style),
        ]),
        Line::from(vec![
            Span::styled("  d        ", key_style),
            Span::styled("Delete selected host", desc_style),
        ]),
        Line::from(vec![
            Span::styled("  p        ", key_style),
            Span::styled("Set/remove password for host", desc_style),
        ]),
        Line::from(vec![
            Span::styled("  f        ", key_style),
            Span::styled("Toggle favorite", desc_style),
        ]),
        Line::from(vec![
            Span::styled("  F        ", key_style),
            Span::styled("Port forwarding", desc_style),
        ]),
        Line::from(vec![
            Span::styled("  i        ", key_style),
            Span::styled("Show host info", desc_style),
        ]),
        Line::from(vec![
            Span::styled("  s        ", key_style),
            Span::styled("Toggle sort mode", desc_style),
        ]),
        Line::from(vec![
            Span::styled("  y        ", key_style),
            Span::styled("Copy host to clipboard", desc_style),
        ]),
        Line::from(vec![
            Span::styled("  t        ", key_style),
            Span::styled("Toggle tag sidebar", desc_style),
        ]),
        Line::from(vec![
            Span::styled("  r        ", key_style),
            Span::styled("Refresh connectivity status", desc_style),
        ]),
        Line::from(vec![
            Span::styled("  T        ", key_style),
            Span::styled("Cycle color theme", desc_style),
        ]),
        Line::from(vec![
            Span::styled("  S        ", key_style),
            Span::styled("Command snippets", desc_style),
        ]),
        Line::from(vec![
            Span::styled("  G        ", key_style),
            Span::styled("Create group", desc_style),
        ]),
        Line::from(vec![
            Span::styled("  g        ", key_style),
            Span::styled("Assign host to group", desc_style),
        ]),
        Line::from(vec![
            Span::styled("  Enter    ", key_style),
            Span::styled("Collapse/expand group (on group header)", desc_style),
        ]),
        Line::from(""),
        // File Transfer
        Line::from(Span::styled("File Transfer", section_style)),
        Line::from(""),
        Line::from(vec![
            Span::styled("  x        ", key_style),
            Span::styled("Quick SFTP session", desc_style),
        ]),
        Line::from(vec![
            Span::styled("  X        ", key_style),
            Span::styled("SCP file transfer", desc_style),
        ]),
        Line::from(vec![
            Span::styled("  W        ", key_style),
            Span::styled("Open with sshm-term (terminal + SFTP)", desc_style),
        ]),
        Line::from(""),
        // System
        Line::from(Span::styled("System", section_style)),
        Line::from(""),
        Line::from(vec![
            Span::styled("  ?/h      ", key_style),
            Span::styled("Show this help", desc_style),
        ]),
        Line::from(vec![
            Span::styled("  q        ", key_style),
            Span::styled("Quit", desc_style),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "Press Esc, h, ?, or Enter to close",
            styles::help_text_style(),
        )),
    ];

    let paragraph = Paragraph::new(lines)
        .alignment(Alignment::Left)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(styles::border_focused_style())
                .style(Style::default().bg(styles::bg()).fg(styles::fg())),
        );
    f.render_widget(paragraph, popup_area);
}
