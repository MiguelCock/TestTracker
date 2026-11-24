use gpui_kit::Styled;
use gpui_kit::gpui::{Div, FontWeight, Hsla, div, hsla, rgb};

pub const BG: u32 = 0x020617;
pub const PANEL: u32 = 0x0f172a;
pub const PANEL_ALT: u32 = 0x111c33;
pub const BORDER: u32 = 0x1e293b;
pub const BORDER_SOFT: u32 = 0x172033;
pub const TEXT: u32 = 0xe2e8f0;
pub const TEXT_MUTED: u32 = 0x94a3b8;
pub const TEXT_DIM: u32 = 0x64748b;
pub const ACCENT: u32 = 0x10b981;
pub const ACCENT_SOFT: u32 = 0x34d399;
pub const WARNING: u32 = 0xfbbf24;
pub const DANGER: u32 = 0xf87171;
pub const SEVERITY_INFO: u32 = 0x94a3b8;
pub const SEVERITY_LOW: u32 = 0x38bdf8;
pub const SEVERITY_MEDIUM: u32 = 0xfbbf24;
pub const SEVERITY_HIGH: u32 = 0xfb923c;
pub const SEVERITY_CRITICAL: u32 = 0xf87171;

pub fn overlay() -> Hsla {
    hsla(0.0, 0.0, 0.0, 0.62)
}

pub fn panel() -> Div {
    div()
        .bg(rgb(PANEL))
        .border_1()
        .border_color(rgb(BORDER))
        .rounded_lg()
}

pub fn page() -> Div {
    div()
        .flex()
        .flex_col()
        .size_full()
        .bg(rgb(BG))
        .text_color(rgb(TEXT))
}

pub fn h1() -> Div {
    div()
        .text_2xl()
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(rgb(0xffffff))
}

pub fn h2() -> Div {
    div()
        .text_lg()
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(rgb(0xffffff))
}

pub fn h3() -> Div {
    div()
        .text_sm()
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(rgb(0xffffff))
}

pub fn muted() -> Div {
    div().text_xs().text_color(rgb(TEXT_MUTED))
}

pub fn dim() -> Div {
    div().text_xs().text_color(rgb(TEXT_DIM))
}

pub fn metric_value() -> Div {
    div()
        .text_3xl()
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(rgb(0xffffff))
}

pub fn severity_color(value: testtracker_core::domain::enums::Severity) -> u32 {
    use testtracker_core::domain::enums::Severity::*;
    match value {
        Informational => SEVERITY_INFO,
        Low => SEVERITY_LOW,
        Medium => SEVERITY_MEDIUM,
        High => SEVERITY_HIGH,
        Critical => SEVERITY_CRITICAL,
    }
}

pub fn status_tone(status: testtracker_core::domain::enums::EngagementStatus) -> (u32, u32) {
    use testtracker_core::domain::enums::EngagementStatus::*;
    match status {
        Draft => (0x64748b, 0x0f172a),
        Active => (0x34d399, 0x052e2b),
        InReview => (0xfbbf24, 0x3b2f0b),
        Completed => (0x60a5fa, 0x0c2544),
        Archived => (0x94a3b8, 0x1e293b),
    }
}
