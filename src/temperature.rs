use amane::{
    Canvas, Center, Column, Path, Rectangle, Row, Service, Shape, SpaceBetween, children, shapes,
};

use crate::popup::{frame, inner};
use crate::state::{Temp, Ui};
use crate::theme::Theme;
use crate::ui::{bold, icon, label};

const HEADER_HEIGHT: f32 = 36.0;
const LABEL_HEIGHT: f32 = 16.0;
const GAP: f32 = 10.0;

pub fn view(theme: &Theme, _ui: &Ui, width: f32, height: f32) -> Rectangle {
    let inner_width = inner(width);
    // всё содержимое должно влезть в высоту рамки без её padding и border
    let chart_height = (height - 26.0 - HEADER_HEIGHT - LABEL_HEIGHT - GAP * 2.0).max(0.0);

    // состояние читаем один раз и отпускаем guard до сборки виджетов
    let temp = Temp::read();
    let celsius = temp.celsius;
    let min = temp.min();
    let max = temp.max();
    let chart_points = points(temp.history(), inner_width, chart_height);
    drop(temp);

    let color = if celsius >= 80 { theme.red } else { theme.text };

    let title = Row::new(children![
        icon("\u{f050f}", 18.0, color),
        label("Температура", 14.0, theme.text),
    ])
    .gap(8.0)
    .align(Center);

    let header = Row::new(children![title, bold(format!("{celsius}°C"), 24.0, color)])
        .width(inner_width)
        .height(HEADER_HEIGHT)
        .justify(SpaceBetween)
        .align(Center);

    // меньше двух точек — путь не рисуем: одиночный move_to дал бы точку
    let mut shapes: Vec<Box<dyn Shape>> = shapes![];
    if chart_points.len() >= 2 {
        let mut path = Path::new().move_to(chart_points[0].0, chart_points[0].1);

        for (x, y) in &chart_points[1..] {
            path = path.line_to(*x, *y);
        }

        shapes.push(Box::new(path.stroke(2.0, theme.accent)));
    }

    let chart = Canvas::new()
        .width(inner_width)
        .height(chart_height)
        .shapes(shapes);

    let labels = Row::new(children![
        label(format!("мин {min}°"), 12.0, theme.muted),
        label(format!("макс {max}°"), 12.0, theme.muted),
    ])
    .width(inner_width)
    .height(LABEL_HEIGHT)
    .justify(SpaceBetween);

    frame(
        theme,
        width,
        height,
        Column::new(children![header, chart, labels])
            .gap(GAP)
            .width(inner_width),
    )
}

// точки графика: x по номеру замера, y инвертирован (горячее сверху)
pub fn points(values: &[i32], width: f32, height: f32) -> Vec<(f32, f32)> {
    if values.len() < 2 {
        return Vec::new();
    }

    let min = values.iter().copied().min().unwrap_or(0);
    let max = values.iter().copied().max().unwrap_or(0);
    let span = (max - min) as f32;
    let last = (values.len() - 1) as f32;

    values
        .iter()
        .enumerate()
        .map(|(index, &value)| {
            let x = width * index as f32 / last;
            // плоский ряд рисуем посередине, иначе деление на ноль
            let y = if span == 0.0 {
                height / 2.0
            } else {
                height * (max - value) as f32 / span
            };

            (x, y)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::points;

    #[test]
    fn empty_and_single_have_no_points() {
        assert!(points(&[], 100.0, 50.0).is_empty());
        assert!(points(&[42], 100.0, 50.0).is_empty());
    }

    #[test]
    fn flat_row_is_middle() {
        let result = points(&[50, 50, 50], 100.0, 40.0);

        assert_eq!(result.len(), 3);
        assert!(result.iter().all(|&(_, y)| y == 20.0));
    }

    #[test]
    fn monotonic_row_is_inverted() {
        // холодное первое — внизу, горячее последнее — вверху
        assert_eq!(
            points(&[30, 40, 50], 100.0, 40.0),
            vec![(0.0, 40.0), (50.0, 20.0), (100.0, 0.0)]
        );
    }

    #[test]
    fn edges_span_full_width() {
        let result = points(&[10, 20], 200.0, 100.0);

        assert_eq!(result[0], (0.0, 100.0));
        assert_eq!(result[1], (200.0, 0.0));
    }
}
