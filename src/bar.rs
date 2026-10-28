use amane::{
    Audio, Battery, Bluetooth, Button, Center, Color, Cpu, End, Full, Layer, LayerWindow, Link,
    Memory, Monitor, Network, Parent, Pointer, Rectangle, Row, Scroll, Service, Start, Text,
    Vertical, Weight, Widget, Workspace, Workspaces, Zone, children,
};

use crate::state::{Clock, KbLayout, Mixer, Popup, Temp, Ui, Vpn};
use crate::theme::{Theme, theme};

pub const HEIGHT: f32 = 30.0;

const ICON_SIZE: f32 = 16.0;
const TEXT_SIZE: f32 = 16.0;

// иконки рабочих столов из waybar: "1".."9"
const WORKSPACE_ICONS: [&str; 9] = [
    "\u{f268}", "\u{f120}", "\u{f249}", "\u{f121}", "\u{f2c6}", "\u{f19d}", "\u{f11b}", "\u{f1b2}", "\u{f001}",
];

// иконки раскладки: порядок как в niri (layout "us,ru")
const LAYOUT_FLAGS: [&str; 2] = ["🇺🇸", "🇷🇺"];

pub fn view(monitor: &Monitor) -> LayerWindow {
    let theme = theme();
    let ui = Ui::read();

    // три равные части, чтобы время оставалось по центру
    let third = monitor.width as f32 / 3.0;

    let sections = Row::new(children![
        left(&theme, &ui, monitor, third),
        center(&theme, &ui, monitor, third),
        right(&theme, &ui, monitor, third),
    ]);

    LayerWindow::new()
        .width(Full)
        .height(HEIGHT)
        .anchor_vertical(Vertical::Top)
        .layer(Layer::Top)
        .space(Zone::Reserve)
        .namespace("bar")
        .child(
            Rectangle::new()
                .width(Parent)
                .height(Parent)
                .fill(theme.bg)
                .child(sections),
        )
}

// ---------------------------------------------------------------- слева: рабочие столы

fn left(theme: &Theme, ui: &Ui, monitor: &Monitor, width: f32) -> Row {
    let workspaces = Workspaces::read();

    let mut own: Vec<&Workspace> = workspaces
        .list()
        .iter()
        .filter(|workspace| workspace.output() == Some(monitor.name.as_str()))
        .collect();

    own.sort_by_key(|workspace| workspace.index());

    let mut buttons: Vec<Box<dyn Widget>> = vec![Box::new(Rectangle::new().width(4.0).height(1.0))];

    for workspace in own {
        buttons.push(Box::new(workspace_button(theme, ui, workspace)));
    }

    Row::new(buttons).width(width).height(Parent).gap(2.0).justify(Start).align(Center)
}

fn workspace_button(theme: &Theme, ui: &Ui, workspace: &Workspace) -> Rectangle {
    let id = workspace.id();

    // как в waybar: иконка по имени рабочего стола, иначе по номеру
    let number = workspace
        .name()
        .and_then(|name| name.parse::<usize>().ok())
        .unwrap_or(workspace.index() as usize);

    let glyph = number
        .checked_sub(1)
        .and_then(|position| WORKSPACE_ICONS.get(position))
        .copied()
        .unwrap_or("\u{e2ca}");

    let key = format!("ws:{id}");

    let fill = if workspace.focused() || workspace.active() {
        theme.accent
    } else if ui.hovered.as_deref() == Some(key.as_str()) {
        theme.hover
    } else {
        Color::TRANSPARENT
    };

    let color = if workspace.urgent() { theme.red } else { theme.text };

    Rectangle::new()
        .width(34.0)
        .height(24.0)
        .radius(6.0)
        .fill(fill)
        .cursor(Pointer)
        .on_hover(move |inside| Ui::set_hover(&key, inside))
        .on_click(move |_| Workspaces::focus(id))
        .align_child(Center, Center)
        .child(Text::new(glyph).size(ICON_SIZE).color(color).tight())
}

// ---------------------------------------------------------------- центр: время

fn center(theme: &Theme, ui: &Ui, monitor: &Monitor, width: f32) -> Rectangle {
    let clock = Clock::read();

    let key = String::from("clock");
    let open = ui.popup == Some(Popup::Calendar) && ui.monitor == monitor.name;

    let fill = if open || ui.hovered.as_deref() == Some("clock") {
        theme.hover
    } else {
        Color::TRANSPARENT
    };

    let name = monitor.name.clone();

    let chip = Rectangle::new()
        .width(72.0)
        .height(24.0)
        .radius(6.0)
        .fill(fill)
        .cursor(Pointer)
        .on_hover(move |inside| Ui::set_hover(&key, inside))
        .on_click(move |_| Ui::toggle(Popup::Calendar, &name))
        .align_child(Center, Center)
        .child(
            Text::new(clock.time())
                .size(TEXT_SIZE)
                .color(theme.text)
                .weight(Weight::Bold),
        );

    Rectangle::new()
        .width(width)
        .height(Parent)
        .align_child(Center, Center)
        .child(chip)
}

// ---------------------------------------------------------------- справа

// amane не умеет подгонять размер под содержимое, поэтому ширину кнопок считаем сами:
// JetBrains Mono моноширинный (0.6 от размера на символ), иконка стоит в слоте 18px
enum Part {
    Icon(&'static str, Color),
    Value(String, Color),
}

const ICON_SLOT: f32 = 18.0;
const PART_GAP: f32 = 5.0;
const CHIP_PADDING: f32 = 6.0;

fn part_width(part: &Part) -> f32 {
    match part {
        Part::Icon(..) => ICON_SLOT,
        Part::Value(text, _) => text.chars().count() as f32 * TEXT_SIZE * 0.6,
    }
}

fn right(theme: &Theme, ui: &Ui, monitor: &Monitor, width: f32) -> Row {
    let mut items: Vec<Box<dyn Widget>> = vec![Box::new(layout(theme))];

    items.push(Box::new(metric(theme, ui, "\u{ec19}", format!("{}%", Cpu::read().percent()), false)));
    items.push(Box::new(metric(theme, ui, "\u{e266}", format!("{}%", Memory::read().percent()), false)));

    let celsius = Temp::read().celsius;

    items.push(Box::new(metric(theme, ui, "", format!("{celsius}°C"), celsius >= 80)));
    items.push(Box::new(bluetooth(theme, ui, monitor)));
    items.push(Box::new(vpn(theme, ui)));
    items.push(Box::new(wifi(theme, ui, monitor)));
    items.push(Box::new(volume(theme, ui, monitor)));

    if let Some(battery) = battery(theme, ui, monitor) {
        items.push(Box::new(battery));
    }

    items.push(Box::new(Rectangle::new().width(2.0).height(1.0)));

    Row::new(items).width(width).height(Parent).gap(4.0).justify(End).align(Center)
}

fn content(parts: Vec<Part>) -> (Row, f32) {
    let mut width = CHIP_PADDING * 2.0 + PART_GAP * parts.len().saturating_sub(1) as f32;
    let mut widgets: Vec<Box<dyn Widget>> = Vec::new();

    for part in &parts {
        width += part_width(part);
    }

    for part in parts {
        match part {
            Part::Icon(glyph, color) => widgets.push(Box::new(
                Rectangle::new()
                    .width(ICON_SLOT)
                    .height(24.0)
                    .align_child(Center, Center)
                    .child(Text::new(glyph).size(ICON_SIZE).color(color).tight()),
            )),
            Part::Value(text, color) => {
                widgets.push(Box::new(Text::new(text).size(TEXT_SIZE).color(color).weight(Weight::Bold)));
            }
        }
    }

    (Row::new(widgets).gap(PART_GAP).align(Center), width)
}

// кнопка справа: иконка и значение; с ключом подсвечивается под курсором и при открытом окне
fn chip(
    theme: &Theme,
    ui: &Ui,
    key: Option<&str>,
    popup: Option<(Popup, &Monitor)>,
    parts: Vec<Part>,
) -> Rectangle {
    let (row, width) = content(parts);

    let open = popup.is_some_and(|(kind, monitor)| ui.popup == Some(kind) && ui.monitor == monitor.name);

    let hovered = key.is_some() && ui.hovered.as_deref() == key;

    let fill = if open || hovered { theme.hover } else { Color::TRANSPARENT };

    let mut chip = Rectangle::new()
        .width(width)
        .height(24.0)
        .radius(6.0)
        .fill(fill)
        .align_child(Center, Center)
        .child(row);

    if let Some(key) = key {
        let name = key.to_string();

        chip = chip.on_hover(move |inside| Ui::set_hover(&name, inside));
    }

    if let Some((kind, monitor)) = popup {
        let monitor = monitor.name.clone();

        chip = chip.cursor(Pointer).on_click(move |button| {
            if button == Button::Left {
                Ui::toggle(kind, &monitor);
            }
        });
    }

    chip
}

fn layout(theme: &Theme) -> Rectangle {
    let layout = KbLayout::read();

    let flag = LAYOUT_FLAGS.get(layout.index).copied().unwrap_or("?");

    Rectangle::new()
        .width(30.0)
        .height(24.0)
        .align_child(Center, Center)
        .child(Text::new(flag).size(ICON_SIZE).color(theme.text))
}

// cpu, память, температура: без нажатия
fn metric(theme: &Theme, ui: &Ui, icon: &'static str, value: String, hot: bool) -> Rectangle {
    let color = if hot { theme.red } else { theme.text };

    let mut parts = Vec::new();

    if !icon.is_empty() {
        parts.push(Part::Icon(icon, color));
    }

    parts.push(Part::Value(value, color));

    chip(theme, ui, None, None, parts)
}

fn bluetooth(theme: &Theme, ui: &Ui, monitor: &Monitor) -> Rectangle {
    let bluetooth = Bluetooth::read();

    let connected = bluetooth.devices().iter().any(|device| device.connected());

    let (icon, color) = if !bluetooth.powered() {
        ("\u{f00b2}", theme.muted)
    } else if connected {
        ("\u{f00b1}", theme.text)
    } else {
        ("\u{f00af}", theme.text)
    };

    chip(theme, ui, Some("bluetooth"), Some((Popup::Bluetooth, monitor)), vec![Part::Icon(icon, color)])
}

fn vpn(theme: &Theme, ui: &Ui) -> Rectangle {
    let vpn = Vpn::read();

    let color = if vpn.active { theme.green } else { theme.dim };

    chip(theme, ui, None, None, vec![Part::Icon("\u{f0582}", color)])
}

fn wifi(theme: &Theme, ui: &Ui, monitor: &Monitor) -> Rectangle {
    let network = Network::read();

    let mut parts = Vec::new();

    if !network.wifi_enabled() && network.link() != Link::Wired {
        parts.push(Part::Icon("\u{f071}", theme.yellow));
        parts.push(Part::Value(String::from("Disabled"), theme.text));
    } else {
        match network.link() {
            Link::Offline => {
                parts.push(Part::Icon("\u{f071}", theme.yellow));
                parts.push(Part::Value(String::from("Disconnected"), theme.text));
            }
            Link::Wired => parts.push(Part::Icon("\u{f0e0b}", theme.text)),
            Link::Wifi => parts.push(Part::Icon("\u{f1eb}", theme.text)),
            Link::Other => parts.push(Part::Icon("\u{f0c1}", theme.text)),
        }
    }

    chip(theme, ui, Some("wifi"), Some((Popup::Wifi, monitor)), parts)
}

fn volume(theme: &Theme, ui: &Ui, monitor: &Monitor) -> Rectangle {
    let audio = Audio::read();
    let mixer = Mixer::read();

    let mut parts = Vec::new();

    if audio.muted() {
        parts.push(Part::Icon("\u{f026}", theme.text));
    } else {
        let icon = if mixer.headphones() {
            "\u{f02cb}"
        } else if audio.volume() < 50 {
            "\u{f027}"
        } else {
            "\u{f028}"
        };

        parts.push(Part::Icon(icon, theme.text));
        parts.push(Part::Value(format!("{}%", audio.volume()), theme.text));

        if mixer.bluetooth() {
            parts.push(Part::Icon("\u{f294}", theme.text));
        }
    }

    chip(theme, ui, Some("volume"), Some((Popup::Audio, monitor)), parts).on_scroll(|scroll: Scroll| {
        // как у waybar: колесо меняет громкость
        let volume = i32::from(Audio::read().volume());
        let step = if scroll.y < 0.0 { 5 } else { -5 };

        Audio::set_volume((volume + step).clamp(0, 100) as u8);
    })
}

fn battery(theme: &Theme, ui: &Ui, monitor: &Monitor) -> Option<Rectangle> {
    let battery = Battery::read();

    if !battery.present() {
        return None;
    }

    let percent = battery.percent();
    let charging = battery.charging() || battery.full();

    // иконки waybar: 5 ступеней
    const LEVELS: [&str; 5] = ["\u{f007b}", "\u{f007d}", "\u{f007f}", "\u{f0081}", "\u{f0079}"];

    let icon = if charging {
        "\u{f0084}"
    } else {
        LEVELS[((usize::from(percent) * LEVELS.len()) / 100).min(LEVELS.len() - 1)]
    };

    let color = if charging {
        theme.text
    } else if percent <= 15 {
        theme.red
    } else if percent <= 30 {
        theme.yellow
    } else {
        theme.text
    };

    Some(chip(
        theme,
        ui,
        Some("battery"),
        Some((Popup::Battery, monitor)),
        vec![Part::Icon(icon, color), Part::Value(format!("{percent}%"), color)],
    ))
}
