use amane::{
    Audio, Center, Column, End, Pointer, Rectangle, Row, ScrollArea, Service, Start, Text, Widget,
    children,
};

use crate::popup::{frame, inner};
use crate::state::{Mixer, Sink, Stream, Ui};
use crate::theme::Theme;
use crate::ui::{bold, icon, icon_button, label, slider};

const GAP: f32 = 6.0;
const SECTION_HEIGHT: f32 = 22.0;
const SINK_HEIGHT: f32 = 38.0;
const APP_HEIGHT: f32 = 58.0;
const PERCENT_WIDTH: f32 = 44.0;

const VOLUME_HIGH: &str = "\u{f057e}";
const VOLUME_OFF: &str = "\u{f0581}";

pub fn view(theme: &Theme, ui: &Ui, width: f32, height: f32) -> Rectangle {
    let inner_width = inner(width);

    let audio = Audio::read();
    let mixer = Mixer::read();

    // общая громкость: кнопка «без звука», ползунок и значение
    let volume = ui.slider_value("master", audio.volume());
    let muted = audio.muted();

    let mute = icon_button(
        ui,
        theme,
        "audio:mute",
        if muted { VOLUME_OFF } else { VOLUME_HIGH },
        36.0,
        if muted { theme.red } else { theme.text },
    )
    .on_click(|_| Audio::toggle_mute());

    let track = slider(
        theme,
        inner_width - 36.0 - PERCENT_WIDTH - 16.0,
        volume,
        if muted { theme.dim } else { theme.accent },
        |value| {
            if Ui::drag("master", value) {
                Audio::set_volume(value);
            }
        },
    );

    let percent = Rectangle::new()
        .width(PERCENT_WIDTH)
        .height(36.0)
        .align_child(End, Center)
        .child(bold(format!("{volume}%"), 14.0, theme.text));

    let master = Row::new(children![mute, track, percent]).gap(8.0).align(Center).height(36.0);

    // всё остальное прокручивается
    let mut parts: Vec<Box<dyn Widget>> = Vec::new();
    let mut total = 0.0;

    add(&mut parts, &mut total, section(theme, "Вывод звука", inner_width), SECTION_HEIGHT);

    if mixer.sinks.is_empty() {
        add(&mut parts, &mut total, note(theme, "Устройства не найдены", inner_width), 30.0);
    }

    for sink in &mixer.sinks {
        add(&mut parts, &mut total, sink_row(theme, ui, sink, &mixer.default, inner_width), SINK_HEIGHT);
    }

    add(&mut parts, &mut total, section(theme, "Приложения", inner_width), SECTION_HEIGHT + 6.0);

    if mixer.streams.is_empty() {
        add(&mut parts, &mut total, note(theme, "Сейчас ничего не играет", inner_width), 30.0);
    }

    for stream in &mixer.streams {
        add(&mut parts, &mut total, app_row(theme, ui, stream, inner_width), APP_HEIGHT);
    }

    let body_height = height - 26.0 - 36.0 - 10.0;

    let body = Rectangle::new()
        .width(inner_width)
        .height(body_height)
        .clip()
        .child(ScrollArea::new(
            "audio-body",
            Column::new(parts).width(inner_width).height(total).gap(GAP),
        ));

    frame(theme, width, height, Column::new(children![master, body]).gap(10.0))
}

fn add(parts: &mut Vec<Box<dyn Widget>>, total: &mut f32, widget: impl Widget + 'static, height: f32) {
    if !parts.is_empty() {
        *total += GAP;
    }

    *total += height;

    parts.push(Box::new(widget));
}

fn section(theme: &Theme, title: &str, width: f32) -> Rectangle {
    Rectangle::new()
        .width(width)
        .height(SECTION_HEIGHT)
        .align_child(Start, End)
        .child(bold(title, 12.0, theme.muted))
}

fn note(theme: &Theme, text: &str, width: f32) -> Rectangle {
    Rectangle::new()
        .width(width)
        .height(30.0)
        .align_child(Center, Center)
        .child(label(text, 12.0, theme.dim))
}

// устройство вывода: нажатие делает его основным и переносит на него играющие приложения
fn sink_row(theme: &Theme, ui: &Ui, sink: &Sink, default: &str, width: f32) -> Rectangle {
    let active = sink.name == default;

    let key = format!("sink:{}", sink.name);

    let fill = if active || ui.hovered.as_deref() == Some(key.as_str()) { theme.hover } else { theme.surface };

    let lower = sink.description.to_lowercase();

    let glyph = if sink.name.contains("bluez") {
        "\u{f00b0}"
    } else if lower.contains("headphone") || lower.contains("headset") {
        "\u{f02cb}"
    } else {
        "\u{f04c3}"
    };

    let text_width = width - 20.0 - 24.0 - 22.0 - 10.0 * 2.0;

    let name = Rectangle::new()
        .width(text_width)
        .height(SINK_HEIGHT)
        .align_child(Start, Center)
        .child(Text::new(sink.description.clone()).size(13.0).color(theme.text).elide());

    let check = Rectangle::new()
        .width(22.0)
        .height(SINK_HEIGHT)
        .align_child(Center, Center)
        .child(icon(if active { "\u{f012c}" } else { "" }, 15.0, theme.accent));

    let badge = Rectangle::new()
        .width(24.0)
        .height(SINK_HEIGHT)
        .align_child(Center, Center)
        .child(icon(glyph, 16.0, if active { theme.accent } else { theme.muted }));

    let device = sink.name.clone();

    Rectangle::new()
        .width(width)
        .height(SINK_HEIGHT)
        .radius(10.0)
        .fill(fill)
        .padding(amane::Padding { top: 0.0, right: 10.0, bottom: 0.0, left: 10.0 })
        .cursor(Pointer)
        .on_hover(move |inside| Ui::set_hover(&key, inside))
        .on_click(move |_| Mixer::select(&device))
        .child(Row::new(children![badge, name, check]).gap(10.0).align(Center))
}

// приложение: имя, кнопка «без звука», ползунок и значение
fn app_row(theme: &Theme, ui: &Ui, stream: &Stream, width: f32) -> Rectangle {
    let id = stream.id;
    let key = format!("app:{id}");

    let volume = ui.slider_value(&key, stream.volume);

    let mute = icon_button(
        ui,
        theme,
        &format!("app-mute:{id}"),
        if stream.muted { VOLUME_OFF } else { VOLUME_HIGH },
        26.0,
        if stream.muted { theme.red } else { theme.text },
    )
    .on_click(move |_| Mixer::toggle_stream_mute(id));

    let name = Rectangle::new()
        .width(width - 20.0 - 26.0 - 8.0)
        .height(26.0)
        .align_child(Start, Center)
        .child(Text::new(stream.name.clone()).size(13.0).color(theme.text).elide());

    let top = Row::new(children![name, mute]).gap(8.0).align(Center);

    let track = slider(
        theme,
        width - 20.0 - PERCENT_WIDTH - 8.0,
        volume,
        if stream.muted { theme.dim } else { theme.accent },
        {
            let key = key.clone();

            move |value| {
                if Ui::drag(&key, value) {
                    Mixer::set_stream_volume(id, value);
                }
            }
        },
    );

    let percent = Rectangle::new()
        .width(PERCENT_WIDTH)
        .height(22.0)
        .align_child(End, Center)
        .child(label(format!("{volume}%"), 12.0, theme.muted));

    let bottom = Row::new(children![track, percent]).gap(8.0).align(Center);

    Rectangle::new()
        .width(width)
        .height(APP_HEIGHT)
        .radius(10.0)
        .fill(theme.surface)
        .padding(amane::Padding { top: 4.0, right: 10.0, bottom: 4.0, left: 10.0 })
        .child(Column::new(children![top, bottom]))
}
