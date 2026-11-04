mod approach;
mod benchmark;
mod cli;
mod club;
mod collision;
mod colors;
mod eugenics;
mod fighter;
mod gameloop;
mod headless;
mod math;
mod render;
#[allow(dead_code)]
mod round;
mod train;

use crate::cli::{Cli, FightArgs, Mode, SavePerceptronArgs};

fn main() {
    let cli = Cli::parse_args();

    match cli.mode {
        // Бой: окно с поединком либо раунд без окна
        Mode::Fight(args) => run_fight(&args),
        // Обучение клуба бойцов
        Mode::TrainStrategist(args) => {
            if let Err(err) = train::run(&args) {
                eprintln!("Ошибка обучения: {err}");
                std::process::exit(1);
            }
        }
        // Обучение перцептронов: генетический алгоритм
        Mode::TrainEugenics(args) => {
            if let Err(err) = eugenics::run(&args) {
                eprintln!("Ошибка обучения: {err}");
                std::process::exit(1);
            }
        }
        // Сравнение двух поколений бойцов
        Mode::Benchmark(args) => {
            if let Err(err) = benchmark::run(&args) {
                eprintln!("Ошибка сравнения: {err}");
                std::process::exit(1);
            }
        }
        // Сохранение мозга на нейросети со случайными весами
        Mode::SavePerceptron(args) => {
            if let Err(err) = save_perceptron(&args) {
                eprintln!("Ошибка: {err}");
                std::process::exit(1);
            }
        }
    }
}

/// Сохраняет мозг на нейросети со случайными весами.
///
/// Нужен, чтобы заполнить файл обученного мозга до того, как появится
/// обучение: иначе готовой сети взять неоткуда.
fn save_perceptron(args: &SavePerceptronArgs) -> std::io::Result<()> {
    let seed = args.seed.unwrap_or_else(|| fastrand::u64(..));

    let brain = fighter::brain::perceptron::Perceptron::random_with_hidden(args.hidden, seed);
    brain.save_to_file(&args.out, &args.name)?;

    println!(
        "Мозг сохранён: \"{}\" (зерно {seed}, скрытый слой {})",
        args.out.display(),
        args.hidden
    );

    Ok(())
}

/// Бой: поединок двух бойцов. Без окна симуляция крутится, отрисовка не выполняется.
fn run_fight(args: &FightArgs) {
    // Мозги бойцов: без окна игроком управлять нечем, поэтому там
    // такой мозг заменяется заглушкой
    let red = match args.red.build(args.no_gui) {
        Ok(brain) => brain,
        Err(err) => {
            eprintln!("Ошибка: {err}");
            std::process::exit(1);
        }
    };
    let blue = match args.blue.build(args.no_gui) {
        Ok(brain) => brain,
        Err(err) => {
            eprintln!("Ошибка: {err}");
            std::process::exit(1);
        }
    };

    if args.no_gui {
        headless::run(red, blue);
        return;
    }

    gameloop::run(red, blue);
}
