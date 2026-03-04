#![allow(dead_code)]
use ratatui::style::{Color, Modifier, Style};

// Tokyo Night color palette
pub const BG: Color = Color::Rgb(0x1a, 0x1b, 0x26);
pub const FG: Color = Color::Rgb(0xc0, 0xca, 0xf5);
pub const PRIMARY: Color = Color::Rgb(0x7a, 0xa2, 0xf7); // blue
pub const GREEN: Color = Color::Rgb(0x9e, 0xce, 0x6a);
pub const RED: Color = Color::Rgb(0xf7, 0x76, 0x8e);
pub const YELLOW: Color = Color::Rgb(0xe0, 0xaf, 0x68);
pub const MUTED: Color = Color::Rgb(0x73, 0x7a, 0xa2);
pub const CYAN: Color = Color::Rgb(0x7d, 0xcf, 0xff);
pub const PURPLE: Color = Color::Rgb(0xbb, 0x9a, 0xf7);
pub const ORANGE: Color = Color::Rgb(0xff, 0x9e, 0x64);
pub const SELECTION_BG: Color = Color::Rgb(0x36, 0x4a, 0x82);

pub fn header_style() -> Style {
    Style::default().fg(PRIMARY).add_modifier(Modifier::BOLD)
}

pub fn table_header_style() -> Style {
    Style::default()
        .fg(PRIMARY)
        .add_modifier(Modifier::BOLD)
        .add_modifier(Modifier::UNDERLINED)
}

pub fn table_row_style() -> Style {
    Style::default().fg(FG).bg(BG)
}

pub fn table_selected_style() -> Style {
    Style::default()
        .fg(FG)
        .bg(SELECTION_BG)
        .add_modifier(Modifier::BOLD)
}

pub fn search_focused_style() -> Style {
    Style::default().fg(PRIMARY)
}

pub fn search_unfocused_style() -> Style {
    Style::default().fg(MUTED)
}

pub fn help_text_style() -> Style {
    Style::default().fg(MUTED)
}

pub fn status_online_style() -> Style {
    Style::default().fg(GREEN)
}

pub fn status_offline_style() -> Style {
    Style::default().fg(RED)
}

pub fn status_unknown_style() -> Style {
    Style::default().fg(MUTED)
}

pub fn status_connecting_style() -> Style {
    Style::default().fg(YELLOW)
}

pub fn border_focused_style() -> Style {
    Style::default().fg(PRIMARY)
}

pub fn border_unfocused_style() -> Style {
    Style::default().fg(MUTED)
}

pub fn delete_title_style() -> Style {
    Style::default().fg(RED).add_modifier(Modifier::BOLD)
}

pub fn delete_warning_style() -> Style {
    Style::default().fg(RED)
}

pub fn help_key_style() -> Style {
    Style::default().fg(PRIMARY).add_modifier(Modifier::BOLD)
}

pub fn help_desc_style() -> Style {
    Style::default().fg(FG)
}

pub fn help_section_style() -> Style {
    Style::default()
        .fg(PRIMARY)
        .add_modifier(Modifier::BOLD)
        .add_modifier(Modifier::UNDERLINED)
}
