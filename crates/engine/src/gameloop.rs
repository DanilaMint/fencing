use macroquad::{miniquad::conf::Platform, prelude::*};

use fencing_core::{brain::Brain, math::DT, round::Round};

use crate::render::{draw_round, draw_round_result};

/// Заголовок окна
const WINDOW_TITLE: &str = "Fencing";
/// Ширина окна, пиксели
const WINDOW_WIDTH: i32 = 1600;
/// Высота окна, пиксели
const WINDOW_HEIGHT: i32 = 900;

/// Конфигурация окна игры
fn conf() -> Conf {
    Conf {
        window_title: String::from(WINDOW_TITLE),
        window_width: WINDOW_WIDTH,
        window_height: WINDOW_HEIGHT,
        window_resizable: true,
        fullscreen: false,
        high_dpi: false,
        icon: None,
        sample_count: 1,
        platform: Platform::default(),
    }
}

/// Игра в окне: создаёт окно и запускает игровой цикл.
///
/// Блокирует выполнение, пока окно открыто.
pub fn run(red: Box<dyn Brain>, blue: Box<dyn Brain>) {
    macroquad::Window::from_config(conf(), game_loop(red, blue));
}

/// Игровой цикл: шаги симуляции и отрисовка
async fn game_loop(red: Box<dyn Brain>, blue: Box<dyn Brain>) {
    let mut round = Round::new(red, blue);

    let mut time_delta = 0.0;

    loop {
        clear_background(BLACK);

        time_delta += get_frame_time();

        while time_delta >= DT {
            time_delta -= DT;
            // После окончания раунда tick() ничего не делает: бойцы замирают.
            // Аккумулятор времени при этом вычитывается, чтобы после рестарта
            // не было «перемотки» нескольких тиков за один кадр
            round.tick();
        }

        draw_round(&round);
        draw_round_result(&round);
        draw_fps();

        // Рестарт раунда: мозги бойцов переиспользуются
        if round.is_finished() && is_key_pressed(KeyCode::Space) {
            let (red_brain, blue_brain) = round.extract_brains();
            round = Round::new(red_brain, blue_brain);
        }

        next_frame().await;
    }
}
