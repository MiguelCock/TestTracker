use gpui_kit::component::input::{Input, InputState, Textarea, TextareaState};
use gpui_kit::gpui::{
    AnyElement, App, Div, ElementId, Entity, FontWeight, InteractiveElement as _, ParentElement,
    SharedString, Stateful, Styled, div, px, rgb,
};

use crate::theme::*;

pub fn primary_btn(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Stateful<Div> {
    let label: SharedString = label.into();
    div()
        .id(id)
        .flex()
        .items_center()
        .justify_center()
        .gap_2()
        .px_3()
        .py_1p5()
        .rounded_md()
        .bg(rgb(0x059669))
        .border_1()
        .border_color(rgb(0x10b981))
        .text_color(rgb(0xffffff))
        .text_sm()
        .font_weight(FontWeight::MEDIUM)
        .cursor_pointer()
        .hover(|style| style.bg(rgb(0x10b981)))
        .child(label)
}

pub fn danger_btn(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Stateful<Div> {
    let label: SharedString = label.into();
    div()
        .id(id)
        .flex()
        .items_center()
        .justify_center()
        .gap_2()
        .px_3()
        .py_1p5()
        .rounded_md()
        .bg(rgb(0x3f1515))
        .border_1()
        .border_color(rgb(0x7f1d1d))
        .text_color(rgb(0xfecaca))
        .text_sm()
        .cursor_pointer()
        .hover(|style| style.bg(rgb(0x7f1d1d)))
        .child(label)
}

pub fn ghost_btn(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Stateful<Div> {
    let label: SharedString = label.into();
    div()
        .id(id)
        .flex()
        .items_center()
        .justify_center()
        .gap_2()
        .px_2()
        .py_1()
        .rounded_md()
        .text_color(rgb(TEXT_MUTED))
        .text_xs()
        .cursor_pointer()
        .hover(|style| style.bg(rgb(PANEL_ALT)).text_color(rgb(TEXT)))
        .child(label)
}

pub fn badge(text: impl Into<SharedString>, foreground: u32, background: u32) -> Div {
    div()
        .flex()
        .items_center()
        .px_2()
        .py_0p5()
        .rounded_full()
        .bg(rgb(background))
        .text_color(rgb(foreground))
        .text_xs()
        .font_weight(FontWeight::MEDIUM)
        .child(text.into())
}

pub fn card() -> Div {
    panel().p_4()
}

pub fn field_label(label: impl Into<SharedString>) -> Div {
    div()
        .text_xs()
        .font_weight(FontWeight::MEDIUM)
        .text_color(rgb(TEXT_MUTED))
        .child(label.into())
}

pub fn text_input(state: &Entity<InputState>, _placeholder: impl Into<SharedString>) -> Input {
    Input::new(state).w_full().cleanable(true)
}

pub fn password_input(state: &Entity<InputState>, placeholder: impl Into<SharedString>) -> Input {
    Input::new(state)
        .w_full()
        .mask_toggle()
        .content_type(gpui_kit::component::input::InputContentType::Password)
        .aria_label(placeholder.into())
}

pub fn textarea(state: &Entity<TextareaState>, height: f32) -> Textarea {
    Textarea::new(state).w_full().h(px(height))
}

pub fn labeled(
    label: impl Into<SharedString>,
    state: &Entity<InputState>,
    placeholder: impl Into<SharedString>,
) -> Div {
    div()
        .flex()
        .flex_col()
        .gap_1p5()
        .w_full()
        .child(field_label(label))
        .child(text_input(state, placeholder))
}

pub fn labeled_area(
    label: impl Into<SharedString>,
    state: &Entity<TextareaState>,
    height: f32,
) -> Div {
    div()
        .flex()
        .flex_col()
        .gap_1p5()
        .w_full()
        .child(field_label(label))
        .child(textarea(state, height))
}

pub fn pill(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    active: bool,
) -> Stateful<Div> {
    let label: SharedString = label.into();
    div()
        .id(id)
        .px_2p5()
        .py_1()
        .rounded_full()
        .border_1()
        .border_color(if active { rgb(0x10b981) } else { rgb(BORDER) })
        .bg(if active { rgb(0x052e2b) } else { rgb(PANEL) })
        .text_color(if active {
            rgb(0x6ee7b7)
        } else {
            rgb(TEXT_MUTED)
        })
        .text_xs()
        .cursor_pointer()
        .hover(|style| style.border_color(rgb(0x334155)))
        .child(label)
}

pub fn empty_state(title: impl Into<SharedString>, description: impl Into<SharedString>) -> Div {
    div()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap_2()
        .p_8()
        .w_full()
        .rounded_lg()
        .border_1()
        .border_color(rgb(BORDER_SOFT))
        .bg(rgb(PANEL))
        .child(
            div()
                .text_sm()
                .font_weight(FontWeight::MEDIUM)
                .text_color(rgb(TEXT))
                .child(title.into()),
        )
        .child(
            div()
                .text_xs()
                .text_color(rgb(TEXT_MUTED))
                .child(description.into()),
        )
}

pub fn error_banner(message: impl Into<SharedString>) -> Div {
    div()
        .w_full()
        .px_3()
        .py_2()
        .rounded_md()
        .border_1()
        .border_color(rgb(0x7f1d1d))
        .bg(rgb(0x2a0f12))
        .text_color(rgb(0xfecaca))
        .text_xs()
        .child(message.into())
}

pub fn success_banner(message: impl Into<SharedString>) -> Div {
    div()
        .w_full()
        .px_3()
        .py_2()
        .rounded_md()
        .border_1()
        .border_color(rgb(0x065f46))
        .bg(rgb(0x052e2b))
        .text_color(rgb(0x6ee7b7))
        .text_xs()
        .child(message.into())
}

pub fn loading(label: impl Into<SharedString>) -> Div {
    div()
        .flex()
        .items_center()
        .gap_2()
        .text_xs()
        .text_color(rgb(TEXT_MUTED))
        .child(label.into())
}

pub fn modal_backdrop() -> Div {
    div()
        .absolute()
        .top_0()
        .left_0()
        .size_full()
        .bg(overlay())
        .flex()
        .items_center()
        .justify_center()
        .p_6()
}

pub fn modal_card(width: f32) -> Div {
    div()
        .w(px(width))
        .max_h_full()
        .flex()
        .flex_col()
        .bg(rgb(PANEL))
        .border_1()
        .border_color(rgb(BORDER))
        .rounded_xl()
        .overflow_hidden()
}

pub fn modal_header(title: impl Into<SharedString>, subtitle: Option<SharedString>) -> Div {
    let mut header = div()
        .flex()
        .flex_col()
        .gap_1()
        .px_5()
        .py_4()
        .border_b_1()
        .border_color(rgb(BORDER))
        .child(h2().child(title.into()));
    if let Some(subtitle) = subtitle {
        header = header.child(muted().child(subtitle));
    }
    header
}

pub fn modal_footer(children: impl IntoIterator<Item = AnyElement>) -> Div {
    div()
        .flex()
        .items_center()
        .justify_end()
        .gap_2()
        .px_5()
        .py_4()
        .border_t_1()
        .border_color(rgb(BORDER))
        .children(children)
}

pub fn spacer() -> Div {
    div().flex_1()
}

pub fn hr() -> Div {
    div().w_full().h_px().bg(rgb(BORDER))
}

pub fn input_value(state: &Entity<InputState>, cx: &App) -> String {
    state.read(cx).value().to_string()
}

pub fn textarea_value(state: &Entity<TextareaState>, cx: &App) -> String {
    state.read(cx).value().to_string()
}
