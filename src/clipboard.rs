// попап буфера обмена: строка поиска и список последних текстов
use amane::{
    Center, Column, Pointer, Rectangle, Scroll, Service, Stack, Start, Text, TextInput, Widget,
    children,
};

use crate::clip_store::preview;
use crate::popup::{frame, inner};
use crate::state::{CLIPBOARD_INPUT, ClipboardHistory, Ui};
use crate::theme::Theme;
use crate::ui::{label, pad};

// сколько строк видно сразу: список не скроллится, а сдвигает окно по выбору
pub const ROWS: usize = 8;

const ROW_HEIGHT: f32 = 40.0;
const ROW_GAP: f32 = 6.0;
const SEARCH_HEIGHT: f32 = 36.0;

// 26 — рамка с отступами, 10 — промежуток между поиском и списком
pub const SIZE: (f32, f32) = (
    480.0,
    26.0 + SEARCH_HEIGHT + 10.0 + ROWS as f32 * ROW_HEIGHT + (ROWS - 1) as f32 * ROW_GAP,
);

pub fn view(theme: &Theme, ui: &Ui, width: f32, height: f32) -> Rectangle {
    let inner_width = inner(width);
    let list_height = height - 26.0 - SEARCH_HEIGHT - 10.0;

    let column = Column::new(children![
        search(theme, inner_width),
        list(theme, ui, inner_width, list_height),
    ])
    .gap(10.0);

    frame(theme, width, height, column)
}

fn search(theme: &Theme, width: f32) -> Rectangle {
    let input = TextInput::new(CLIPBOARD_INPUT)
        .size(13.0)
        .color(theme.text)
        .placeholder("Поиск в буфере…")
        .focused()
        .on_change(|query| Ui::clip_set_query(&query))
        .on_submit(|_| ClipboardHistory::pick_selected());

    let input = Rectangle::new()
        .width(width - 20.0)
        .height(SEARCH_HEIGHT)
        .translate(10.0, 0.0)
        .align_child(Start, Center)
        .child(input);

    Rectangle::new()
        .width(width)
        .height(SEARCH_HEIGHT)
        .radius(8.0)
        .fill(theme.surface)
        .border(1.0, theme.accent)
        .child(Stack::new(children![input]))
}

fn list(theme: &Theme, ui: &Ui, width: f32, height: f32) -> Rectangle {
    let area = Rectangle::new().width(width).height(height);

    let history = ClipboardHistory::read();
    let matches = history.matches(&ui.clip_query);

    if matches.is_empty() {
        let text = if ui.clip_query.trim().is_empty() {
            "Буфер пуст"
        } else {
            "Ничего не найдено"
        };

        return area.align_child(Center, Center).child(label(text, 13.0, theme.muted));
    }

    let rows: Vec<Box<dyn Widget>> = matches
        .iter()
        .enumerate()
        .skip(ui.clip_top)
        .take(ROWS)
        .map(|(index, text)| Box::new(row(theme, ui, index, text, width)) as Box<dyn Widget>)
        .collect();

    let total = ROW_HEIGHT * rows.len() as f32 + ROW_GAP * (rows.len() - 1) as f32;

    let column = Column::new(rows).width(width).height(total).gap(ROW_GAP);

    // колесо мыши двигает выбор так же, как стрелки; вверх — отрицательный scroll.y, как у громкости в баре
    area.on_scroll(|scroll: Scroll| Ui::clip_move(if scroll.y < 0.0 { -1 } else { 1 }))
        .child(column)
}

// нажатие копирует текст и закрывает окно
fn row(theme: &Theme, ui: &Ui, index: usize, text: &str, width: f32) -> Stack {
    let key = format!("clip:{index}");

    let selected = index == ui.clip_selected;
    let hovered = ui.hovered.as_deref() == Some(key.as_str());

    let fill = if selected || hovered { theme.hover } else { theme.surface };

    let text_width = width - 20.0;

    let line = Rectangle::new()
        .width(text_width)
        .height(18.0)
        .child(Text::new(preview(text, 200)).size(13.0).color(theme.text).elide());

    let body = Rectangle::new()
        .width(width)
        .height(ROW_HEIGHT)
        .radius(10.0)
        .fill(fill)
        .padding(pad(10.0))
        .align_child(Start, Center)
        .child(line);

    let body = if selected { body.border(1.0, theme.accent) } else { body };

    let click = Rectangle::new()
        .width(width)
        .height(ROW_HEIGHT)
        .cursor(Pointer)
        .on_hover(move |inside| Ui::set_hover(&key, inside))
        .on_click(move |_| ClipboardHistory::pick(index));

    Stack::new(vec![Box::new(body) as Box<dyn Widget>, Box::new(click)])
        .width(width)
        .height(ROW_HEIGHT)
}