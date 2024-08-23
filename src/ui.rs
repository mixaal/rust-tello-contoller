use std::{
    sync::{mpsc::Receiver, RwLock},
    thread,
    time::Instant,
};

use rust_gamepad::gamepad::{self, Gamepad, GamepadState};
use rust_sdl_ui::{
    color::{self, RgbColor},
    desktop::{self, CommonWidgetProps},
    sdl,
};
use rust_tello::{TelloController, UpdateData};
use sdl2::{event::Event, keyboard::Keycode};

use crate::utils;

#[derive(Debug)]
struct DroneHandling {
    take_picture: bool,
    toggle_video: bool,
    take_off: bool,
    hover: bool,
    sensitivity: f32,
    vert_thrust: f32,
    slide_right: f32,
    forward: f32,
    turn_clockwise: f32,
    img_carousel_left: bool,
    img_carousel_right: bool,
    img_carousel_toggle_zoom: bool,
}

impl DroneHandling {
    pub fn zero_state(&mut self) {
        self.take_off = false;
        self.hover = false;
        self.take_picture = false;
        self.toggle_video = false;
        self.img_carousel_left = false;
        self.img_carousel_right = false;
        self.img_carousel_toggle_zoom = false;
        self.forward = Default::default();
        self.slide_right = Default::default();
        self.turn_clockwise = Default::default();
        self.vert_thrust = Default::default();
    }
}

impl Default for DroneHandling {
    fn default() -> Self {
        Self {
            take_off: false,
            hover: false,
            take_picture: false,
            toggle_video: false,
            img_carousel_left: false,
            img_carousel_right: false,
            img_carousel_toggle_zoom: false,
            sensitivity: 0.2,
            vert_thrust: Default::default(),
            slide_right: Default::default(),
            forward: Default::default(),
            turn_clockwise: Default::default(),
        }
    }
}

pub struct UI {
    width: u32,
    height: u32,
    fps: u32,
    drone: DroneHandling,
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
            drone: DroneHandling::default(),
        }
    }

    pub fn mainloop(
        &mut self,
        mut tello: TelloController,
        ctrl_rx: Receiver<UpdateData>,
        video_rx: Receiver<Vec<u8>>,
    ) {
        let mut playing = true;
        thread::spawn(move || update_data(ctrl_rx));

        let js = Gamepad::new("/dev/input/js0", gamepad::XBOX_MAPPING.clone());
        js.background_handler();

        let (mut win, mut canvas) = desktop::Window::new(self.width, self.height, self.fps);

        let _video = desktop::VideoWidget::new(
            CommonWidgetProps::new(&canvas)
                .place(0.5, 0.38)
                .size(0.4, 0.5),
            &mut canvas,
            960,
            720,
            2,
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
                .place(0.1, 0.5)
                .size(0.02, 0.12),
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

        let _flight_log = desktop::FlightLogWidget::new(
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
        vx.write().unwrap().set_scale(0.001);
        vy.write().unwrap().set_scale(0.001);
        vz.write().unwrap().set_scale(0.001);
        let bg_texture = sdl::sdl_load_textures(&canvas, vec!["images/bg01.png".to_owned()]);

        //sensitivity.write().unwrap().inc();
        let mut last_state = GamepadState::initial();
        while playing {
            // main loop
            'running: loop {
                let start = Instant::now();

                // handle joystick
                let st = js.state();
                self.joystick_handler(&st, &last_state);
                // handle keyboard events
                if self.keyboard_handler(&mut win.event_pump) {
                    playing = false;
                    break 'running;
                }
                tracing::debug!("drone-movement: {:?}", self.drone);
                // clear before drawing
                sdl::sdl_clear(&mut canvas, 10, 20, 30);
                let w = self.width as i32;
                let h = self.height as i32;
                sdl::sdl_scale_tex(&mut canvas, &bg_texture[0], w / 2, h / 2, w, h);

                if self.drone.take_off {
                    let flying = tello.flying();
                    if !flying {
                        tracing::info!("takeoff");
                        tello.takeoff();
                    } else {
                        tracing::info!("land");
                        tello.land();
                    }
                }
                if self.drone.hover {
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
                    light_signal
                        .write()
                        .unwrap()
                        .timestamp(light.light_strength_updated);
                }
                if let Some(ref log_record) = g_data.log {
                    if let Some(imu) = &log_record.imu {
                        horizon.write().unwrap().set(
                            -imu.pitch as f32,
                            -imu.roll as f32,
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

                if self.drone.take_picture {
                    tracing::info!("take picture");
                    tello.take_picture();
                }

                if self.drone.toggle_video {
                    tracing::info!("toggle video");
                    tello.toggle_video();
                }

                sensitivity.write().unwrap().set(self.drone.sensitivity);

                // control tello
                tello.forward(self.drone.forward);
                tello.right(self.drone.slide_right);
                tello.up(-self.drone.vert_thrust);
                tello.turn_clockwise(self.drone.turn_clockwise);

                if self.drone.img_carousel_toggle_zoom {
                    image_carousel.write().unwrap().toggle_show();
                }
                if self.drone.img_carousel_left {
                    image_carousel.write().unwrap().turn_left();
                }
                if self.drone.img_carousel_right {
                    image_carousel.write().unwrap().turn_right();
                }

                vert_thrust.write().unwrap().set(self.drone.vert_thrust);

                let ls = (self.drone.slide_right, -self.drone.forward);
                let rs = (self.drone.turn_clockwise, 0.0);
                left_stick.write().unwrap().set_stick(ls);
                right_stick.write().unwrap().set_stick(rs);

                win.draw(&mut canvas);
                canvas.present();
                sdl::sdl_maintain_fps(start, self.fps);
                last_state = st;
                self.drone.zero_state();
            }
        }
        tracing::info!("exiting mainloop");
    }

    fn joystick_handler(&mut self, st: &GamepadState, last_state: &GamepadState) {
        if st.button_clicked(gamepad::Buttons::START, &last_state) {
            self.drone.take_off = true;
        }
        if st.button_clicked(gamepad::Buttons::SELECT, &last_state) {
            self.drone.hover = true;
        }
        if st.button_clicked(gamepad::Buttons::A, &last_state) {
            self.drone.take_picture = true;
        }

        if st.button_clicked(gamepad::Buttons::B, &last_state) {
            self.drone.toggle_video = true;
        }
        if st.button_clicked(gamepad::Buttons::RB, &last_state) {
            if self.drone.sensitivity + 0.2 <= 1.01 {
                self.drone.sensitivity += 0.2;
            }
        }
        if st.button_clicked(gamepad::Buttons::LB, &last_state) {
            if self.drone.sensitivity > 0.19 {
                self.drone.sensitivity -= 0.2;
            }
        }

        let horiz = st.horiz();
        if st.button_clicked(gamepad::Buttons::X, &last_state) {
            self.drone.img_carousel_toggle_zoom = true;
        }
        if horiz < 0.0 {
            self.drone.img_carousel_left = true;
        }
        if horiz > 0.0 {
            self.drone.img_carousel_right = true;
        }

        let ls = st.l_stick(self.drone.sensitivity);
        let rs = st.r_stick(self.drone.sensitivity);
        let lt = st.lt(self.drone.sensitivity);
        let rt = st.rt(self.drone.sensitivity);

        self.drone.forward = -ls.1;
        self.drone.slide_right = ls.0;
        self.drone.vert_thrust = lt - rt;
        self.drone.turn_clockwise = rs.0;
    }

    fn keyboard_handler(&mut self, event_pump: &mut sdl2::EventPump) -> bool {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. } => {
                    return true;
                }
                Event::KeyDown {
                    keycode: Some(sdl2::keyboard::Keycode::Escape),
                    ..
                } => {
                    return true;
                }

                Event::KeyDown {
                    keycode: Some(Keycode::NUM_1),
                    ..
                } => {
                    self.drone.sensitivity = 0.0;
                    return false;
                }

                Event::KeyDown {
                    keycode: Some(Keycode::NUM_2),
                    ..
                } => {
                    self.drone.sensitivity = 0.2;
                    return false;
                }

                Event::KeyDown {
                    keycode: Some(Keycode::NUM_3),
                    ..
                } => {
                    self.drone.sensitivity = 0.4;
                    return false;
                }

                Event::KeyDown {
                    keycode: Some(Keycode::NUM_4),
                    ..
                } => {
                    self.drone.sensitivity = 0.6;
                    return false;
                }

                Event::KeyDown {
                    keycode: Some(Keycode::NUM_5),
                    ..
                } => {
                    self.drone.sensitivity = 0.8;
                    return false;
                }

                Event::KeyDown {
                    keycode: Some(Keycode::NUM_6),
                    ..
                } => {
                    self.drone.sensitivity = 1.0;
                    return false;
                }

                Event::KeyDown {
                    keycode: Some(Keycode::Left),
                    ..
                } => {
                    self.drone.slide_right = -1.0;
                    return false;
                }
                Event::KeyDown {
                    keycode: Some(Keycode::Right),
                    ..
                } => {
                    self.drone.slide_right = 1.0;
                    return false;
                }

                Event::KeyDown {
                    keycode: Some(Keycode::Up),
                    ..
                } => {
                    self.drone.forward = 1.0;
                    return false;
                }
                Event::KeyDown {
                    keycode: Some(Keycode::Down),
                    ..
                } => {
                    self.drone.forward = -1.0;
                    return false;
                }

                Event::KeyDown {
                    keycode: Some(Keycode::A),
                    ..
                } => {
                    self.drone.vert_thrust = 1.0;
                    return false;
                }
                Event::KeyDown {
                    keycode: Some(Keycode::Q),
                    ..
                } => {
                    self.drone.vert_thrust = -1.0;
                    return false;
                }

                Event::KeyUp {
                    keycode: Some(Keycode::Left),
                    ..
                } => {
                    self.drone.slide_right = 0.0;
                    return false;
                }
                Event::KeyUp {
                    keycode: Some(Keycode::Right),
                    ..
                } => {
                    self.drone.slide_right = 0.0;
                    return false;
                }

                Event::KeyUp {
                    keycode: Some(Keycode::Up),
                    ..
                } => {
                    self.drone.forward = 0.0;
                    return false;
                }
                Event::KeyUp {
                    keycode: Some(Keycode::Down),
                    ..
                } => {
                    self.drone.forward = 0.0;
                    return false;
                }

                Event::KeyUp {
                    keycode: Some(Keycode::A),
                    ..
                } => {
                    self.drone.vert_thrust = 0.0;
                    return false;
                }
                Event::KeyUp {
                    keycode: Some(Keycode::Q),
                    ..
                } => {
                    self.drone.vert_thrust = 0.0;
                    return false;
                }
                Event::KeyUp {
                    keycode: Some(Keycode::V),
                    ..
                } => {
                    self.drone.toggle_video = true;
                    return false;
                }
                Event::KeyUp {
                    keycode: Some(Keycode::P),
                    ..
                } => {
                    self.drone.take_picture = true;
                    return false;
                }
                Event::KeyUp {
                    keycode: Some(Keycode::SPACE),
                    ..
                } => {
                    self.drone.take_off = true;
                    return false;
                }
                _ => {}
            }
        }
        false
    }
}
