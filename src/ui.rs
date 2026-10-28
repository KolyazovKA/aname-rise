// маленькие общие виджеты: подпись, иконка, переключатель, ползунок
use amane::{Center, Color, Column, Full, Padding, Parent, Point, Pointer, Rectangle, Row, Stack,
    Start, Text, Weight, children};

use crate::state::Ui;
use crate::theme::Theme;

pub fn label(text: impl Into<String>, size: f32, color: Color) -> Text {
    Text::new(text).size(size).color(color)
}

pub fn bold(text: impl Into<String>, size: f32, color: Color) -> Text {
    Text::new(text).size(size).color(color).weight(Weight::Bold)
}

pub fn icon(glyph: &str, size: f32, color: Color) -> Text {
    Text::new(glyph).size(size).color(color).tight()
}

pub fn pad(all: f32) -> Padding {
    Padding { top: all, right: all, bottom: all, left: all }
}

// цвет кнопки: обычный или под курсором
pub fn fill_for(ui: &Ui, key: &str, base: Color, hover: Color) -> Color {
    if ui.hovered.as_deref() == Some(key) { hover } else { base }
}

pub const SWITCH_WIDTH: f32 = 48.0;
const SWITCH_HEIGHT: f32 = 28.0;

pub fn switch(theme: &Theme, on: bool, toggle: impl Fn() + 'static) -> Rectangle {
    let knob = if on { 22.0 } else { 16.0 };
    let x = if on { SWITCH_WIDTH - knob - 3.0 } else { 6.0 };

    let (fill, knob_color) = if on {
        (theme.accent, theme.on_accent)
    } else {
        (theme.hover, theme.muted)
    };

    let track = Rectangle::new()
        .width(SWITCH_WIDTH)
        .height(SWITCH_HEIGHT)
        .radius(Full)
        .fill(fill)
        .cursor(Pointer)
        .on_click(move |_| toggle())
        .child(
            Rectangle::new()
                .width(knob)
                .height(knob)
                .radius(Full)
                .fill(knob_color)
                .translate(x, (SWITCH_HEIGHT - knob) / 2.0),
        );

    if on { track } else { track.border(2.0, theme.border) }
}

pub const HEADER_HEIGHT: f32 = 58.0;

// шапка окна bluetooth/wifi: иконка, название, статус и выключатель
pub fn header(
    theme: &Theme,
    glyph: &str,
    title: &str,
    status: &str,
    status_color: Color,
    on: bool,
    width: f32,
    toggle: impl Fn() + 'static,
) -> Rectangle {
    let badge = Rectangle::new()
        .width(36.0)
        .height(36.0)
        .radius(12.0)
        .fill(if on { theme.accent } else { theme.border })
        .align_child(Center, Center)
        .child(icon(glyph, 18.0, theme.on_accent));

    let text_width = width - 20.0 - 36.0 - SWITCH_WIDTH - 20.0;

    let status = Rectangle::new()
        .width(text_width)
        .height(16.0)
        .child(label(status, 11.0, status_color).elide());

    let text = Rectangle::new()
        .width(text_width)
        .height(36.0)
        .align_child(Start, Center)
        .child(Column::new(children![bold(title, 14.0, theme.text), status]));

    Rectangle::new()
        .width(width)
        .height(HEADER_HEIGHT)
        .radius(12.0)
        .fill(theme.surface)
        .padding(pad(10.0))
        .child(
            Row::new(children![badge, text, switch(theme, on, toggle)])
                .gap(10.0)
                .align(Center),
        )
}

pub const SLIDER_HEIGHT: f32 = 22.0;

// ползунок 0..100: нажатие и перетаскивание задают значение
pub fn slider(
    theme: &Theme,
    width: f32,
    value: u8,
    color: Color,
    change: impl Fn(u8) + 'static,
) -> Rectangle {
    let filled = width * f32::from(value.min(100)) / 100.0;

    let track = Rectangle::new()
        .width(width)
        .height(6.0)
        .radius(3.0)
        .fill(theme.border)
        .translate(0.0, (SLIDER_HEIGHT - 6.0) / 2.0)
        .child(Rectangle::new().width(filled.max(0.5)).height(6.0).radius(3.0).fill(color));

    let knob = Rectangle::new()
        .width(14.0)
        .height(14.0)
        .radius(Full)
        .fill(color)
        .translate((filled - 7.0).clamp(0.0, width - 14.0), (SLIDER_HEIGHT - 14.0) / 2.0);

    Rectangle::new()
        .width(width)
        .height(SLIDER_HEIGHT)
        .cursor(Pointer)
        .on_drag(move |point: Point| {
            let value = (point.x / width * 100.0).clamp(0.0, 100.0).round() as u8;

            change(value);
        })
        .child(Stack::new(children![track, knob]).width(Parent).height(Parent))
}

// квадратная кнопка с иконкой
pub fn icon_button(
    ui: &Ui,
    theme: &Theme,
    key: &str,
    glyph: &str,
    size: f32,
    color: Color,
) -> Rectangle {
    let fill = fill_for(ui, key, Color::TRANSPARENT, theme.hover);
    let name = key.to_string();

    Rectangle::new()
        .width(size)
        .height(size)
        .radius(8.0)
        .fill(fill)
        .cursor(Pointer)
        .on_hover(move |inside| Ui::set_hover(&name, inside))
        .align_child(Center, Center)
        .child(icon(glyph, 16.0, color))
}
