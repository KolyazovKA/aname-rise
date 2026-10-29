// сервисы: всё, что бар и окна читают через `Service::read()`
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use amane::{Bluetooth, Network, Service, TextInput};

pub const PASSWORD_INPUT: &str = "wifi-password";

// ---------------------------------------------------------------- окна

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Popup {
    Calendar,
    Bluetooth,
    Wifi,
    Audio,
    Battery,
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

        TextInput::set_text(PASSWORD_INPUT, "");
    }

    pub fn set_hover(key: &str, inside: bool) {
        let mut ui = Self::write();

        if inside {
            ui.hovered = Some(key.to_string());
        } else if ui.hovered.as_deref() == Some(key) {
            ui.hovered = None;
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

const DAY: i64 = 86_400;

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
}

// "+0200" -> секунды
fn read_offset() -> i64 {
    let zone = amane::output("date +%z");

    let sign = if zone.starts_with('-') { -1 } else { 1 };

    let digits: Vec<i64> = zone
        .chars()
        .filter(char::is_ascii_digit)
        .filter_map(|c| c.to_digit(10))
        .map(i64::from)
        .collect();

    if digits.len() < 4 {
        return 0;
    }

    sign * ((digits[0] * 10 + digits[1]) * 3600 + (digits[2] * 10 + digits[3]) * 60)
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

pub struct Temp {
    pub celsius: i32,
    path: Option<PathBuf>,
}

impl Service for Temp {
    fn new() -> Self {
        let path = find_sensor();

        let celsius = path.as_ref().map_or(0, read_temp);

        Self { celsius, path }
    }

    fn interval() -> Duration {
        Duration::from_secs(5)
    }

    fn update(&mut self) -> bool {
        let Some(path) = &self.path else {
            return false;
        };

        let now = read_temp(path);
        let changed = now != self.celsius;

        self.celsius = now;

        changed
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
            "tun",
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

    let digits: String = text[..at]
        .chars()
        .rev()
        .take_while(char::is_ascii_digit)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();

    digits.parse::<u32>().unwrap_or(0).min(150) as u8
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
    let mut found: Vec<PathBuf> = fs::read_dir("/sys/class/power_supply")
        .ok()?
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            fs::read_to_string(path.join("type")).is_ok_and(|kind| kind.trim() == "Battery")
        })
        .collect();

    found.sort();

    found.into_iter().next()
}
