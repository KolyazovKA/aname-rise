use amane::{Battery, Center, Column, End, Rectangle, Row, Service, SpaceBetween, Start, Text, Widget, children};

use crate::popup::{frame, inner};
use crate::state::BatteryInfo;
use crate::theme::Theme;
use crate::ui::{bold, icon, label};

pub fn view(theme: &Theme, width: f32, height: f32) -> Rectangle {
    let inner_width = inner(width);

    let battery = Battery::read();
    let info = BatteryInfo::read();
    let data = &info.data;

    if !battery.present() || !data.present {
        return frame(
            theme,
            width,
            height,
            Rectangle::new()
                .width(inner_width)
                .height(height - 26.0)
                .align_child(Center, Center)
                .child(label("Батарея не найдена", 14.0, theme.muted)),
        );
    }

    let percent = battery.percent();
    let charging = battery.charging();
    let full = battery.full() || data.status == "Full";

    let color = if charging || full {
        theme.green
    } else if percent <= 15 {
        theme.red
    } else if percent <= 30 {
        theme.yellow
    } else {
        theme.accent
    };

    let glyph = if charging { "\u{f0084}" } else if full { "\u{f0079}" } else { "\u{f0081}" };

    let status = match data.status.as_str() {
        "Charging" => "Заряжается",
        "Discharging" => "Разряжается",
        "Full" => "Заряжена",
        "Not charging" => "Подключена, не заряжается",
        "" => "Неизвестно",
        other => other,
    };

    let top = Row::new(children![
        icon(glyph, 38.0, color),
        Column::new(children![
            bold(format!("{percent}%"), 28.0, theme.text),
            label(status, 12.0, theme.muted),
        ]),
    ])
    .gap(12.0)
    .align(Center);

    let filled = inner_width * f32::from(percent.min(100)) / 100.0;

    let bar = Rectangle::new()
        .width(inner_width)
        .height(10.0)
        .radius(5.0)
        .fill(theme.border)
        .child(Rectangle::new().width(filled.max(0.5)).height(10.0).radius(5.0).fill(color));

    let mut rows: Vec<Box<dyn Widget>> = Vec::new();

    // время до разряда или до полной зарядки
    if data.power > 0.5 {
        if charging && data.energy_full > data.energy_now {
            rows.push(Box::new(row(theme, "До полной зарядки", format_hours((data.energy_full - data.energy_now) / data.power), inner_width)));
        } else if !charging && !full {
            rows.push(Box::new(row(theme, "Осталось", format_hours(data.energy_now / data.power), inner_width)));
        }
    }

    if data.power > 0.05 {
        rows.push(Box::new(row(theme, "Мощность", format!("{:.1} Вт", data.power), inner_width)));
    }

    if data.energy_full > 0.0 {
        rows.push(Box::new(row(
            theme,
            "Энергия",
            format!("{:.1} / {:.1} Вт·ч", data.energy_now, data.energy_full),
            inner_width,
        )));
    }

    if data.energy_design > 0.0 && data.energy_full > 0.0 {
        let health = data.energy_full / data.energy_design * 100.0;

        rows.push(Box::new(row(theme, "Состояние батареи", format!("{health:.0}%"), inner_width)));
    }

    if let Some(cycles) = data.cycles {
        rows.push(Box::new(row(theme, "Циклов зарядки", cycles.to_string(), inner_width)));
    }

    if !data.technology.is_empty() {
        rows.push(Box::new(row(theme, "Технология", data.technology.clone(), inner_width)));
    }

    if !data.model.is_empty() {
        rows.push(Box::new(row(theme, "Модель", data.model.clone(), inner_width)));
    }

    let profile = match data.profile.as_str() {
        "performance" => "Производительность",
        "balanced" => "Сбалансированный",
        "power-saver" => "Экономия энергии",
        "" => "",
        other => other,
    };

    if !profile.is_empty() {
        rows.push(Box::new(row(theme, "Профиль питания", profile.to_string(), inner_width)));
    }

    let details = Column::new(rows).gap(4.0);

    frame(theme, width, height, Column::new(children![top, bar, details]).gap(14.0))
}

// «название — значение», длинное значение обрезается
fn row(theme: &Theme, name: &str, value: String, width: f32) -> Row {
    let name = Rectangle::new()
        .width(150.0)
        .height(24.0)
        .align_child(Start, Center)
        .child(label(name, 13.0, theme.muted));

    let value = Rectangle::new()
        .width(width - 150.0 - 8.0)
        .height(24.0)
        .align_child(End, Center)
        .child(Text::new(value).size(13.0).color(theme.text).elide());

    Row::new(children![name, value]).width(width).gap(8.0).justify(SpaceBetween).align(Center)
}

fn format_hours(hours: f64) -> String {
    let minutes = (hours * 60.0).round() as i64;

    if minutes >= 60 {
        format!("{} ч {:02} мин", minutes / 60, minutes % 60)
    } else {
        format!("{minutes} мин")
    }
}

