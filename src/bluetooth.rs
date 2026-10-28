use amane::{
    Bluetooth, BluetoothDevice, Center, Column, End, Pointer, Rectangle, Row, ScrollArea,
    Service, Stack, Start, Text, Weight, Widget, children,
};

use crate::popup::{frame, inner};
use crate::state::Ui;
use crate::theme::Theme;
use crate::ui::{HEADER_HEIGHT, header, icon, label, pad};

const LIST: &str = "bluetooth-devices";

const CARD_HEIGHT: f32 = 54.0;
const CARD_GAP: f32 = 7.0;
const ICON_SIZE: f32 = 34.0;
const FORGET_SIZE: f32 = 22.0;

pub fn view(theme: &Theme, ui: &Ui, width: f32, height: f32) -> Rectangle {
    let inner_width = inner(width);
    let bluetooth = Bluetooth::read();

    let powered = bluetooth.powered();

    let status = if !bluetooth.available() {
        "Нет адаптера"
    } else if !powered {
        "Выключен"
    } else if bluetooth.scanning() {
        "Поиск устройств…"
    } else {
        "Готов"
    };

    let head = header(
        theme,
        "\u{f00af}",
        "Bluetooth",
        status,
        theme.muted,
        powered,
        inner_width,
        move || Bluetooth::set_powered(!powered),
    );

    let mut parts: Vec<Box<dyn Widget>> = vec![Box::new(head)];

    if powered {
        let list_height = height - 26.0 - HEADER_HEIGHT - 10.0;

        parts.push(Box::new(list(theme, ui, &bluetooth, inner_width, list_height)));
    }

    frame(theme, width, height, Column::new(parts).gap(10.0))
}

// сначала сопряжённые, потом найденные при поиске
fn list(theme: &Theme, ui: &Ui, bluetooth: &Bluetooth, width: f32, height: f32) -> Rectangle {
    let area = Rectangle::new().width(width).height(height);

    if bluetooth.devices().is_empty() {
        let text = if bluetooth.scanning() { "Поиск…" } else { "Устройства не найдены" };

        return area.align_child(Center, Center).child(label(text, 13.0, theme.muted));
    }

    let mut paired = Vec::new();
    let mut found = Vec::new();

    for device in bluetooth.devices() {
        if device.paired() {
            paired.push(device);
        } else {
            found.push(device);
        }
    }

    paired.extend(found);

    let mut cards: Vec<Box<dyn Widget>> = Vec::new();

    for device in &paired {
        cards.push(Box::new(card(theme, ui, device, width)));
    }

    let total = CARD_HEIGHT * cards.len() as f32 + CARD_GAP * (cards.len() - 1) as f32;

    let column = Column::new(cards).width(width).height(total).gap(CARD_GAP);

    area.clip().child(ScrollArea::new(LIST, column))
}

// нажатие подключает/отключает сопряжённое устройство, новое — сопрягает
fn card(theme: &Theme, ui: &Ui, device: &BluetoothDevice, width: f32) -> Stack {
    let connected = device.connected();
    let paired = device.paired();

    let badge = Rectangle::new()
        .width(ICON_SIZE)
        .height(ICON_SIZE)
        .radius(8.0)
        .fill(theme.hover)
        .align_child(Center, Center)
        .child(icon(device_icon(device.icon()), 16.0, theme.accent));

    let (status, status_color) = if connected {
        ("Подключено", theme.green)
    } else if paired {
        ("Сопряжено", theme.muted)
    } else {
        ("Доступно", theme.muted)
    };

    let weight = if connected { Weight::Bold } else { Weight::Regular };

    // ширина текста: что осталось рядом с иконкой, батареей и кнопкой «забыть»
    let text_width = width - 18.0 - ICON_SIZE - 36.0 - FORGET_SIZE - 9.0 * 3.0;

    let name = Rectangle::new()
        .width(text_width)
        .height(18.0)
        .child(Text::new(device.name()).size(13.0).color(theme.text).weight(weight).elide());

    let text = Rectangle::new()
        .width(text_width)
        .height(ICON_SIZE)
        .align_child(Start, Center)
        .child(Column::new(children![name, label(status, 10.0, status_color)]));

    let battery = match device.battery() {
        Some(percent) => format!("{percent}%"),
        None => String::new(),
    };

    let battery = Rectangle::new()
        .width(36.0)
        .height(ICON_SIZE)
        .align_child(End, Center)
        .child(label(battery, 10.0, theme.muted));

    let key = format!("bt:{}", device.path());

    let fill = if connected {
        theme.hover
    } else if ui.hovered.as_deref() == Some(key.as_str()) {
        theme.hover
    } else {
        theme.surface
    };

    let body = Rectangle::new()
        .width(width)
        .height(CARD_HEIGHT)
        .radius(12.0)
        .fill(fill)
        .padding(pad(9.0))
        .child(Row::new(children![badge, text, battery]).gap(9.0).align(Center));

    let path = String::from(device.path());

    let hover_key = key.clone();

    let click = Rectangle::new()
        .width(width)
        .height(CARD_HEIGHT)
        .cursor(Pointer)
        .on_hover(move |inside| Ui::set_hover(&hover_key, inside))
        .on_click(move |_| {
            if !paired {
                Bluetooth::pair(&path);
            } else if connected {
                Bluetooth::disconnect(&path);
            } else {
                Bluetooth::connect(&path);
            }
        });

    let mut layers: Vec<Box<dyn Widget>> = vec![Box::new(body), Box::new(click)];

    // поверх области нажатия, чтобы «забыть» не подключало заодно
    if paired {
        layers.push(Box::new(forget(theme, ui, device.path(), width)));
    }

    Stack::new(layers).width(width).height(CARD_HEIGHT)
}

fn forget(theme: &Theme, ui: &Ui, device: &str, card_width: f32) -> Rectangle {
    let key = format!("bt-forget:{device}");

    let color = if ui.hovered.as_deref() == Some(key.as_str()) { theme.red } else { theme.muted };

    let path = String::from(device);

    Rectangle::new()
        .width(FORGET_SIZE)
        .height(CARD_HEIGHT)
        .translate(card_width - 9.0 - FORGET_SIZE, 0.0)
        .align_child(Center, Center)
        .cursor(Pointer)
        .on_hover(move |inside| Ui::set_hover(&key, inside))
        .on_click(move |_| Bluetooth::forget(&path))
        .child(icon("\u{f0156}", 14.0, color))
}

// bluez называет типы вроде "audio-headset" или "input-mouse"
fn device_icon(kind: &str) -> &'static str {
    const ICONS: [(&str, &str); 6] = [
        ("audio", "\u{f02cb}"),
        ("input-mouse", "\u{f037d}"),
        ("input-keyboard", "\u{f030c}"),
        ("input-gaming", "\u{f02b4}"),
        ("phone", "\u{f03f2}"),
        ("computer", "\u{f0379}"),
    ];

    for (prefix, glyph) in ICONS {
        if kind.starts_with(prefix) {
            return glyph;
        }
    }

    "\u{f00af}"
}

