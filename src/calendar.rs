use amane::{Center, Color, Column, Pointer, Rectangle, Row, Service, SpaceBetween, Widget, children};

use crate::popup::{frame, inner};
use crate::state::{Clock, Ui, civil, days_from_civil};
use crate::theme::Theme;
use crate::ui::{bold, icon_button, label};

const MONTHS: [&str; 12] = [
    "Январь", "Февраль", "Март", "Апрель", "Май", "Июнь", "Июль", "Август", "Сентябрь", "Октябрь",
    "Ноябрь", "Декабрь",
];

// неделя начинается с понедельника
const WEEKDAYS: [&str; 7] = ["Пн", "Вт", "Ср", "Чт", "Пт", "Сб", "Вс"];

const CELL_WIDTH: f32 = 36.0;
const CELL_HEIGHT: f32 = 34.0;

pub fn view(theme: &Theme, ui: &Ui, width: f32, height: f32) -> Rectangle {
    let inner_width = inner(width);

    let (today_year, today_month, today_day) = Clock::read().today();

    let total = today_year * 12 + (today_month - 1) + ui.month;
    let year = total.div_euclid(12);
    let month = total.rem_euclid(12) + 1;

    let title = bold(format!("{} {year}", MONTHS[(month - 1) as usize]), 16.0, theme.text);

    let previous = icon_button(ui, theme, "cal:previous", "\u{f004d}", 28.0, theme.text)
        .on_click(|_| Ui::write().month -= 1);

    let next = icon_button(ui, theme, "cal:next", "\u{f0054}", 28.0, theme.text)
        .on_click(|_| Ui::write().month += 1);

    let mut controls: Vec<Box<dyn Widget>> = Vec::new();

    // кнопка возврата к текущему месяцу, пока смотрим на другой
    if ui.month != 0 {
        let fill = if ui.hovered.as_deref() == Some("cal:today") { theme.hover } else { theme.surface };

        controls.push(Box::new(
            Rectangle::new()
                .width(74.0)
                .height(28.0)
                .radius(8.0)
                .fill(fill)
                .cursor(Pointer)
                .on_hover(|inside| Ui::set_hover("cal:today", inside))
                .on_click(|_| Ui::write().month = 0)
                .align_child(Center, Center)
                .child(label("Сегодня", 12.0, theme.accent)),
        ));
    }

    controls.push(Box::new(previous));
    controls.push(Box::new(next));

    let header = Row::new(vec![Box::new(title), Box::new(Row::new(controls).gap(4.0).align(Center))])
        .width(inner_width)
        .height(32.0)
        .justify(SpaceBetween)
        .align(Center);

    let mut weekdays: Vec<Box<dyn Widget>> = Vec::new();

    for name in WEEKDAYS {
        weekdays.push(Box::new(
            Rectangle::new()
                .width(CELL_WIDTH)
                .height(20.0)
                .align_child(Center, Center)
                .child(label(name, 12.0, theme.muted)),
        ));
    }

    let weekdays = Row::new(weekdays).width(inner_width).justify(SpaceBetween);

    // 6 недель по 7 дней, дни соседних месяцев приглушены
    let first = days_from_civil(year, month, 1);
    let lead = (first + 3).rem_euclid(7);

    let mut rows: Vec<Box<dyn Widget>> = Vec::new();

    for row in 0..6_i64 {
        let mut cells: Vec<Box<dyn Widget>> = Vec::new();

        for column in 0..7_i64 {
            let offset = row * 7 + column - lead;

            let (cell_year, cell_month, cell_day) = civil(first + offset);

            let today = (cell_year, cell_month, cell_day) == (today_year, today_month, today_day);
            let key = format!("cal:{offset}");

            let (fill, color) = if today {
                (theme.accent, theme.on_accent)
            } else if cell_month != month {
                (Color::TRANSPARENT, theme.dim)
            } else {
                let hovered = ui.hovered.as_deref() == Some(key.as_str());

                (if hovered { theme.hover } else { Color::TRANSPARENT }, theme.text)
            };

            let text = if today { bold(cell_day.to_string(), 14.0, color) } else { label(cell_day.to_string(), 14.0, color) };

            cells.push(Box::new(
                Rectangle::new()
                    .width(CELL_WIDTH)
                    .height(CELL_HEIGHT)
                    .radius(8.0)
                    .fill(fill)
                    .on_hover(move |inside| Ui::set_hover(&key, inside))
                    .align_child(Center, Center)
                    .child(text),
            ));
        }

        rows.push(Box::new(Row::new(cells).width(inner_width).justify(SpaceBetween)));
    }

    let grid = Column::new(rows).gap(4.0);

    frame(theme, width, height, Column::new(children![header, weekdays, grid]).gap(6.0))
}
