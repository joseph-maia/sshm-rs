use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph};
use ratatui::Frame;

use crate::ui::styles;

pub fn draw(f: &mut Frame, area: Rect) {
    let popup_width = 64u16.min(area.width.saturating_sub(4));
    let popup_height = 30u16.min(area.height.saturating_sub(4));
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
                .fg(styles::PRIMARY)
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
            Span::styled("  End / G  ", key_style),
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
            Span::styled("  i        ", key_style),
            Span::styled("Show host info", desc_style),
        ]),
        Line::from(vec![
            Span::styled("  s        ", key_style),
            Span::styled("Toggle sort mode", desc_style),
        ]),
        Line::from(vec![
            Span::styled("  t        ", key_style),
            Span::styled("Toggle tag sidebar", desc_style),
        ]),
        Line::from(vec![
            Span::styled("  r        ", key_style),
            Span::styled("Refresh connectivity status", desc_style),
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
                .style(Style::default().bg(styles::BG).fg(styles::FG)),
        );
    f.render_widget(paragraph, popup_area);
}
