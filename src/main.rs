mod approach;
mod colors;
mod fighter;
mod math;
mod render;
mod round;

use macroquad::{miniquad::conf::Platform, prelude::*};

use crate::{
    fighter::{Fighter, brain::Doll},
    math::SPT,
    render::draw_fighter,
};

fn conf() -> Conf {
    Conf {
        window_title: String::from("Fight"),
        window_width: 1600,
        window_height: 900,
        window_resizable: true,
        fullscreen: false,
        high_dpi: false,
        icon: None,
        sample_count: 1,
        platform: Platform::default(),
    }
}

#[macroquad::main(conf)]
async fn main() {
    let test_brain = Box::new(Doll);

    let mut fighter = Fighter::new(test_brain);

    let mut time_delta = 0.0;

    loop {
        clear_background(BLACK);

        time_delta += get_frame_time();

        while time_delta >= SPT {
            time_delta -= SPT * 2.;
            // tick
            fighter.chok();
            fighter.rotate(0.1);
            fighter.tick();
        }

        draw_fps();

        draw_fighter(&fighter, RED);

        next_frame().await;
    }
}
