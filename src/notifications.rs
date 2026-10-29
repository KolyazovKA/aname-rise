// тосты (выезжают справа) + окно центра уведомлений
use std::collections::HashSet;
use std::thread;
use std::time::{Duration, Instant, SystemTime};

use amane::{
    Animation, Center, Column, Color, Easing, Gradient, Horizontal, Image, Layer, LayerWindow,
    Margin, Monitor, Parent, Pointer, Rectangle, Row, ScrollArea, Service, SpaceBetween, Start,
    Text, Vertical, Weight, Widget, children,
};

use amane::Notifications;

use crate::popup::{frame, inner};
use crate::state::{NotificationHistory, NotificationHistoryEntry, Ui};
use crate::theme::Theme;
use crate::ui::{bold, label};

// ---------------------------------------------------------------- константы

const TOAST_WIDTH: f32 = 340.0;
const TOAST_HEIGHT: f32 = 92.0;
const TOAST_GAP: f32 = 8.0;
const TOAST_RADIUS: f32 = 12.0;
const BORDER_THICKNESS: f32 = 3.0;
const MAX_TOASTS: usize = 5;
const TOAST_DURATION: Duration = Duration::from_secs(5);
const ENTER_DURATION: Duration = Duration::from_millis(300);
const LEAVE_DURATION: Duration = Duration::from_millis(400);

// ---------------------------------------------------------------- состояние тостов

#[derive(Clone, Copy, PartialEq)]
enum Phase {
    Entering,
    Visible,
    Leaving,
}

struct Toast {
    id: u32,
    app_name: String,
    summary: String,
    body: String,
    image: String,
    urgency: amane::Urgency,
    phase: Phase,
    phase_start: Instant,
    anim: Animation<f32>,
}

pub struct ToastState {
    toasts: Vec<Toast>,
}

impl Service for ToastState {
    fn new() -> Self {
        Self { toasts: Vec::new() }
    }

    fn interval() -> Duration {
        Duration::from_millis(100)
    }

    fn update(&mut self) -> bool {
        self.poll()
    }

    fn listen() {
        loop {
            thread::sleep(Self::interval());
            Self::write().poll();
        }
    }
}

impl ToastState {
    fn poll(&mut self) -> bool {
        let now = Instant::now();
        let mut changed = false;

        // новые уведомления
        let active_ids: HashSet<u32> = self.toasts.iter().map(|t| t.id).collect();

        for n in Notifications::read().list() {
            if !active_ids.contains(&n.id()) {
                self.push(n);
                changed = true;
            }
        }

        // обновляем фазы и анимации
        for toast in &mut self.toasts {
            let elapsed = now.duration_since(toast.phase_start);

            match toast.phase {
                Phase::Entering if elapsed >= ENTER_DURATION => {
                    toast.phase = Phase::Visible;
                    toast.phase_start = now;
                    toast.anim = Animation::new(0.0);
                    changed = true;
                }
                Phase::Visible if elapsed >= TOAST_DURATION => {
                    toast.phase = Phase::Leaving;
                    toast.phase_start = now;
                    let mut a = Animation::new(0.0)
                        .duration(LEAVE_DURATION)
                        .easing(Easing::InOut);
                    a.to(TOAST_WIDTH);
                    toast.anim = a;
                    changed = true;
                }
                _ => {}
            }

            // читаем анимацию (просит кадры)
            let _ = toast.anim.value();
        }

        // удаляем ушедшие (dismiss из демона, чтобы не пушились снова)
        self.toasts.retain(|t| {
            let alive = t.phase != Phase::Leaving || now.duration_since(t.phase_start) < LEAVE_DURATION;
            if !alive {
                Notifications::dismiss(t.id);
            }
            alive
        });

        changed
    }
}

impl ToastState {
    fn push(&mut self, n: &amane::Notification) {
        if self.toasts.len() >= MAX_TOASTS {
            if let Some(removed) = self.toasts.pop() {
                Notifications::dismiss(removed.id);
            }
        }

        let mut anim = Animation::new(TOAST_WIDTH)
            .duration(ENTER_DURATION)
            .easing(Easing::Out);
        anim.to(0.0);

        let toast = Toast {
            id: n.id(),
            app_name: n.app_name().to_string(),
            summary: n.summary().to_string(),
            body: n.body().to_string(),
            image: n.image().to_string(),
            urgency: n.urgency(),
            phase: Phase::Entering,
            phase_start: Instant::now(),
            anim,
        };

        NotificationHistory::push(n);
        self.toasts.insert(0, toast);
    }

    pub fn dismiss(&mut self, id: u32) {
        if let Some(toast) = self.toasts.iter_mut().find(|t| t.id == id) {
            toast.phase = Phase::Leaving;
            toast.phase_start = Instant::now();
            let mut a = Animation::new(0.0)
                .duration(LEAVE_DURATION)
                .easing(Easing::InOut);
            a.to(TOAST_WIDTH);
            toast.anim = a;
        }
    }

    pub fn window_height(&self) -> f32 {
        if self.toasts.is_empty() {
            return 0.0;
        }
        self.toasts.len() as f32 * (TOAST_HEIGHT + TOAST_GAP) - TOAST_GAP
    }
}

// ---------------------------------------------------------------- вид тостов

pub fn toast_view(_monitor: &Monitor) -> LayerWindow {
    let state = ToastState::read();
    let open = !state.toasts.is_empty();

    let cards: Vec<Box<dyn Widget>> = state
        .toasts
        .iter()
        .map(|t| Box::new(toast_card(t)) as Box<dyn Widget>)
        .collect();

    LayerWindow::new()
        .width(TOAST_WIDTH + 16.0)
        .height(if open { state.window_height() } else { 1.0 })
        .anchor_vertical(Vertical::Top)
        .anchor_horizontal(Horizontal::Right)
        .margin(Margin { top: 8, right: 8, bottom: 0, left: 0 })
        .layer(Layer::Overlay)
        .namespace("toasts")
        .visible(open)
        .child(
            Rectangle::new()
                .width(Parent)
                .height(Parent)
                .fill(Color::TRANSPARENT)
                .child(Column::new(cards).gap(TOAST_GAP).align(Start)),
        )
}

fn toast_card(toast: &Toast) -> Rectangle {
    let theme = crate::theme::theme();
    let translate_x = toast.anim.value();

    let border_fill = if toast.urgency == amane::Urgency::Critical {
        Gradient::linear(135.0, [(0.0, Color::from("#fb4934")), (1.0, Color::from("#fb4934"))])
    } else {
        Gradient::linear(135.0, [(0.0, Color::from("#fabd2f")), (1.0, Color::from("#b8bb26"))])
    };

    Rectangle::new()
        .width(TOAST_WIDTH)
        .height(TOAST_HEIGHT)
        .radius(TOAST_RADIUS)
        .fill(border_fill)
        .translate(translate_x, 0.0)
        .child(build_inner(&theme, toast))
}

fn build_inner(theme: &Theme, toast: &Toast) -> Rectangle {
    let id = toast.id;
    let has_image = !toast.image.is_empty();

    let top_row = if has_image {
        let img_path = toast.image.clone();
        Row::new(children![
            Rectangle::new()
                .width(32.0)
                .height(32.0)
                .radius(16.0)
                .fill(Image::contain(&img_path)),
            Rectangle::new().width(6.0).height(1.0),
            bold(&toast.app_name, 12.0, theme.muted),
        ])
        .gap(0.0)
        .align(Center)
    } else {
        Row::new(vec![Box::new(bold(&toast.app_name, 12.0, theme.muted))]).align(Center)
    };

    let summary = Rectangle::new()
        .width(TOAST_WIDTH - BORDER_THICKNESS * 2.0 - 20.0)
        .height(18.0)
        .align_child(Start, Center)
        .child(
            Text::new(&toast.summary)
                .size(14.0)
                .color(theme.text)
                .weight(Weight::Bold)
                .elide(),
        );

    let body_widget = if toast.body.is_empty() {
        Rectangle::new().width(1.0).height(1.0)
    } else {
        Rectangle::new()
            .width(TOAST_WIDTH - BORDER_THICKNESS * 2.0 - 20.0)
            .height(28.0)
            .align_child(Start, Center)
            .child(
                Text::new(&toast.body)
                    .size(13.0)
                    .color(theme.muted)
                    .elide(),
            )
    };

    let content = Rectangle::new()
        .width(TOAST_WIDTH - BORDER_THICKNESS * 2.0)
        .height(TOAST_HEIGHT - BORDER_THICKNESS * 2.0)
        .radius(TOAST_RADIUS - BORDER_THICKNESS)
        .fill(theme.bg)
        .translate(BORDER_THICKNESS, BORDER_THICKNESS)
        .padding(amane::Padding {
            top: 8.0,
            right: 10.0,
            bottom: 8.0,
            left: 10.0,
        })
        .child(Column::new(children![top_row, summary, body_widget]));

    content
        .cursor(Pointer)
        .on_click(move |_| {
            Notifications::click(id);
            ToastState::write().dismiss(id);
        })
}

// ---------------------------------------------------------------- окно центра уведомлений

pub fn center_view(theme: &Theme, _ui: &Ui, width: f32, height: f32) -> Rectangle {
    let inner_width = inner(width);
    let history = NotificationHistory::read();
    let entries = history.list();

    let head = Row::new(children![
        bold("Уведомления", 16.0, theme.text),
        Rectangle::new().width(1.0).height(1.0),
        clear_button(theme),
    ])
    .justify(SpaceBetween)
    .align(Center)
    .width(inner_width)
    .height(28.0);

    if entries.is_empty() {
        return frame(
            theme,
            width,
            height,
            Column::new(children![
                head,
                Rectangle::new()
                    .width(inner_width)
                    .height(height - 26.0 - 28.0 - 10.0)
                    .align_child(Center, Center)
                    .child(label("Нет уведомлений", 13.0, theme.muted)),
            ])
            .gap(10.0),
        );
    }

    let list_height = height - 26.0 - 28.0 - 10.0;
    let list = notification_list(theme, entries, inner_width, list_height);

    frame(
        theme,
        width,
        height,
        Column::new(children![head, list]).gap(10.0),
    )
}

fn clear_button(theme: &Theme) -> Rectangle {
    let key = "notif:clear";
    let hovered = Ui::read().hovered.as_deref() == Some(key);

    let fill = if hovered { theme.hover } else { Color::TRANSPARENT };

    Rectangle::new()
        .width(60.0)
        .height(24.0)
        .radius(8.0)
        .fill(fill)
        .cursor(Pointer)
        .on_hover(move |inside| {
            let mut ui = Ui::write();
            ui.hovered = if inside {
                Some(String::from(key))
            } else {
                None
            };
        })
        .on_click(|_| {
            Notifications::clear();
            NotificationHistory::clear();
        })
        .align_child(Center, Center)
        .child(label("Очистить", 12.0, theme.accent))
}

fn notification_list(
    theme: &Theme,
    entries: &[NotificationHistoryEntry],
    width: f32,
    height: f32,
) -> Rectangle {
    const CARD_H: f32 = 72.0;
    const GAP: f32 = 6.0;

    let mut cards: Vec<Box<dyn Widget>> = Vec::new();
    let mut total = 0.0;

    for entry in entries.iter().rev() {
        if !cards.is_empty() {
            total += GAP;
        }
        cards.push(Box::new(notif_card(theme, entry, width)));
        total += CARD_H;
    }

    let column = Column::new(cards).width(width).height(total).gap(GAP);

    Rectangle::new()
        .width(width)
        .height(height)
        .clip()
        .child(ScrollArea::new("notif-list", column))
}

fn notif_card(theme: &Theme, entry: &NotificationHistoryEntry, width: f32) -> Rectangle {
    let n = &entry.notification;
    let id = n.id();
    let key = format!("notif:{id}");
    let hovered = Ui::read().hovered.as_deref() == Some(key.as_str());

    let fill = if hovered { theme.hover } else { theme.surface };
    let has_image = !n.image().is_empty();

    let time_str = format_time(&entry.timestamp);
    let text_width = width - 20.0 - if has_image { 32.0 + 8.0 } else { 0.0 };

    let top_line = Row::new(children![
        bold(n.app_name(), 11.0, theme.muted),
        Rectangle::new().width(4.0).height(1.0),
        label(&time_str, 11.0, theme.dim),
    ])
    .gap(0.0)
    .align(Center);

    let summary = Rectangle::new()
        .width(text_width)
        .height(16.0)
        .align_child(Start, Center)
        .child(
            Text::new(n.summary())
                .size(13.0)
                .color(theme.text)
                .weight(Weight::Bold)
                .elide(),
        );

    let body = Rectangle::new()
        .width(text_width)
        .height(24.0)
        .align_child(Start, Center)
        .child(Text::new(n.body()).size(12.0).color(theme.muted).elide());

    let info = Column::new(children![top_line, summary, body]);

    let row = if has_image {
        let img_path = n.image().to_string();
        Row::new(children![
            Rectangle::new()
                .width(32.0)
                .height(32.0)
                .radius(8.0)
                .fill(Image::contain(&img_path)),
            Rectangle::new()
                .width(text_width)
                .height(1.0)
                .align_child(Start, Center)
                .child(info),
        ])
        .gap(8.0)
        .align(Center)
    } else {
        Row::new(children![
            Rectangle::new()
                .width(text_width)
                .height(1.0)
                .align_child(Start, Center)
                .child(info),
        ])
        .align(Center)
    };

    let k = key.clone();

    Rectangle::new()
        .width(width)
        .height(72.0)
        .radius(10.0)
        .fill(fill)
        .padding(amane::Padding {
            top: 8.0,
            right: 10.0,
            bottom: 8.0,
            left: 10.0,
        })
        .cursor(Pointer)
        .on_hover(move |inside| {
            let mut ui = Ui::write();
            if inside {
                ui.hovered = Some(k.clone());
            } else if ui.hovered.as_deref() == Some(k.as_str()) {
                ui.hovered = None;
            }
        })
        .on_click(move |_| {
            Notifications::dismiss(id);
        })
        .child(row)
}

fn format_time(ts: &SystemTime) -> String {
    let dur = ts
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = dur.as_secs();
    let mins = secs / 60;
    let hrs = mins / 60;
    let mins = mins % 60;

    if hrs > 0 {
        format!("{:02}:{:02}", hrs, mins)
    } else {
        format!("{} мин назад", secs / 60)
    }
}