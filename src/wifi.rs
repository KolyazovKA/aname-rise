use std::cell::RefCell;

use amane::{
    AccessPoint, Center, Color, Column, End, Network, Pointer, Rectangle, Row, ScrollArea, Service,
    Stack, Start, Text, TextInput, Weight, Widget, children,
};

use crate::popup::{frame, inner};
use crate::state::{PASSWORD_INPUT, Ui};
use crate::theme::Theme;
use crate::ui::{HEADER_HEIGHT, header, icon, label, pad};

const LIST: &str = "wifi-networks";

const CARD_GAP: f32 = 7.0;
const CARD_HEIGHT: f32 = 50.0;
const OPEN_CARD_HEIGHT: f32 = 104.0;
const ROW_HEIGHT: f32 = 32.0;
const FIELD_HEIGHT: f32 = 34.0;

thread_local! {
    // пароль так, как его набрали, для кнопки «подключиться»
    static TYPED: RefCell<String> = const { RefCell::new(String::new()) };
}

pub fn view(theme: &Theme, ui: &Ui, width: f32, height: f32) -> Rectangle {
    let inner_width = inner(width);
    let network = Network::read();

    let enabled = network.wifi_enabled();

    let (status, status_color) = if !enabled {
        ("Выключен", theme.muted)
    } else if !network.ssid().is_empty() {
        (network.ssid(), theme.green)
    } else {
        ("Не подключено", theme.muted)
    };

    let head = header(
        theme,
        "\u{f05a9}",
        "Wi-Fi",
        status,
        status_color,
        enabled,
        inner_width,
        move || Network::set_wifi(!enabled),
    );

    let mut parts: Vec<Box<dyn Widget>> = vec![Box::new(head)];

    if enabled {
        let list_height = height - 26.0 - HEADER_HEIGHT - 10.0;

        parts.push(Box::new(list(theme, ui, &network, inner_width, list_height)));
    }

    frame(theme, width, height, Column::new(parts).gap(10.0))
}

fn list(theme: &Theme, ui: &Ui, network: &Network, width: f32, height: f32) -> Rectangle {
    let area = Rectangle::new().width(width).height(height);

    if network.access_points().is_empty() {
        return area
            .align_child(Center, Center)
            .child(label("Сети не найдены", 13.0, theme.muted));
    }

    // сначала текущая сеть, остальные как пришли (сильные первыми)
    let mut ordered: Vec<&AccessPoint> = Vec::new();

    for access_point in network.access_points() {
        if access_point.active() {
            ordered.insert(0, access_point);
        } else {
            ordered.push(access_point);
        }
    }

    let mut cards: Vec<Box<dyn Widget>> = Vec::new();
    let mut total = 0.0;

    for access_point in ordered {
        let (card, card_height) = card(theme, ui, network, access_point, width);

        total += card_height;

        cards.push(Box::new(card));
    }

    total += CARD_GAP * (cards.len() - 1) as f32;

    let column = Column::new(cards).width(width).height(total).gap(CARD_GAP);

    area.clip().child(ScrollArea::new(LIST, column))
}

// одна сеть; у новой защищённой под ней раскрывается поле пароля
fn card(
    theme: &Theme,
    ui: &Ui,
    network: &Network,
    access_point: &AccessPoint,
    width: f32,
) -> (Stack, f32) {
    let ssid = access_point.ssid();
    let active = access_point.active();

    let open = !active && ui.password_for.as_deref() == Some(ssid);
    let height = if open { OPEN_CARD_HEIGHT } else { CARD_HEIGHT };

    let inner_width = width - 18.0;

    let connecting = network.connecting() && ui.joining.as_deref() == Some(ssid);

    let mut rows: Vec<Box<dyn Widget>> = vec![Box::new(top_row(theme, ui, access_point, connecting, inner_width))];

    if open {
        rows.push(Box::new(password_row(theme, ui, ssid, inner_width)));
    }

    let key = format!("wifi:{ssid}");

    let fill = if active || ui.hovered.as_deref() == Some(key.as_str()) { theme.hover } else { theme.surface };

    let body = Rectangle::new()
        .width(width)
        .height(height)
        .radius(12.0)
        .fill(fill)
        .padding(pad(9.0))
        .child(Column::new(rows).gap(7.0));

    // вся верхняя часть карточки подключает, кроме кнопки «отключить» у текущей
    let click_width = if active { width - CARD_HEIGHT } else { width };

    let name = String::from(ssid);
    let secured = access_point.secured();
    let saved = access_point.saved();

    let mut click = Rectangle::new().width(click_width).height(CARD_HEIGHT);

    if !active {
        click = click
            .cursor(Pointer)
            .on_hover(move |inside| Ui::set_hover(&key, inside))
            .on_click({
                let name = name.clone();

                move |_| card_clicked(&name, secured, saved)
            });
    }

    (Stack::new(children![body, click]), height)
}

fn top_row(theme: &Theme, ui: &Ui, access_point: &AccessPoint, connecting: bool, width: f32) -> Row {
    let active = access_point.active();

    let signal = Rectangle::new()
        .width(22.0)
        .height(ROW_HEIGHT)
        .align_child(Center, Center)
        .child(icon(signal_icon(access_point.strength()), 17.0, theme.accent));

    let status = if active {
        "Подключено"
    } else if connecting {
        "Подключение…"
    } else if access_point.saved() {
        "Сохранена"
    } else if access_point.secured() {
        "Защищена"
    } else {
        "Открытая сеть"
    };

    let (weight, status_color) = if active {
        (Weight::Bold, theme.green)
    } else {
        (Weight::Regular, theme.muted)
    };

    let text_width = width - 22.0 - ROW_HEIGHT - 9.0 * 2.0;

    let name = Rectangle::new()
        .width(text_width)
        .height(18.0)
        .child(Text::new(access_point.ssid()).size(13.0).color(theme.text).weight(weight).elide());

    let text = Rectangle::new()
        .width(text_width)
        .height(ROW_HEIGHT)
        .align_child(Start, Center)
        .child(Column::new(children![name, label(status, 10.0, status_color)]));

    let action: Rectangle = if active {
        disconnect_button(theme, ui, connecting)
    } else {
        let glyph = if access_point.saved() { "\u{f012c}" } else if access_point.secured() { "\u{f033e}" } else { "" };

        Rectangle::new()
            .width(ROW_HEIGHT)
            .height(ROW_HEIGHT)
            .align_child(End, Center)
            .child(icon(glyph, 14.0, theme.muted))
    };

    Row::new(children![signal, text, action]).gap(9.0).align(Center)
}

fn disconnect_button(theme: &Theme, ui: &Ui, connecting: bool) -> Rectangle {
    let hovered = ui.hovered.as_deref() == Some("wifi:disconnect");

    let (fill, color) = if hovered {
        (theme.red, theme.on_accent)
    } else {
        (Color::TRANSPARENT, theme.red)
    };

    let button = Rectangle::new()
        .width(ROW_HEIGHT)
        .height(ROW_HEIGHT)
        .radius(ROW_HEIGHT / 2.0)
        .fill(fill)
        .align_child(Center, Center)
        .child(icon("\u{f05aa}", 16.0, color));

    // пока идёт предыдущее изменение, кнопка неактивна
    if connecting {
        return button.opacity(0.4);
    }

    button
        .cursor(Pointer)
        .on_hover(|inside| Ui::set_hover("wifi:disconnect", inside))
        .on_click(|_| Network::disconnect())
}

// поле пароля, «глаз», отмена и подключение
fn password_row(theme: &Theme, ui: &Ui, ssid: &str, width: f32) -> Row {
    let field_width = width - FIELD_HEIGHT * 2.0 - 7.0 * 2.0;

    let name = String::from(ssid);

    let mut input = TextInput::new(PASSWORD_INPUT)
        .size(13.0)
        .color(theme.text)
        .placeholder("Пароль")
        .focused()
        .on_change(|password| TYPED.set(password))
        .on_submit(move |password| join(&name, &password));

    if !ui.show_password {
        input = input.password();
    }

    let input = Rectangle::new()
        .width(field_width - 44.0)
        .height(FIELD_HEIGHT)
        .translate(10.0, 0.0)
        .align_child(Start, Center)
        .child(input);

    let eye = if ui.show_password { "\u{f0208}" } else { "\u{f0209}" };

    let reveal = Rectangle::new()
        .width(34.0)
        .height(FIELD_HEIGHT)
        .translate(field_width - 34.0, 0.0)
        .cursor(Pointer)
        .on_click(|_| {
            let mut ui = Ui::write();

            ui.show_password = !ui.show_password;
        })
        .align_child(Center, Center)
        .child(icon(eye, 14.0, theme.muted));

    let field = Rectangle::new()
        .width(field_width)
        .height(FIELD_HEIGHT)
        .radius(8.0)
        .fill(theme.bg)
        .border(1.0, theme.accent)
        .child(Stack::new(children![input, reveal]));

    let cancel = small_button(theme, ui, "wifi:cancel", "\u{f0156}", theme.red, theme.border)
        .on_click(|_| {
            let mut ui = Ui::write();

            ui.password_for = None;
            ui.show_password = false;
        });

    let name = String::from(ssid);

    let confirm = small_button(theme, ui, "wifi:join", "\u{f012c}", theme.green, theme.green)
        .on_click(move |_| join(&name, &TYPED.take()));

    Row::new(children![field, cancel, confirm]).gap(7.0).align(Center)
}

fn small_button(
    theme: &Theme,
    ui: &Ui,
    key: &str,
    glyph: &str,
    color: Color,
    hover_fill: Color,
) -> Rectangle {
    let hovered = ui.hovered.as_deref() == Some(key);

    let (fill, color) = if hovered && hover_fill == color {
        (hover_fill, theme.on_accent)
    } else if hovered {
        (hover_fill, color)
    } else {
        (theme.bg, color)
    };

    let name = key.to_string();

    Rectangle::new()
        .width(FIELD_HEIGHT)
        .height(FIELD_HEIGHT)
        .radius(8.0)
        .fill(fill)
        .cursor(Pointer)
        .on_hover(move |inside| Ui::set_hover(&name, inside))
        .align_child(Center, Center)
        .child(icon(glyph, 14.0, color))
}

fn signal_icon(strength: u8) -> &'static str {
    match strength {
        76.. => "\u{f0928}",
        51..=75 => "\u{f0925}",
        26..=50 => "\u{f0922}",
        _ => "\u{f091f}",
    }
}

// известная или открытая сеть подключается сразу, остальные сначала просят пароль
fn card_clicked(ssid: &str, secured: bool, saved: bool) {
    if saved || !secured {
        join(ssid, "");

        return;
    }

    let mut ui = Ui::write();

    ui.password_for = Some(String::from(ssid));
    ui.show_password = false;

    TextInput::set_text(PASSWORD_INPUT, "");
    TYPED.take();
}

fn join(ssid: &str, password: &str) {
    let password = if password.is_empty() { None } else { Some(password) };

    Network::connect(ssid, password);

    let mut ui = Ui::write();

    ui.joining = Some(String::from(ssid));
    ui.password_for = None;
    ui.show_password = false;

    TextInput::set_text(PASSWORD_INPUT, "");
    TYPED.take();
}

