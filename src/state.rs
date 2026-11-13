// сервисы: всё, что бар и окна читают через `Service::read()`
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use amane::{Bluetooth, Network, Notification, Service, TextInput};

use crate::clip_store::Store;

pub const PASSWORD_INPUT: &str = "wifi-password";
pub const CLIPBOARD_INPUT: &str = "clipboard-search";
pub const TIMER_INPUT: &str = "timer-minutes";

// ---------------------------------------------------------------- окна

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Popup {
    Calendar,
    Bluetooth,
    Wifi,
    Audio,
    Battery,
    Notifications,
    Clipboard,
    Temperature,
}

// какое окно открыто и на каком мониторе, плюс мелкое состояние интерфейса
pub struct Ui {
    pub popup: Option<Popup>,
    pub monitor: String,
    pub hovered: Option<String>,
    // месяц календаря относительно текущего
    pub month: i64,
    pub password_for: Option<String>,
    pub show_password: bool,
    pub joining: Option<String>,
    // ползунок, который сейчас тянут: ключ, значение, когда
    pub slider: Option<(String, u8, Instant)>,
    // буфер обмена: запрос поиска, выбранная строка среди найденных, первая видимая строка
    pub clip_query: String,
    pub clip_selected: usize,
    pub clip_top: usize,
    // в поле минут календаря сейчас вводят
    pub timer_editing: bool,
}

impl Service for Ui {
    fn new() -> Self {
        Self {
            popup: None,
            monitor: String::new(),
            hovered: None,
            month: 0,
            password_for: None,
            show_password: false,
            joining: None,
            slider: None,
            clip_query: String::new(),
            clip_selected: 0,
            clip_top: 0,
            timer_editing: false,
        }
    }

    // меняется только от ввода
    fn listen() {}
}

impl Ui {
    pub fn toggle(popup: Popup, monitor: &str) {
        let mut ui = Self::write();

        if ui.popup == Some(popup) && ui.monitor == monitor {
            ui.close();

            return;
        }

        ui.close();
        ui.popup = Some(popup);
        ui.monitor = monitor.to_string();

        match popup {
            Popup::Wifi => Network::scan(),
            Popup::Bluetooth => Bluetooth::start_scan(),
            _ => {}
        }
    }

    pub fn close_all() {
        Self::write().close();
    }

    fn close(&mut self) {
        if self.popup == Some(Popup::Bluetooth) {
            Bluetooth::stop_scan();
        }

        self.popup = None;
        self.month = 0;
        self.password_for = None;
        self.show_password = false;

        self.clip_query.clear();
        self.clip_selected = 0;
        self.clip_top = 0;
        self.timer_editing = false;

        TextInput::set_text(PASSWORD_INPUT, "");
        TextInput::set_text(CLIPBOARD_INPUT, "");
        TextInput::set_text(TIMER_INPUT, "");
    }

    pub fn set_hover(key: &str, inside: bool) {
        let mut ui = Self::write();

        if inside {
            ui.hovered = Some(key.to_string());
        } else if ui.hovered.as_deref() == Some(key) {
            ui.hovered = None;
        }
    }

    // новый запрос поиска: выбор и прокрутка возвращаются к началу списка
    pub fn clip_set_query(query: &str) {
        let mut ui = Self::write();

        ui.clip_query = query.to_string();
        ui.clip_selected = 0;
        ui.clip_top = 0;
    }

    // стрелки и колесо мыши: сдвиг выбора, видимое окно списка едет следом
    pub fn clip_move(delta: isize) {
        let mut ui = Self::write();

        if ui.popup != Some(Popup::Clipboard) {
            return;
        }

        let count = ClipboardHistory::read().matches(&ui.clip_query).len();

        if count == 0 {
            return;
        }

        let next = (ui.clip_selected as isize + delta).clamp(0, count as isize - 1) as usize;

        ui.clip_selected = next;

        if next < ui.clip_top {
            ui.clip_top = next;
        } else if next >= ui.clip_top + crate::clipboard::ROWS {
            ui.clip_top = next + 1 - crate::clipboard::ROWS;
        }
    }

    // значение ползунка: то, что тянут сейчас, или настоящее
    pub fn slider_value(&self, key: &str, actual: u8) -> u8 {
        match &self.slider {
            Some((name, value, when))
                if name == key && when.elapsed() < Duration::from_millis(600) =>
            {
                *value
            }
            _ => actual,
        }
    }

    // true, если значение изменилось и его надо отправить
    pub fn drag(key: &str, value: u8) -> bool {
        let mut ui = Self::write();

        let same = matches!(&ui.slider, Some((name, old, _)) if name == key && *old == value);

        ui.slider = Some((key.to_string(), value, Instant::now()));

        !same
    }
}

// ---------------------------------------------------------------- время

pub const DAY: i64 = 86_400;

pub struct Clock {
    offset: i64,
    ticks: u32,
    local: i64,
}

impl Service for Clock {
    fn new() -> Self {
        let mut clock = Self {
            offset: read_offset(),
            ticks: 0,
            local: 0,
        };

        clock.update();

        clock
    }

    fn update(&mut self) -> bool {
        let minute = self.local.div_euclid(60);

        self.ticks += 1;

        // часовой пояс спрашиваем раз в 10 минут, ради перехода на летнее время
        if self.ticks >= 600 {
            self.ticks = 0;
            self.offset = read_offset();
        }

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::ZERO)
            .as_secs() as i64;

        self.local = now + self.offset;

        self.local.div_euclid(60) != minute
    }
}

impl Clock {
    pub fn time(&self) -> String {
        let today = self.local.rem_euclid(DAY);

        format!("{:02}:{:02}", today / 3600, today % 3600 / 60)
    }

    // год, месяц с 1, день с 1
    pub fn today(&self) -> (i64, i64, i64) {
        civil(self.local.div_euclid(DAY))
    }

    pub fn offset(&self) -> i64 {
        self.offset
    }
}

// Форматирование времени уведомления: «N мин назад» или HH:MM
pub fn format_notification_time(ts: SystemTime, offset: i64) -> String {
    let ts_secs = ts
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    let now_secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    let diff = (now_secs + offset).saturating_sub(ts_secs + offset);

    if diff < 60 {
        return "только что".to_string();
    }
    if diff < 3600 {
        return format!("{} мин назад", diff / 60);
    }

    // Показываем HH:MM времени уведомления с учётом часового пояса
    let local_ts = ts_secs + offset;
    let today_secs = local_ts.rem_euclid(DAY);
    format!("{:02}:{:02}", today_secs / 3600, today_secs % 3600 / 60)
}

// "+0200" -> секунды
fn read_offset() -> i64 {
    let zone = amane::output("date +%z");
    let sign = if zone.starts_with('-') { -1 } else { 1 };

    let digits: String = zone.chars().filter(char::is_ascii_digit).collect();
    if digits.len() < 4 {
        return 0;
    }

    let hh: i64 = digits[..2].parse().unwrap_or(0);
    let mm: i64 = digits[2..4].parse().unwrap_or(0);

    sign * (hh * 3600 + mm * 60)
}

// дни с 1970-01-01 -> (год, месяц, день)
pub fn civil(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };

    (if month <= 2 { year + 1 } else { year }, month, day)
}

pub fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let yoe = year.rem_euclid(400);
    let mp = if month > 2 { month - 3 } else { month + 9 };
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;

    era * 146_097 + doe - 719_468
}

// ---------------------------------------------------------------- таймер

pub const TIMER_TOTAL: Duration = Duration::from_secs(25 * 60);

// отсчёт по дедлайну: тики не копят погрешность, а окно перерисовывается только со сменой секунды
pub struct Timer {
    // выбранная длительность: от неё считаются остаток, кольцо и reset
    total: Duration,
    // остаток, пока таймер стоит
    remaining: Duration,
    // момент окончания, пока идёт
    deadline: Option<Instant>,
    // последняя показанная секунда: update() молчит, пока она не сменилась
    shown: u64,
}

impl Service for Timer {
    fn new() -> Self {
        Self {
            total: TIMER_TOTAL,
            remaining: TIMER_TOTAL,
            deadline: None,
            shown: display_seconds(TIMER_TOTAL),
        }
    }

    fn interval() -> Duration {
        Duration::from_millis(100)
    }

    fn update(&mut self) -> bool {
        // дедлайн прошёл: останавливаемся на нуле
        let now = Instant::now();
        let expired = self.deadline.is_some_and(|deadline| now >= deadline);

        if expired {
            self.deadline = None;
            self.remaining = Duration::ZERO;
        }

        let shown = display_seconds(self.remaining());

        if shown == self.shown {
            return false;
        }

        self.shown = shown;

        true
    }
}

impl Timer {
    // play/pause одной кнопкой; на нуле запуск не начинается — нужен reset
    pub fn toggle() {
        let mut timer = Self::write();

        if let Some(deadline) = timer.deadline {
            timer.remaining = deadline.saturating_duration_since(Instant::now());
            timer.deadline = None;
        } else if timer.remaining > Duration::ZERO {
            timer.deadline = Some(Instant::now() + timer.remaining);
        }
    }

    pub fn reset() {
        let mut timer = Self::write();

        timer.remaining = timer.total;
        timer.deadline = None;
        timer.shown = display_seconds(timer.total);
    }

    // смена длительности сбрасывает и останавливает таймер
    pub fn set_minutes(minutes: u32) {
        let total = Duration::from_secs(u64::from(minutes.clamp(1, 99)) * 60);
        let mut timer = Self::write();

        timer.total = total;
        timer.remaining = total;
        timer.deadline = None;
        timer.shown = display_seconds(total);
    }

    pub fn total(&self) -> Duration {
        self.total
    }

    // остаток считается от текущего времени, поэтому задержки отрисовки его не сдвигают
    pub fn remaining(&self) -> Duration {
        match self.deadline {
            Some(deadline) => deadline.saturating_duration_since(Instant::now()),
            None => self.remaining,
        }
    }

    pub fn running(&self) -> bool {
        self.deadline.is_some()
    }
}

// секунды для показа с округлением вверх: 25:00 не превращается в 24:59 сразу после старта
pub fn display_seconds(left: Duration) -> u64 {
    left.as_secs() + u64::from(left.subsec_nanos() > 0)
}

// ---------------------------------------------------------------- раскладка

pub struct KbLayout {
    pub index: usize,
}

impl Service for KbLayout {
    fn new() -> Self {
        Self {
            index: read_layout(),
        }
    }

    fn listen() {
        for line in amane::lines("niri msg event-stream") {
            if line.to_lowercase().contains("layout") {
                Self::write().index = read_layout();
            }
        }
    }
}

// {"names":["English (US)","Russian"],"current_idx":0}
fn read_layout() -> usize {
    let json = amane::output("niri msg -j keyboard-layouts");

    let Some(at) = json.find("current_idx") else {
        return 0;
    };

    let digits: String = json[at..]
        .chars()
        .skip_while(|c| !c.is_ascii_digit())
        .take_while(char::is_ascii_digit)
        .collect();

    digits.parse().unwrap_or(0)
}

// ---------------------------------------------------------------- температура

// 60 замеров по 5 с — последние 5 минут
const TEMP_HISTORY_MAX: usize = 60;

pub struct Temp {
    pub celsius: i32,
    path: Option<PathBuf>,
    // старые первыми, новые в конце
    history: Vec<i32>,
}

impl Service for Temp {
    fn new() -> Self {
        let path = find_sensor();

        let celsius = path.as_ref().map_or(0, read_temp);

        Self {
            celsius,
            path,
            history: vec![celsius],
        }
    }

    fn interval() -> Duration {
        Duration::from_secs(5)
    }

    fn update(&mut self) -> bool {
        let Some(path) = &self.path else {
            return false;
        };

        self.record(read_temp(path));

        // график обновляется на каждом тике, пока окно открыто
        true
    }
}

impl Temp {
    fn record(&mut self, value: i32) {
        self.celsius = value;
        self.history.push(value);

        if self.history.len() > TEMP_HISTORY_MAX {
            self.history.remove(0);
        }
    }

    pub fn history(&self) -> &[i32] {
        &self.history
    }

    pub fn min(&self) -> i32 {
        self.history.iter().copied().min().unwrap_or(self.celsius)
    }

    pub fn max(&self) -> i32 {
        self.history.iter().copied().max().unwrap_or(self.celsius)
    }
}

// coretemp, как у waybar; иначе первая термозона
fn find_sensor() -> Option<PathBuf> {
    if let Ok(entries) = fs::read_dir("/sys/class/hwmon") {
        for entry in entries.flatten() {
            let name = fs::read_to_string(entry.path().join("name")).unwrap_or_default();

            if name.trim() == "coretemp" {
                return Some(entry.path().join("temp1_input"));
            }
        }
    }

    let zone = PathBuf::from("/sys/class/thermal/thermal_zone0/temp");

    zone.exists().then_some(zone)
}

fn read_temp(path: &PathBuf) -> i32 {
    let milli: i32 = fs::read_to_string(path)
        .unwrap_or_default()
        .trim()
        .parse()
        .unwrap_or(0);

    milli / 1000
}

// ---------------------------------------------------------------- VPN

pub struct Vpn {
    pub active: bool,
}

impl Service for Vpn {
    fn new() -> Self {
        Self { active: vpn_up() }
    }

    fn interval() -> Duration {
        Duration::from_secs(3)
    }

    fn update(&mut self) -> bool {
        let now = vpn_up();
        let changed = now != self.active;

        self.active = now;

        changed
    }
}

// happ-xray (happ) и обычные tun/wireguard-интерфейсы
fn vpn_up() -> bool {
    let Ok(entries) = fs::read_dir("/sys/class/net") else {
        return false;
    };

    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();

        for prefix in [
            "happ",
            "xray",
            "wg",
            "tailscale",
            "ppp",
            "proton",
            "nordlynx",
        ] {
            if name.starts_with(prefix) {
                return true;
            }
        }
        // tun0 — VPN; tunl0 — IPIP-туннель ядра (Docker/K8s), не VPN
        if name.starts_with("tun") && !name.starts_with("tunl") {
            return true;
        }
    }

    false
}

// ---------------------------------------------------------------- звук

#[derive(Clone, PartialEq)]
pub struct Sink {
    pub name: String,
    pub description: String,
}

#[derive(Clone, PartialEq)]
pub struct Stream {
    pub id: u32,
    pub name: String,
    pub volume: u8,
    pub muted: bool,
}

// устройства вывода и приложения, которые сейчас играют (через pactl)
pub struct Mixer {
    pub sinks: Vec<Sink>,
    pub default: String,
    pub streams: Vec<Stream>,
}

static DIRTY: AtomicBool = AtomicBool::new(false);

impl Service for Mixer {
    fn new() -> Self {
        Self::load()
    }

    fn listen() {
        // события приходят пачками, поэтому перечитываем не чаще раза в 120 мс
        thread::spawn(|| {
            loop {
                thread::sleep(Duration::from_millis(120));

                if DIRTY.swap(false, Ordering::Relaxed) {
                    let fresh = Mixer::load();

                    *Mixer::write() = fresh;
                }
            }
        });

        for line in amane::lines("pactl subscribe") {
            if line.contains("sink") || line.contains("server") {
                DIRTY.store(true, Ordering::Relaxed);
            }
        }
    }
}

impl Mixer {
    fn load() -> Self {
        let sinks = parse_sinks(&amane::output("LC_ALL=C pactl list sinks"));
        let default = amane::output("pactl get-default-sink");
        let streams = parse_streams(&amane::output("LC_ALL=C pactl list sink-inputs"));

        Self {
            sinks,
            default,
            streams,
        }
    }

    // текущее устройство вывода
    pub fn current(&self) -> Option<&Sink> {
        self.sinks.iter().find(|sink| sink.name == self.default)
    }

    pub fn bluetooth(&self) -> bool {
        self.default.contains("bluez")
    }

    pub fn headphones(&self) -> bool {
        let description = self
            .current()
            .map(|sink| sink.description.to_lowercase())
            .unwrap_or_default();

        description.contains("headphone")
            || description.contains("headset")
            || description.contains("наушник")
    }

    pub fn select(name: &str) {
        let quoted = quote(name);

        let ids: Vec<u32> = Self::read()
            .streams
            .iter()
            .map(|stream| stream.id)
            .collect();

        let mut command = format!("pactl set-default-sink {quoted}");

        // уже играющие приложения переезжают на выбранное устройство
        for id in ids {
            command.push_str(&format!("; pactl move-sink-input {id} {quoted}"));
        }

        amane::spawn(&command);

        DIRTY.store(true, Ordering::Relaxed);
    }

    pub fn set_stream_volume(id: u32, percent: u8) {
        amane::spawn(&format!("pactl set-sink-input-volume {id} {percent}%"));
    }

    pub fn toggle_stream_mute(id: u32) {
        amane::spawn(&format!("pactl set-sink-input-mute {id} toggle"));
    }
}

fn quote(text: &str) -> String {
    format!("'{}'", text.replace('\'', "'\\''"))
}

fn parse_sinks(text: &str) -> Vec<Sink> {
    let mut sinks = Vec::new();
    let mut current: Option<Sink> = None;

    for line in text.lines() {
        if line.starts_with("Sink #") {
            if let Some(sink) = current.take() {
                sinks.push(sink);
            }

            current = Some(Sink {
                name: String::new(),
                description: String::new(),
            });

            continue;
        }

        let trimmed = line.trim();

        if let Some(sink) = current.as_mut() {
            if let Some(value) = trimmed.strip_prefix("Name: ") {
                if sink.name.is_empty() {
                    sink.name = value.to_string();
                }
            } else if let Some(value) = trimmed.strip_prefix("Description: ") {
                if sink.description.is_empty() {
                    sink.description = value.to_string();
                }
            }
        }
    }

    if let Some(sink) = current {
        sinks.push(sink);
    }

    sinks
}

fn parse_streams(text: &str) -> Vec<Stream> {
    let mut streams = Vec::new();
    let mut current: Option<Stream> = None;
    let mut media = String::new();

    let finish = |stream: Option<Stream>, media: &str, streams: &mut Vec<Stream>| {
        if let Some(mut stream) = stream {
            if stream.name.is_empty() {
                stream.name = if media.is_empty() {
                    String::from("Stream")
                } else {
                    media.to_string()
                };
            }

            streams.push(stream);
        }
    };

    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("Sink Input #") {
            finish(current.take(), &media, &mut streams);

            media.clear();

            current = Some(Stream {
                id: rest.trim().parse().unwrap_or(0),
                name: String::new(),
                volume: 0,
                muted: false,
            });

            continue;
        }

        let trimmed = line.trim();

        let Some(stream) = current.as_mut() else {
            continue;
        };

        if let Some(value) = trimmed.strip_prefix("Volume: ") {
            stream.volume = percent_in(value);
        } else if let Some(value) = trimmed.strip_prefix("Mute: ") {
            stream.muted = value.trim() == "yes";
        } else if let Some(value) = trimmed.strip_prefix("application.name = ") {
            stream.name = value.trim_matches('"').to_string();
        } else if let Some(value) = trimmed.strip_prefix("media.name = ") {
            media = value.trim_matches('"').to_string();
        }
    }

    finish(current, &media, &mut streams);

    streams
}

// "front-left: 42598 /  65% / -11.22 dB" -> 65
fn percent_in(text: &str) -> u8 {
    let Some(at) = text.find('%') else {
        return 0;
    };

    let before = &text[..at];
    let start = before.rfind(|c: char| !c.is_ascii_digit()).map(|i| i + 1).unwrap_or(0);

    before[start..].parse::<u32>().unwrap_or(0).min(150) as u8
}

// ---------------------------------------------------------------- батарея

#[derive(Clone, Default, PartialEq)]
pub struct BatteryData {
    pub present: bool,
    pub status: String,
    pub capacity: u8,
    // Вт·ч
    pub energy_now: f64,
    pub energy_full: f64,
    pub energy_design: f64,
    // Вт
    pub power: f64,
    pub cycles: Option<u32>,
    pub technology: String,
    pub model: String,
    pub profile: String,
}

pub struct BatteryInfo {
    pub data: BatteryData,
    dir: Option<PathBuf>,
    ticks: u32,
}

impl Service for BatteryInfo {
    fn new() -> Self {
        let dir = find_battery();

        let mut info = Self {
            data: BatteryData::default(),
            dir,
            ticks: 0,
        };

        info.data = info.snapshot();
        info.data.profile = amane::output("powerprofilesctl get");

        info
    }

    fn interval() -> Duration {
        Duration::from_secs(5)
    }

    fn update(&mut self) -> bool {
        self.ticks += 1;

        let mut fresh = self.snapshot();

        // профиль питания спрашиваем реже остального
        fresh.profile = if self.ticks % 6 == 0 {
            amane::output("powerprofilesctl get")
        } else {
            self.data.profile.clone()
        };

        let changed = fresh != self.data;

        self.data = fresh;

        changed
    }
}

impl BatteryInfo {
    fn snapshot(&self) -> BatteryData {
        let Some(dir) = &self.dir else {
            return BatteryData::default();
        };

        let text = |name: &str| {
            fs::read_to_string(dir.join(name))
                .unwrap_or_default()
                .trim()
                .to_string()
        };
        let number = |name: &str| {
            fs::read_to_string(dir.join(name))
                .ok()
                .and_then(|v| v.trim().parse::<f64>().ok())
        };

        let voltage = number("voltage_now").unwrap_or(0.0);

        // у части батарей вместо энергии (мкВт·ч) есть заряд (мкА·ч)
        let energy = |name: &str, charge: &str| -> f64 {
            match number(name) {
                Some(value) => value / 1e6,
                None => number(charge).map_or(0.0, |value| value * voltage / 1e12),
            }
        };

        let power = match number("power_now") {
            Some(value) => value / 1e6,
            None => number("current_now").map_or(0.0, |value| value * voltage / 1e12),
        };

        BatteryData {
            present: true,
            status: text("status"),
            capacity: number("capacity").unwrap_or(0.0) as u8,
            energy_now: energy("energy_now", "charge_now"),
            energy_full: energy("energy_full", "charge_full"),
            energy_design: energy("energy_full_design", "charge_full_design"),
            power,
            cycles: number("cycle_count")
                .map(|value| value as u32)
                .filter(|value| *value > 0),
            technology: text("technology"),
            model: text("model_name"),
            profile: String::new(),
        }
    }
}

fn find_battery() -> Option<PathBuf> {
    fs::read_dir("/sys/class/power_supply")
        .ok()?
        .flatten()
        .map(|entry| entry.path())
        .find(|path| {
            fs::read_to_string(path.join("type")).is_ok_and(|kind| kind.trim() == "Battery")
        })
}

// ---------------------------------------------------------------- история уведомлений

#[derive(Clone)]
pub struct NotificationHistoryEntry {
    pub notification: Notification,
    pub timestamp: SystemTime,
}

const NOTIF_HISTORY_MAX: usize = 100;

pub struct NotificationHistory {
    entries: Vec<NotificationHistoryEntry>,
    pub clearing: bool,
    pub clear_start: Instant,
}

impl Service for NotificationHistory {
    fn new() -> Self {
        Self {
            entries: Vec::new(),
            clearing: false,
            clear_start: Instant::now(),
        }
    }

    fn interval() -> Duration {
        Duration::from_millis(50)
    }

    fn update(&mut self) -> bool {
        if self.clearing {
            // после завершения каскадной анимации очищаем список
            // 400ms анимация + N*100ms задержка
            let total = 400 + self.entries.len() as u128 * 100;
            if Instant::now().duration_since(self.clear_start) >= Duration::from_millis(total as u64) {
                self.entries.clear();
                self.clearing = false;
                return true;
            }
        }
        false
    }
}

impl NotificationHistory {
    pub fn push(n: &Notification) {
        let mut hist = Self::write();
        if hist.entries.len() >= NOTIF_HISTORY_MAX {
            hist.entries.remove(0);
        }
        hist.entries.push(NotificationHistoryEntry {
            notification: n.clone(),
            timestamp: SystemTime::now(),
        });
    }

    pub fn list(&self) -> &[NotificationHistoryEntry] {
        &self.entries
    }

    pub fn clear() {
        let mut hist = Self::write();
        if hist.entries.is_empty() {
            return;
        }
        hist.clearing = true;
        hist.clear_start = Instant::now();
    }

    /// сдвиг карточки (чистая функция, не дергает read)
    pub fn clear_translate(index_from_end: usize, clearing: bool, elapsed_ms: u128) -> f32 {
        if !clearing {
            return 0.0;
        }
        let delay = index_from_end as u128 * 100;
        let anim_time = elapsed_ms.saturating_sub(delay);
        if anim_time == 0 {
            return 0.0;
        }
        let duration = 400;
        let t = (anim_time as f32 / duration as f32).min(1.0);
        let eased = if t < 0.5 {
            2.0 * t * t
        } else {
            1.0 - (-2.0 * t + 2.0).powi(2) / 2.0
        };
        340.0 * eased
    }

}

// ---------------------------------------------------------------- буфер обмена

pub struct ClipboardHistory {
    store: Store,
}

impl Service for ClipboardHistory {
    fn new() -> Self {
        Self { store: Store::new() }
    }

    fn listen() {
        // wl-paste --watch печатает строку при каждом изменении буфера;
        // если он завершился, поднимаем его снова
        loop {
            if let Ok(mut child) = Command::new("wl-paste")
                .args(["--type", "text", "--watch", "echo"])
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .spawn()
            {
                if let Some(stdout) = child.stdout.take() {
                    for _ in BufReader::new(stdout).lines().map_while(Result::ok) {
                        capture();
                    }
                }

                let _ = child.wait();
            }

            thread::sleep(Duration::from_secs(2));
        }
    }
}

// читает текущее содержимое буфера и кладёт в историю
fn capture() {
    // менеджеры паролей помечают такие записи, их в историю не берём
    let secret = run("wl-paste", &["--list-types"])
        .is_some_and(|types| types.contains("x-kde-passwordManagerHint"));

    if secret {
        return;
    }

    // картинки и прочее не текст: команда завершится ошибкой, запись пропускается
    if let Some(text) = run("wl-paste", &["--no-newline", "--type", "text"]) {
        ClipboardHistory::write().store.push(&text);
    }
}

fn run(program: &str, args: &[&str]) -> Option<String> {
    let output = Command::new(program).args(args).output().ok()?;

    if !output.status.success() {
        return None;
    }

    String::from_utf8(output.stdout).ok()
}

impl ClipboardHistory {
    // записи, подходящие под запрос, новые первыми
    pub fn matches(&self, query: &str) -> Vec<&str> {
        self.store.filter(query)
    }

    // строка списка под номером index (среди найденных по текущему запросу)
    pub fn pick(index: usize) {
        let query = Ui::read().clip_query.clone();

        let text = Self::read().matches(&query).get(index).map(|text| text.to_string());

        if let Some(text) = text {
            copy(text.clone());

            // выбранное поднимается на первое место
            Self::write().store.push(&text);

            Ui::close_all();
        }
    }

    // Enter в поле поиска
    pub fn pick_selected() {
        let selected = Ui::read().clip_selected;

        Self::pick(selected);
    }
}

// wl-copy читает текст из stdin, а не из аргументов: так он не виден в `ps`
fn copy(text: String) {
    thread::spawn(move || {
        let child = Command::new("wl-copy").stdin(Stdio::piped()).spawn();

        if let Ok(mut child) = child {
            if let Some(mut stdin) = child.stdin.take() {
                let _ = stdin.write_all(text.as_bytes());
            }

            let _ = child.wait();
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_seconds_rounds_up() {
        assert_eq!(display_seconds(Duration::from_secs(1500)), 1500);
        assert_eq!(display_seconds(Duration::from_millis(1_499_001)), 1500);
        assert_eq!(display_seconds(Duration::from_secs(1499)), 1499);
        assert_eq!(display_seconds(Duration::ZERO), 0);
        assert_eq!(display_seconds(Duration::from_millis(1)), 1);
    }

    #[test]
    fn set_minutes_clamps_and_resets() {
        Timer::set_minutes(0);
        assert_eq!(Timer::read().total(), Duration::from_secs(60));

        Timer::set_minutes(100);
        assert_eq!(Timer::read().total(), Duration::from_secs(99 * 60));

        Timer::set_minutes(45);
        assert!(!Timer::read().running());
        assert_eq!(Timer::read().remaining(), Duration::from_secs(45 * 60));
        assert_eq!(Timer::read().total(), Duration::from_secs(45 * 60));

        Timer::reset();
        assert_eq!(Timer::read().remaining(), Duration::from_secs(45 * 60));
    }

    fn temp_with(history: Vec<i32>) -> Temp {
        Temp {
            celsius: history.last().copied().unwrap_or(0),
            path: None,
            history,
        }
    }

    #[test]
    fn temp_history_keeps_last_60() {
        let mut temp = temp_with(vec![0]);

        for value in 1..=61 {
            temp.record(value);
        }

        assert_eq!(temp.history().len(), TEMP_HISTORY_MAX);
        // первый замер (0) и второй (1) вытеснены
        assert_eq!(temp.history()[0], 2);
        assert_eq!(temp.history()[TEMP_HISTORY_MAX - 1], 61);
    }

    #[test]
    fn temp_min_max_from_history() {
        let temp = temp_with(vec![40, 55, 47]);

        assert_eq!(temp.min(), 40);
        assert_eq!(temp.max(), 55);
    }

    #[test]
    fn temp_min_max_fall_back_to_celsius() {
        let temp = Temp {
            celsius: 42,
            path: None,
            history: Vec::new(),
        };

        assert_eq!(temp.min(), 42);
        assert_eq!(temp.max(), 42);
    }
}
