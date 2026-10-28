mod audio;
mod bar;
mod battery;
mod bluetooth;
mod calendar;
mod popup;
mod state;
mod theme;
mod ui;
mod wifi;

use amane::{App, Audio, Battery, Bluetooth, Network, Service};

fn main() {
    // сервисы стартуют заранее, чтобы окна открывались без задержки
    drop(state::Mixer::read());
    drop(state::Clock::read());
    drop(Audio::read());
    drop(Network::read());
    drop(Bluetooth::read());
    drop(Battery::read());

    App::new()
        .font(theme::FONT)
        .window_per_monitor(bar::view)
        // слой для щелчка мимо окон создаётся до самих окон, чтобы лежать под ними
        .window_per_monitor(popup::dismiss)
        .window_per_monitor(popup::view)
        .run();
}
