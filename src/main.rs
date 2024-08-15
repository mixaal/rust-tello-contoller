use std::{sync::mpsc, thread, time::Duration};

use rust_gamepad::gamepad::{self, Gamepad, GamepadState};
use rust_tello::TelloController;
use rust_tello_controller::{help::XBOX, ui::UI};

fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::TRACE)
        .init();

    tracing::info!("{}", XBOX);

    let mut tello = TelloController::new();

    let (update_tx, update_rx) = rust_tello::comm_channel();
    let (video_tx, video_rx) = mpsc::channel();
    let _h = tello.start_ctrl_receiver(update_tx);
    tello.start_video_receiver(video_tx);
    tello.start_video_contoller();

    tello.connect();
    loop {
        tracing::info!("waiting to connect to tello...");
        if tello.is_connected() {
            tracing::info!("connected to tello");
            break;
        }
        thread::sleep(Duration::from_secs(1));
    }
    tello.start_stick_update();
    tracing::info!("use gamepad to fly the drone");

    let ui = UI::new(3440, 1440);
    ui.mainloop(tello, update_rx, video_rx);
}
