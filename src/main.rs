mod cli;
mod loader;

use crate::cli::{Cli, FightArgs, Mode, SavePerceptronArgs};

fn main() {
    let cli = Cli::parse_args();

    match cli.mode {
        // Бой: окно с поединком либо раунд без окна
        Mode::Fight(args) => run_fight(&args),
        // Обучение клуба бойцов
        Mode::TrainStrategist(args) => {
            let config = fencing_strategist::train::TrainConfig {
                club: args.club,
                fighters: args.fighters,
                epochs: args.epochs,
                sparring: args.sparring,
                seed: args.seed,
                out: args.out,
            };

            if let Err(err) = fencing_strategist::train::run(&config) {
                eprintln!("Ошибка обучения: {err}");
                std::process::exit(1);
            }
        }
        // Обучение перцептронов: генетический алгоритм
        Mode::TrainEugenics(args) => {
            let config = fencing_eugenics::TrainConfig {
                club: args.club,
                elite: args.elite,
                offspring: args.offspring,
                generations: args.generations,
                repeat: args.repeat,
                hidden: args.hidden,
                mutation_rate: args.mutation_rate,
                mutation_scale: args.mutation_scale,
                crossover_rate: args.crossover_rate,
                tournament: args.tournament,
                scoring: args.scoring,
                benchmark_repeat: args.benchmark_repeat,
                seed: args.seed,
                out: args.out,
            };

            if let Err(err) = fencing_eugenics::run(&config) {
                eprintln!("Ошибка обучения: {err}");
                std::process::exit(1);
            }
        }
        // Сравнение двух поколений бойцов
        Mode::Benchmark(args) => {
            if let Err(err) =
                fencing_eugenics::benchmark::run(&args.first, &args.second, args.repeat, args.scoring)
            {
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

    let brain = fencing_perceptron::Perceptron::random_with_hidden(args.hidden, seed);
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
        fencing_engine::headless::run(red, blue);
        return;
    }

    fencing_engine::gameloop::run(red, blue);
}
