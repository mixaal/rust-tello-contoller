use std::{
    sync::{mpsc::Receiver, Arc, RwLock},
    thread,
    time::{Duration, Instant},
};

use rust_gamepad::gamepad::{self, Gamepad, GamepadState};
use rust_sdl_ui::{
    color::{self, RgbColor},
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

fn update_data(ctrl_rx: Receiver<UpdateData>) {
    loop {
        if let Ok(update) = ctrl_rx.recv() {
            let mut g = UPDATE_DATA.write().unwrap();
            *g = update;
            drop(g);
        }
    }
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
        thread::spawn(move || update_data(ctrl_rx));

        let js = Gamepad::new("/dev/input/js0", gamepad::XBOX_MAPPING.clone());
        js.background_handler();

        let (mut win, mut canvas) = desktop::Window::new(self.width, self.height, self.fps);

        let video = desktop::VideoWidget::new(
            CommonWidgetProps::new(&canvas)
                .place(0.5, 0.38)
                .size(0.4, 0.5),
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
            CommonWidgetProps::new(&canvas).place(0.1, 0.2).rect(0.1),
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
            color::YELLOW.clone(),
        )
        .on_window(&mut win);

        let image_carousel = desktop::ImageCarouselWidget::new(
            CommonWidgetProps::new(&canvas)
                .place(0.5, 0.9)
                .size(0.8, 0.1),
            "./save_pics",
            10,
        )
        .on_window(&mut win);

        let drone_yaw = desktop::DroneYawWidget::new(
            CommonWidgetProps::new(&canvas).place(0.35, 0.7).rect(0.12),
        )
        .on_window(&mut win);

        let temperature = desktop::VertThrustWidget::new(
            CommonWidgetProps::new(&canvas).place(0.25, 0.2).rect(0.1),
        )
        .on_window(&mut win);

        let vx = desktop::VertThrustWidget::new(
            CommonWidgetProps::new(&canvas).place(0.4, 0.1).rect(0.05),
        )
        .on_window(&mut win);
        let vy = desktop::VertThrustWidget::new(
            CommonWidgetProps::new(&canvas).place(0.5, 0.1).rect(0.05),
        )
        .on_window(&mut win);
        let vz = desktop::VertThrustWidget::new(
            CommonWidgetProps::new(&canvas).place(0.6, 0.1).rect(0.05),
        )
        .on_window(&mut win);

        let flight_log = desktop::FlightLogWidget::new(
            CommonWidgetProps::new(&canvas).place(0.65, 0.7).rect(0.12),
        )
        .on_window(&mut win);

        let mut t = temperature.write().unwrap();
        t.set_color1(RgbColor::new(0.0, 0.3, 1.0, 1.0));
        t.set_color2(RgbColor::new(1.0, 0.0, 0.0, 1.0));
        t.set(0.0);
        t.set_color_scale_factor(0.65);
        t.set_scale(0.01);
        drop(t);
        vx.write().unwrap().set_scale(0.05);
        vy.write().unwrap().set_scale(0.05);
        vz.write().unwrap().set_scale(0.05);
        let bg_texture = sdl::sdl_load_textures(&canvas, vec!["images/bg01.png".to_owned()]);

        sensitivity.write().unwrap().inc();
        let mut last_state = GamepadState::initial();
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
                let w = self.width as i32;
                let h = self.height as i32;
                sdl::sdl_scale_tex(&mut canvas, &bg_texture[0], w / 2, h / 2, w, h);

                // finally draw the game and maintain fps
                let st = js.state();

                if st.button_clicked(gamepad::Buttons::START, &last_state) {
                    let flying = tello.flying();
                    if !flying {
                        tracing::info!("takeoff");
                        tello.takeoff();
                    } else {
                        tracing::info!("land");
                        tello.land();
                    }
                }
                if st.button_clicked(gamepad::Buttons::SELECT, &last_state) {
                    tracing::info!("hover");
                    tello.hover();
                }

                let g_data = UPDATE_DATA.read().unwrap();
                if let Some(ref wifi) = g_data.wifi {
                    wifi_strength
                        .write()
                        .unwrap()
                        .set(wifi.wifi_strength as f32 / 100.0);
                }
                if let Some(ref flight) = g_data.flight {
                    battery
                        .write()
                        .unwrap()
                        .set(flight.battery_percentage as f32 / 100.0);
                }
                if let Some(ref light) = g_data.light {
                    light_signal.write().unwrap().now();
                }
                if let Some(ref log_record) = g_data.log {
                    if let Some(imu) = &log_record.imu {
                        horizon.write().unwrap().set(
                            imu.pitch as f32,
                            imu.roll as f32,
                            imu.yaw as f32,
                        );
                        drone_yaw.write().unwrap().set(imu.yaw as f32);
                        temperature.write().unwrap().set(-imu.temperature as f32);
                    }
                    if let Some(mvo) = &log_record.mvo {
                        if let Some(v_x) = mvo.vx {
                            vx.write().unwrap().set(-v_x as f32);
                        }
                        if let Some(v_y) = mvo.vy {
                            vy.write().unwrap().set(-v_y as f32);
                        }
                        if let Some(v_z) = mvo.vz {
                            vz.write().unwrap().set(-v_z as f32);
                        }
                    }
                }
                drop(g_data);

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
