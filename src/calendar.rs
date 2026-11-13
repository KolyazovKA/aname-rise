use std::time::Duration;

use amane::{
    Arc, Canvas, Cap, Center, Color, Column, Parent, Pointer, Rectangle, Row, Service, Shape,
    SpaceBetween, Stack, Start, TextInput, Widget, children, shapes,
};

use crate::popup::{frame, inner};
use crate::state::{Clock, TIMER_INPUT, Timer, Ui, civil, days_from_civil, display_seconds};
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

// прежняя ширина окна календаря: из неё получается ширина сетки
const CALENDAR_WIDTH: f32 = 308.0;
const TIMER_WIDTH: f32 = 120.0;

pub fn view(theme: &Theme, ui: &Ui, width: f32, height: f32) -> Rectangle {
    // состояние таймера читаем один раз и отпускаем guard до сборки виджетов
    let timer = Timer::read();
    let remaining = timer.remaining();
    let running = timer.running();
    let total = timer.total();
    drop(timer);

    let divider = Rectangle::new().width(1.0).height(Parent).fill(theme.border);

    let row = Row::new(children![
        calendar(theme, ui),
        divider,
        timer_column(theme, ui, remaining, running, total)
    ])
    .gap(16.0)
    .align(Center);

    frame(theme, width, height, row)
}

// календарь: прежняя вёрстка на фиксированной ширине, чтобы не растягивался на всё окно
fn calendar(theme: &Theme, ui: &Ui) -> Column {
    let inner_width = inner(CALENDAR_WIDTH);

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

    Column::new(children![header, weekdays, grid]).gap(6.0).width(inner_width)
}

// таймер: круг с остатком и временем внутри, под ним play/pause и reset
fn timer_column(theme: &Theme, ui: &Ui, remaining: Duration, running: bool, total: Duration) -> Column {
    let sweep = 360.0 * remaining.as_secs_f32() / total.as_secs_f32();

    // дуги без stroke не рисуются: фон — полный круг, прогресс — остаток
    let ring = Canvas::new()
        .width(TIMER_WIDTH)
        .height(TIMER_WIDTH)
        .shapes(shapes![
            Arc::new().stroke(6.0, theme.border),
            Arc::new().start(0.0).sweep(sweep).stroke(6.0, theme.accent).cap(Cap::Round),
        ]);

    // округление вверх: сразу после старта показываем 25:00, а не 24:59
    let seconds = display_seconds(remaining);
    let time = format!("{:02}:{:02}", seconds / 60, seconds % 60);

    let face = Stack::new(children![
        ring,
        Rectangle::new()
            .width(Parent)
            .height(Parent)
            .align_child(Center, Center)
            .child(bold(time, 20.0, theme.text)),
    ])
    .width(TIMER_WIDTH)
    .height(TIMER_WIDTH);

    let play = icon_button(
        ui,
        theme,
        "timer:play",
        if running { "\u{f03e4}" } else { "\u{f040a}" },
        28.0,
        theme.text,
    )
    .on_click(|_| Timer::toggle());

    let reset = icon_button(ui, theme, "timer:reset", "\u{f099b}", 28.0, theme.text)
        .on_click(|_| Timer::reset());

    // поле минут: Enter меняет длительность, пустой или нечисловой ввод ничего не делает
    let mut input = TextInput::new(TIMER_INPUT)
        .width(Parent)
        .size(13.0)
        .color(theme.text)
        .placeholder("мин")
        .on_submit(|text| {
            if let Ok(minutes) = text.trim().parse::<u32>() {
                Timer::set_minutes(minutes);
            }

            TextInput::set_text(TIMER_INPUT, "");
            Ui::write().timer_editing = false;
            // shortcut: фокус поля не снимается после Enter (focus::clear в amane не экспортируется), цифры продолжают попадать в поле; снять, когда в SDK появится TextInput::blur()
        });

    if ui.timer_editing {
        input = input.focused();
    }

    let border = if ui.timer_editing { theme.accent } else { theme.border };

    let field = Rectangle::new()
        .width(TIMER_WIDTH)
        .height(28.0)
        .radius(8.0)
        .fill(theme.bg)
        .border(1.0, border)
        .child(Stack::new(children![
            // TextInput::height() — высота строки, поэтому обёртка задаёт высоту поля
            Rectangle::new()
                .width(TIMER_WIDTH - 16.0)
                .height(Parent)
                .translate(8.0, 0.0)
                .align_child(Start, Center)
                .child(input),
            // ловушка последней: TextInput забирает клик себе, окно иначе не узнает о начале ввода
            Rectangle::new()
                .width(Parent)
                .height(Parent)
                .cursor(Pointer)
                .on_click(|_| Ui::write().timer_editing = true),
        ]));

    Column::new(children![
        face,
        Row::new(children![play, reset]).gap(12.0).align(Center),
        field
    ])
    .gap(8.0)
    .width(TIMER_WIDTH)
    .align(Center)
}
