use amane::{
    Color, Full, Horizontal, Key, Keyboard, Layer, LayerWindow, Margin, Monitor, Parent, Rectangle,
    Service, TextInput, Vertical, Widget, Zone,
};

use crate::state::{Popup, TIMER_INPUT, Ui};
use crate::theme::{Theme, theme};
use crate::{audio, battery, bluetooth, calendar, clipboard, temperature, wifi};

// ширина и высота окна
pub fn size(kind: Popup) -> (f32, f32) {
    match kind {
        // рамка и padding + календарь + зазор + разделитель + зазор + таймер
        Popup::Calendar => (26.0 + 282.0 + 16.0 + 1.0 + 16.0 + 120.0, 316.0),
        Popup::Bluetooth => (344.0, 420.0),
        Popup::Wifi => (344.0, 460.0),
        Popup::Audio => (368.0, 480.0),
        Popup::Battery => (320.0, 372.0),
        Popup::Notifications => (344.0, 480.0),
        Popup::Clipboard => clipboard::SIZE,
        Popup::Temperature => (320.0, 220.0),
    }
}

// рамка окна: фон, скругление, отступ
pub fn frame(theme: &Theme, width: f32, height: f32, child: impl Widget + 'static) -> Rectangle {
    Rectangle::new()
        .width(width)
        .height(height)
        .radius(12.0)
        .fill(theme.bg)
        .border(1.0, theme.border)
        .padding(12.0)
        .child(child)
}

// ширина содержимого внутри рамки
pub fn inner(width: f32) -> f32 {
    width - 12.0 * 2.0 - 2.0
}

// прозрачный слой на весь экран под окнами: щелчок мимо закрывает окно
pub fn dismiss(_monitor: &Monitor) -> LayerWindow {
    let open = Ui::read().popup.is_some();

    LayerWindow::new()
        .width(Full)
        .height(Full)
        .layer(Layer::Top)
        // не заходит на место, зарезервированное баром
        .space(Zone::Respect)
        .namespace("dismiss")
        .visible(open)
        .child(
            Rectangle::new()
                .width(Parent)
                .height(Parent)
                .fill(Color::TRANSPARENT)
                .on_click(|_| Ui::close_all()),
        )
}

pub fn view(monitor: &Monitor) -> LayerWindow {
    let theme = theme();
    let ui = Ui::read();

    let open = ui.popup.is_some() && ui.monitor == monitor.name;
    let kind = ui.popup.unwrap_or(Popup::Calendar);

    let (width, height) = size(kind);

    // пока вводят пароль, ищут в буфере или набирают минуты, окно забирает клавиатуру целиком
    let typing = (kind == Popup::Wifi && ui.password_for.is_some())
        || kind == Popup::Clipboard
        || (kind == Popup::Calendar && ui.timer_editing);

    let keyboard = if open && typing {
        Keyboard::Exclusive
    } else if open {
        Keyboard::OnDemand
    } else {
        Keyboard::None
    };

    let content: Rectangle = if !open {
        Rectangle::new().width(width).height(height)
    } else {
        match kind {
            Popup::Calendar => calendar::view(&theme, &ui, width, height),
            Popup::Bluetooth => bluetooth::view(&theme, &ui, width, height),
            Popup::Wifi => wifi::view(&theme, &ui, width, height),
            Popup::Audio => audio::view(&theme, &ui, width, height),
            Popup::Battery => battery::view(&theme, width, height),
            Popup::Notifications => crate::notifications::center_view(&theme, &ui, width, height),
            Popup::Clipboard => clipboard::view(&theme, &ui, width, height),
            Popup::Temperature => temperature::view(&theme, &ui, width, height),
        }
    };

    // календарь и буфер по центру, остальные у правого края, под своими значками
    let (horizontal, right) = if kind == Popup::Calendar || kind == Popup::Clipboard {
        (Horizontal::Middle, 0)
    } else {
        (Horizontal::Right, 8)
    };

    LayerWindow::new()
        .width(width)
        .height(height)
        .anchor_vertical(Vertical::Top)
        .anchor_horizontal(horizontal)
        .margin(Margin {
            top: 6,
            right,
            bottom: 0,
            left: 0,
        })
        .layer(Layer::Overlay)
        .space(Zone::Respect)
        .keyboard(keyboard)
        .on_key(|key: Key| match key {
            Key::Escape => {
                // читаем до write(): иначе взаимоблокировка
                let editing = Ui::read().timer_editing;

                if editing {
                    TextInput::set_text(TIMER_INPUT, "");
                    Ui::write().timer_editing = false;
                } else {
                    Ui::close_all();
                }
            }
            // имена Key::Up и Key::Down сверить с SDK
            Key::Up => Ui::clip_move(-1),
            Key::Down => Ui::clip_move(1),
            _ => {}
        })
        .namespace("popup")
        .visible(open)
        .child(content)
}
