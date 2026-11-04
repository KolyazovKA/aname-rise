mod audio;
mod bar;
mod battery;
mod bluetooth;
mod calendar;
mod clip_store;
mod clipboard;
mod ipc;
mod notifications;
mod popup;
mod state;
mod theme;
mod ui;
mod wifi;

use amane::{App, Audio, Battery, Bluetooth, Network, Notifications, Service};

fn main() {
    // `amane-shell toggle clipboard`: передаём команду работающему бару и выходим
    let args: Vec<String> = std::env::args().skip(1).collect();

    if !args.is_empty() {
        std::process::exit(if ipc::send(&args.join(" ")) { 0 } else { 1 });
    }

    // сервисы стартуют заранее, чтобы окна открывались без задержки
    drop(state::Mixer::read());
    drop(state::Clock::read());
    drop(Audio::read());
    drop(Network::read());
    drop(Bluetooth::read());
    drop(Battery::read());
    drop(Notifications::read());
    drop(state::NotificationHistory::read());
    drop(notifications::ToastState::read());
    drop(state::ClipboardHistory::read());

    ipc::serve();

    App::new()
        .font(theme::FONT)
        .window_per_monitor(bar::view)
        // тосты поверх dismiss-слоя, чтобы клики по ним не закрывали popup
        .window_per_monitor(notifications::toast_view)
        // слой для щелчка мимо окон создаётся до самих окон, чтобы лежать под ними
        .window_per_monitor(popup::dismiss)
        .window_per_monitor(popup::view)
        .run();
}
