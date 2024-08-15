use std::{
    sync::{mpsc::Receiver, Arc, RwLock},
    thread,
    time::{Duration, Instant},
};

use rust_gamepad::gamepad::{self, Gamepad, GamepadState};
use rust_sdl_ui::{
    desktop::{self, CommonWidgetProps, RawImage},
    sdl,
};
use rust_tello::{TelloController, UpdateData};

use crate::utils;

pub struct UI {
    width: u32,
    height: u32,
    fps: u32,
}

lazy_static! {
    static ref VIDEO_FRAME: RwLock<Vec<u8>> = RwLock::new(utils::make(960 * 720 * 3));
    static ref UPDATE_DATA: RwLock<UpdateData> = RwLock::new(UpdateData::default());
}

impl UI {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            fps: 60,
        }
    }

    pub fn mainloop(
        &self,
        mut tello: TelloController,
        ctrl_rx: Receiver<UpdateData>,
        video_rx: Receiver<Vec<u8>>,
    ) {
        let mut playing = true;

        let js = Gamepad::new("/dev/input/js0", gamepad::XBOX_MAPPING.clone());
        js.background_handler();

        let (mut win, mut canvas) = desktop::Window::new(self.width, self.height, self.fps);

        let video = desktop::VideoWidget::new(
            CommonWidgetProps::new(&canvas)
                .place(0.5, 0.3)
                .size(0.4, 0.3),
            &mut canvas,
            960,
            720,
            50,
        )
        .on_window(&mut win, video_rx);

        let sensitivity = desktop::HorizSliderWidget::new(
            desktop::CommonWidgetProps::new(&canvas)
                .place(0.2, 0.5)
                .size(0.15, 0.003),
            0.0,
            1.0,
            5.0,
        )
        .on_window(&mut win);

        let left_stick = desktop::GamepadStickWidget::new(
            desktop::CommonWidgetProps::new(&canvas)
                .place(0.2, 0.7)
                .rect(0.1),
        )
        .on_window(&mut win);

        let right_stick = desktop::GamepadStickWidget::new(
            desktop::CommonWidgetProps::new(&canvas)
                .place(0.8, 0.7)
                .rect(0.1),
        )
        .on_window(&mut win);

        let vert_thrust = desktop::VertThrustWidget::new(
            CommonWidgetProps::new(&canvas).place(0.2, 0.2).rect(0.1),
        )
        .on_window(&mut win);

        let battery = desktop::BatteryStatusWidget::new(
            CommonWidgetProps::new(&canvas)
                .place(0.1, 0.1)
                .size(0.01, 0.06),
        )
        .on_window(&mut win);

        let wifi_strength = desktop::WifiStrengthWidget::new(
            CommonWidgetProps::new(&canvas).place(0.8, 0.2).rect(0.1),
        )
        .on_window(&mut win);

        let light_signal = desktop::LightSignalWidget::new(
            CommonWidgetProps::new(&canvas).place(0.8, 0.45).rect(0.1),
        )
        .on_window(&mut win);

        let horizon = desktop::HorizonWidget::new(
            CommonWidgetProps::new(&canvas).place(0.5, 0.7).rect(0.12),
            40.0,
        )
        .on_window(&mut win);

        let image_carousel = desktop::ImageCarouselWidget::new(
            CommonWidgetProps::new(&canvas)
                .place(0.5, 0.9)
                .size(0.8, 0.1),
            "examples/widget-demo/images",
            10,
        )
        .on_window(&mut win);

        let drone_yaw = desktop::DroneYawWidget::new(
            CommonWidgetProps::new(&canvas).place(0.35, 0.7).rect(0.12),
        )
        .on_window(&mut win);

        battery.write().unwrap().set(0.09);
        wifi_strength.write().unwrap().set(0.4);

        sensitivity.write().unwrap().inc();
        let mut last_state = GamepadState::initial();
        let mut pitch = 0.0;
        let mut roll = 0.0;
        while playing {
            // reset game state

            // main loop
            let mut now = Instant::now();
            'running: loop {
                let start = Instant::now();
                // handle keyboard events
                if win.default_keyhandler() {
                    playing = false;
                    break 'running;
                }
                // clear before drawing
                sdl::sdl_clear(&mut canvas, 10, 20, 30);

                // finally draw the game and maintain fps
                let st = js.state();

                horizon.write().unwrap().set(pitch, roll, 120.0);

                let a_clicked = st.button_clicked(gamepad::Buttons::A, &last_state);
                if a_clicked {
                    tracing::info!("take picture");
                    tello.take_picture();
                }

                let b_clicked = st.button_clicked(gamepad::Buttons::B, &last_state);
                if b_clicked {
                    tracing::info!("toggle video");
                    tello.toggle_video();
                }

                let rb = st.button_clicked(gamepad::Buttons::RB, &last_state);
                let lb = st.button_clicked(gamepad::Buttons::LB, &last_state);
                if rb {
                    sensitivity.write().unwrap().inc();
                }
                if lb {
                    sensitivity.write().unwrap().dec();
                }
                let sensitivity_valye = sensitivity.read().unwrap().get();

                let ls = st.l_stick(sensitivity_valye);
                let rs = st.r_stick(sensitivity_valye);
                let lt = st.lt(sensitivity_valye);
                let rt = st.rt(sensitivity_valye);

                // control tello
                tello.forward(ls.0);
                tello.right(ls.1);
                let vert_speed = lt - rt;
                tello.up(vert_speed);
                tello.turn_clockwise(rs.0);

                let horiz = st.horiz();
                let x_clicked = st.button_clicked(gamepad::Buttons::X, &last_state);
                if x_clicked {
                    image_carousel.write().unwrap().toggle_show();
                }
                if horiz < 0.0 {
                    image_carousel.write().unwrap().turn_left();
                }
                if horiz > 0.0 {
                    image_carousel.write().unwrap().turn_right();
                }
                pitch += ls.1;
                roll += rs.0;

                vert_thrust.write().unwrap().set(vert_speed);

                left_stick.write().unwrap().set_stick(ls);
                right_stick.write().unwrap().set_stick(rs);

                win.draw(&mut canvas);
                // self.draw(&mut canvas, &mut texture);
                canvas.present();
                now = Instant::now();
                sdl::sdl_maintain_fps(start, self.fps);
                last_state = st;
            }
        }
        tracing::info!("exiting mainloop");
    }
}
