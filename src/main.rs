mod approach;
mod cli;
mod collision;
mod colors;
mod fighter;
mod headless;
mod math;
mod render;
#[allow(dead_code)]
mod round;
mod train;

use macroquad::{miniquad::conf::Platform, prelude::*};

use crate::{
    cli::Cli,
    fighter::brain::Brain,
    math::DT,
    render::{draw_round, draw_round_result},
    round::Round,
};

/// Заголовок окна
const WINDOW_TITLE: &str = "Fencing";
/// Ширина окна, пиксели
const WINDOW_WIDTH: i32 = 1600;
/// Высота окна, пиксели
const WINDOW_HEIGHT: i32 = 900;

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

fn main() {
    let cli = Cli::parse_args();

    // Обучение клуба бойцов вместо игры
    if cli.train {
        if let Err(err) = train::run(&cli) {
            eprintln!("Ошибка обучения: {err}");
            std::process::exit(1);
        }

        return;
    }

    // Мозги бойцов: без окна игроком управлять нечем, поэтому там
    // такой мозг заменяется заглушкой
    let red = match cli.red.build(cli.no_gui) {
        Ok(brain) => brain,
        Err(err) => {
            eprintln!("Ошибка: {err}");
            std::process::exit(1);
        }
    };
    let blue = match cli.blue.build(cli.no_gui) {
        Ok(brain) => brain,
        Err(err) => {
            eprintln!("Ошибка: {err}");
            std::process::exit(1);
        }
    };

    // Без окна: симуляция крутится, отрисовка не выполняется
    if cli.no_gui {
        headless::run(red, blue);
        return;
    }

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
