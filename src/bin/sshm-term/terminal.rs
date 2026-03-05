use ratatui::{
    buffer::Buffer,
    layout::Rect,
    widgets::Widget,
};
use tui_term::widget::PseudoTerminal;
use vt100::Parser;

pub struct TerminalPanel {
    parser: Parser,
    pub detected_cwd: Option<String>,
}

impl TerminalPanel {
    pub fn new(cols: u16, rows: u16) -> Self {
        Self {
            parser: Parser::new(rows, cols, 0),
            detected_cwd: None,
        }
    }

    pub fn process_output(&mut self, bytes: &[u8]) {
        self.detect_osc7(bytes);
        self.parser.process(bytes);
    }

    fn detect_osc7(&mut self, bytes: &[u8]) {
        let data = match std::str::from_utf8(bytes) {
            Ok(s) => s,
            Err(_) => return,
        };

        for part in data.split("\x1b]7;") {
            let end = match part.find('\x07').or_else(|| part.find("\x1b\\")) {
                Some(pos) => pos,
                None => continue,
            };

            let uri = &part[..end];

            if let Some(path_with_host) = uri.strip_prefix("file://") {
                if path_with_host.is_empty() {
                    continue;
                }
                // file://hostname/path — skip hostname component to get /path
                let path = if path_with_host.starts_with('/') {
                    path_with_host
                } else if let Some(slash_pos) = path_with_host.find('/') {
                    &path_with_host[slash_pos..]
                } else {
                    continue
                };
                if !path.is_empty() {
                    self.detected_cwd = Some(path.to_string());
                }
            } else if uri.starts_with('/') {
                self.detected_cwd = Some(uri.to_string());
            }
        }
    }

    pub fn resize(&mut self, cols: u16, rows: u16) {
        self.parser.set_size(rows, cols);
    }

    pub fn screen(&self) -> &vt100::Screen {
        self.parser.screen()
    }
}

pub struct TerminalPanelWidget<'a> {
    panel: &'a TerminalPanel,
}

impl<'a> TerminalPanelWidget<'a> {
    pub fn new(panel: &'a TerminalPanel) -> Self {
        Self { panel }
    }
}

impl Widget for TerminalPanelWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        PseudoTerminal::new(self.panel.screen()).render(area, buf);
    }
}
