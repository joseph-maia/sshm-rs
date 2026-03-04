#![allow(dead_code)]
use ratatui::style::{Color, Modifier, Style};
use std::sync::OnceLock;

use crate::theme::Theme;

// ---------------------------------------------------------------------------
// Global theme store — initialised once at startup via `init_theme`.
// ---------------------------------------------------------------------------

static THEME: OnceLock<Theme> = OnceLock::new();

/// Call this once (before any drawing code) to install the active theme.
/// Subsequent calls are silently ignored (OnceLock semantics).
pub fn init_theme(theme: Theme) {
    let _ = THEME.set(theme);
}

/// Replace the active theme at runtime (e.g. when the user presses T).
/// Because OnceLock cannot be reset we store the mutable fallback in a
/// separate `static`.  We use an `std::sync::RwLock` for the dynamic slot.
static DYNAMIC_THEME: std::sync::OnceLock<std::sync::RwLock<Option<Theme>>> =
    std::sync::OnceLock::new();

fn dynamic_store() -> &'static std::sync::RwLock<Option<Theme>> {
    DYNAMIC_THEME.get_or_init(|| std::sync::RwLock::new(None))
}

/// Overwrite the active theme without restarting the process.
pub fn set_theme(theme: Theme) {
    if let Ok(mut guard) = dynamic_store().write() {
        *guard = Some(theme);
    }
}

/// Return a clone of the currently active theme.
fn theme() -> Theme {
    // Dynamic override takes priority.
    if let Ok(guard) = dynamic_store().read() {
        if let Some(ref t) = *guard {
            return t.clone();
        }
    }
    // Fall back to the OnceLock value (set at startup).
    THEME.get().cloned().unwrap_or_else(Theme::tokyo_night)
}

// ---------------------------------------------------------------------------
// Colour accessors — replace the old `pub const` values.
// ---------------------------------------------------------------------------

pub fn bg() -> Color {
    let t = theme();
    Color::Rgb(t.bg[0], t.bg[1], t.bg[2])
}

pub fn fg() -> Color {
    let t = theme();
    Color::Rgb(t.fg[0], t.fg[1], t.fg[2])
}

pub fn primary() -> Color {
    let t = theme();
    Color::Rgb(t.primary[0], t.primary[1], t.primary[2])
}

pub fn green() -> Color {
    let t = theme();
    Color::Rgb(t.green[0], t.green[1], t.green[2])
}

pub fn red() -> Color {
    let t = theme();
    Color::Rgb(t.red[0], t.red[1], t.red[2])
}

pub fn yellow() -> Color {
    let t = theme();
    Color::Rgb(t.yellow[0], t.yellow[1], t.yellow[2])
}

pub fn muted() -> Color {
    let t = theme();
    Color::Rgb(t.muted[0], t.muted[1], t.muted[2])
}

pub fn cyan() -> Color {
    let t = theme();
    Color::Rgb(t.cyan[0], t.cyan[1], t.cyan[2])
}

pub fn purple() -> Color {
    let t = theme();
    Color::Rgb(t.purple[0], t.purple[1], t.purple[2])
}

pub fn orange() -> Color {
    let t = theme();
    Color::Rgb(t.orange[0], t.orange[1], t.orange[2])
}

pub fn selection_bg() -> Color {
    let t = theme();
    Color::Rgb(t.selection_bg[0], t.selection_bg[1], t.selection_bg[2])
}

// ---------------------------------------------------------------------------
// Composite style helpers (unchanged API, now use functions above).
// ---------------------------------------------------------------------------

pub fn header_style() -> Style {
    Style::default().fg(primary()).add_modifier(Modifier::BOLD)
}

pub fn table_header_style() -> Style {
    Style::default()
        .fg(primary())
        .add_modifier(Modifier::BOLD)
        .add_modifier(Modifier::UNDERLINED)
}

pub fn table_row_style() -> Style {
    Style::default().fg(fg()).bg(bg())
}

pub fn table_selected_style() -> Style {
    Style::default()
        .fg(fg())
        .bg(selection_bg())
        .add_modifier(Modifier::BOLD)
}

pub fn multi_selected_style() -> Style {
    Style::default()
        .fg(cyan())
        .bg(Color::Rgb(0x1e, 0x2a, 0x3a))
}

pub fn search_focused_style() -> Style {
    Style::default().fg(primary())
}

pub fn search_unfocused_style() -> Style {
    Style::default().fg(muted())
}

pub fn help_text_style() -> Style {
    Style::default().fg(muted())
}

pub fn status_online_style() -> Style {
    Style::default().fg(green())
}

pub fn status_offline_style() -> Style {
    Style::default().fg(red())
}

pub fn status_unknown_style() -> Style {
    Style::default().fg(muted())
}

pub fn status_connecting_style() -> Style {
    Style::default().fg(yellow())
}

pub fn border_focused_style() -> Style {
    Style::default().fg(primary())
}

pub fn border_unfocused_style() -> Style {
    Style::default().fg(muted())
}

pub fn delete_title_style() -> Style {
    Style::default().fg(red()).add_modifier(Modifier::BOLD)
}

pub fn delete_warning_style() -> Style {
    Style::default().fg(red())
}

pub fn help_key_style() -> Style {
    Style::default().fg(primary()).add_modifier(Modifier::BOLD)
}

pub fn help_desc_style() -> Style {
    Style::default().fg(fg())
}

pub fn help_section_style() -> Style {
    Style::default()
        .fg(primary())
        .add_modifier(Modifier::BOLD)
        .add_modifier(Modifier::UNDERLINED)
}
