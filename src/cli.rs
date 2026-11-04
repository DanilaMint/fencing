use std::{ffi::OsString, path::PathBuf};

use clap::{Args, Parser, Subcommand};

use crate::{
    eugenics::scoring::Scoring,
    fighter::brain::loader::BrainKind,
};

/// Аргументы командной строки
#[derive(Parser, Debug)]
#[command(
    name = "fencing",
    version,
    about = "Симулятор фехтования",
    subcommand_required = true,
    arg_required_else_help = false
)]
pub struct Cli {
    /// Режим работы программы
    #[command(subcommand)]
    pub mode: Mode,
}

/// Режимы работы программы
#[derive(Subcommand, Debug)]
pub enum Mode {
    /// Бой: поединок двух бойцов в окне
    Fight(FightArgs),
    /// Обучение стратегов: эволюция клуба бойцов
    TrainStrategist(TrainStrategistArgs),
    /// Обучение перцептронов: генетический алгоритм
    TrainEugenics(TrainEugenicsArgs),
    /// Сравнение двух поколений бойцов: кто сильнее
    Benchmark(BenchmarkArgs),
    /// Сохранение мозга на нейросети со случайными весами
    SavePerceptron(SavePerceptronArgs),
}

/// Аргументы режима сравнения поколений
#[derive(Args, Debug)]
pub struct BenchmarkArgs {
    /// Каталог с мозгами первой группы: обычно epoch_NN
    #[arg(long = "first")]
    pub first: PathBuf,
    /// Каталог с мозгами второй группы: обычно epoch_MM
    #[arg(long = "second")]
    pub second: PathBuf,
    /// Число боёв на каждую пару
    #[arg(long = "repeat", default_value_t = 5)]
    pub repeat: u32,
    /// Расценка очков: classic — как в исходных правилах, epic — бой дороже измора
    #[arg(long = "scoring", default_value_t = Scoring::Classic, value_parser = parse_scoring)]
    pub scoring: Scoring,
}

/// Аргументы режима боя
#[derive(Args, Debug)]
pub struct FightArgs {
    /// Не открывать окно: игра работает без отображения
    #[arg(long = "no-gui", short = 'n')]
    pub no_gui: bool,

    /// Мозг красного бойца: player, doll, perceptron или путь к файлу обученного мозга
    #[arg(long = "red", value_name = "BRAIN", default_value = "doll", value_parser = parse_brain)]
    pub red: BrainKind,

    /// Мозг синего бойца: player, doll, perceptron или путь к файлу обученного мозга
    #[arg(long = "blue", value_name = "BRAIN", default_value = "doll", value_parser = parse_brain)]
    pub blue: BrainKind,
}

/// Аргументы режима сохранения мозга на нейросети
#[derive(Args, Debug)]
pub struct SavePerceptronArgs {
    /// Файл, в который сохраняется мозг
    #[arg(long = "out", value_name = "FILE")]
    pub out: PathBuf,

    /// Зерно генератора: одинаковое зерно — одинаковый мозг
    #[arg(long = "seed")]
    pub seed: Option<u64>,

    /// Ширина скрытого слоя сети
    #[arg(long = "hidden", default_value_t = crate::fighter::brain::perceptron::net::DEFAULT_HIDDEN)]
    pub hidden: usize,

    /// Имя бойца, записываемое в файл
    #[arg(long = "name", default_value = "perceptron")]
    pub name: String,
}

/// Аргументы режима обучения стратегов
#[derive(Args, Debug)]
pub struct TrainStrategistArgs {
    /// Имя клуба для обучения
    #[arg(long = "club", default_value = crate::train::DEFAULT_CLUB)]
    pub club: String,
    /// Число бойцов в клубе
    #[arg(long = "fighters", default_value_t = crate::train::DEFAULT_FIGHTERS)]
    pub fighters: usize,
    /// Число эпох обучения
    #[arg(long = "epochs", default_value_t = crate::train::DEFAULT_EPOCHS)]
    pub epochs: u32,
    /// Число спарингов на пару за эпоху
    #[arg(long = "sparring", default_value_t = crate::train::DEFAULT_SPARINGS)]
    pub sparring: u32,
    /// Базовое зерно обучения: одинаковое зерно — одинаковый прогон
    #[arg(long = "seed", default_value_t = crate::train::DEFAULT_SEED)]
    pub seed: u64,
    /// Каталог, куда складываются бэкапы и отчёт обучения
    #[arg(long = "out", default_value = crate::train::DEFAULT_CLUB)]
    pub out: String,
}

/// Аргументы режима генетического обучения перцептронов
#[derive(Args, Debug)]
pub struct TrainEugenicsArgs {
    /// Имя клуба для обучения
    #[arg(long = "club", default_value = crate::eugenics::DEFAULT_CLUB)]
    pub club: String,
    /// Число бойцов в элите: они переходят в следующее поколение
    #[arg(long = "elite", default_value_t = crate::eugenics::DEFAULT_ELITE)]
    pub elite: usize,
    /// Число наследников за поколение
    #[arg(long = "offspring", default_value_t = crate::eugenics::DEFAULT_OFFSPRING)]
    pub offspring: usize,
    /// Число поколений обучения
    #[arg(long = "generations", default_value_t = crate::eugenics::DEFAULT_GENERATIONS)]
    pub generations: u32,
    /// Число боёв на пару за поколение
    #[arg(long = "repeat", default_value_t = crate::eugenics::DEFAULT_REPEAT)]
    pub repeat: u32,
    /// Ширина скрытого слоя сети
    #[arg(long = "hidden", default_value_t = crate::fighter::brain::perceptron::net::DEFAULT_HIDDEN)]
    pub hidden: usize,
    /// Вероятность мутации одного гена
    #[arg(long = "mutation-rate", default_value_t = 0.05)]
    pub mutation_rate: f32,
    /// Величина мутации: доля от предела слоя
    #[arg(long = "mutation-scale", default_value_t = 0.25)]
    pub mutation_scale: f32,
    /// Доля генов, берущихся от первого родителя
    #[arg(long = "crossover-rate", default_value_t = 0.5)]
    pub crossover_rate: f32,
    /// Размер турнира при выборе родителей
    #[arg(long = "tournament", default_value_t = 3)]
    pub tournament: usize,
    /// Число боёв на пару при проверке силы против первого поколения
    #[arg(long = "benchmark-repeat", default_value_t = 3)]
    pub benchmark_repeat: u32,
    /// Расценка очков: classic — как в исходных правилах, epic — бой дороже измора
    #[arg(long = "scoring", default_value_t = Scoring::Classic, value_parser = parse_scoring)]
    pub scoring: Scoring,
    /// Базовое зерно обучения
    #[arg(long = "seed", default_value_t = 20240101)]
    pub seed: u64,
    /// Каталог, куда складываются бэкапы и отчёт обучения
    #[arg(long = "out", default_value = crate::eugenics::DEFAULT_CLUB)]
    pub out: String,
}

impl Cli {    /// Разбирает аргументы командной строки.
    ///
    /// Clap различает короткие (`-n`) и длинные (`--no-gui`) опции,
    /// поэтому аргументы вида `-слово` приводятся к виду `--слово`:
    /// одинаково работают и `-no-gui`, и `--no-gui`.
    ///
    /// При ошибке разбора печатается сообщение, программа завершается с кодом 2.
    pub fn parse_args() -> Self {
        let args: Vec<OsString> = std::env::args_os().map(normalize_arg).collect();

        Self::parse_from(args)
    }
}

/// Разбирает значение флага `--red` / `--blue`: имя встроенного мозга
/// или путь к файлу с обученным мозгом.
fn parse_brain(value: &str) -> Result<BrainKind, String> {
    value.parse()
}

/// Разбирает значение флага `--scoring`: classic или epic
fn parse_scoring(value: &str) -> Result<Scoring, String> {
    value.parse()
}

/// Одиночный дефис перед длинным именем заменяется двойным: `-no-gui` → `--no-gui`.
/// Короткие опции (`-n`) и числа со знаком минус остаются как есть.
fn normalize_arg(arg: OsString) -> OsString {
    let text = arg.to_string_lossy();

    let is_long = text.starts_with('-')
        && !text.starts_with("--")
        && text.len() > 2
        && !text[1..].starts_with(|c: char| c.is_ascii_digit());

    if !is_long {
        return arg;
    }

    OsString::from(format!("--{}", &text[1..]))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    fn normalized(args: &[&str]) -> Vec<String> {
        args.iter()
            .map(|arg| {
                normalize_arg(OsString::from(*arg))
                    .to_string_lossy()
                    .into_owned()
            })
            .collect()
    }

    #[test]
    fn single_dash_long_option_becomes_double_dash() {
        assert_eq!(normalized(&["-no-gui"]), ["--no-gui"]);
    }

    #[test]
    fn double_dash_option_is_unchanged() {
        assert_eq!(normalized(&["--no-gui"]), ["--no-gui"]);
    }

    #[test]
    fn short_option_is_unchanged() {
        assert_eq!(normalized(&["-n"]), ["-n"]);
    }

    #[test]
    fn negative_number_is_unchanged() {
        assert_eq!(normalized(&["-12"]), ["-12"]);
    }

    /// Режим без подкоманды не запускается
    #[test]
    fn mode_is_required() {
        assert!(Cli::try_parse_from(["fencing"]).is_err());
    }

    #[test]
    fn no_gui_flag_is_parsed() {
        let cli = Cli::try_parse_from(["fencing", "fight", "--no-gui"]).unwrap();

        let Mode::Fight(args) = cli.mode else {
            panic!("ожидался режим боя");
        };

        assert!(args.no_gui);
    }

    #[test]
    fn no_gui_flag_is_absent_by_default() {
        let cli = Cli::try_parse_from(["fencing", "fight"]).unwrap();

        let Mode::Fight(args) = cli.mode else {
            panic!("ожидался режим боя");
        };

        assert!(!args.no_gui);
    }

    #[test]
    fn brains_are_dummies_by_default() {
        let cli = Cli::try_parse_from(["fencing", "fight"]).unwrap();

        let Mode::Fight(args) = cli.mode else {
            panic!("ожидался режим боя");
        };

        assert_eq!(args.red, BrainKind::Doll);
        assert_eq!(args.blue, BrainKind::Doll);
    }

    #[test]
    fn red_brain_can_be_player() {
        let cli = Cli::try_parse_from(["fencing", "fight", "--red=player"]).unwrap();

        let Mode::Fight(args) = cli.mode else {
            panic!("ожидался режим боя");
        };

        assert_eq!(args.red, BrainKind::Player);
    }

    #[test]
    fn blue_brain_can_be_a_file() {
        let cli =
            Cli::try_parse_from(["fencing", "fight", "--blue=firsts/epoch_08/firsts_00.json"])
                .unwrap();

        let Mode::Fight(args) = cli.mode else {
            panic!("ожидался режим боя");
        };

        assert_eq!(
            args.blue,
            BrainKind::File(PathBuf::from("firsts/epoch_08/firsts_00.json"))
        );
    }

    #[test]
    fn brains_can_be_set_separately() {
        let cli = Cli::try_parse_from(["fencing", "fight", "--red=player", "--blue=doll"]).unwrap();

        let Mode::Fight(args) = cli.mode else {
            panic!("ожидался режим боя");
        };

        assert_eq!(args.red, BrainKind::Player);
        assert_eq!(args.blue, BrainKind::Doll);
    }

    /// Режим обучения стратегов принимает свои опции
    #[test]
    fn train_strategist_options_are_parsed() {
        let cli = Cli::try_parse_from([
            "fencing",
            "train-strategist",
            "--club=club_test",
            "--fighters=4",
            "--epochs=3",
            "--sparring=20",
            "--seed=7",
            "--out=club_test_out",
        ])
        .unwrap();

        let Mode::TrainStrategist(args) = cli.mode else {
            panic!("ожидался режим обучения стратегов");
        };

        assert_eq!(args.club, "club_test");
        assert_eq!(args.fighters, 4);
        assert_eq!(args.epochs, 3);
        assert_eq!(args.sparring, 20);
        assert_eq!(args.seed, 7);
        assert_eq!(args.out, "club_test_out");
    }

    /// Значения опций обучения по умолчанию
    #[test]
    fn train_strategist_defaults() {
        let cli = Cli::try_parse_from(["fencing", "train-strategist"]).unwrap();

        let Mode::TrainStrategist(args) = cli.mode else {
            panic!("ожидался режим обучения стратегов");
        };

        assert_eq!(args.club, crate::train::DEFAULT_CLUB);
        assert_eq!(args.fighters, crate::train::DEFAULT_FIGHTERS);
        assert_eq!(args.epochs, crate::train::DEFAULT_EPOCHS);
        assert_eq!(args.sparring, crate::train::DEFAULT_SPARINGS);
        assert_eq!(args.seed, crate::train::DEFAULT_SEED);
        assert_eq!(args.out, crate::train::DEFAULT_CLUB);
    }

    /// Режим генетического обучения принимает свои опции
    #[test]
    fn train_eugenics_options_are_parsed() {
        let cli = Cli::try_parse_from([
            "fencing",
            "train-eugenics",
            "--club=club_test",
            "--elite=4",
            "--offspring=12",
            "--generations=3",
            "--repeat=2",
            "--hidden=8",
            "--mutation-rate=0.2",
            "--mutation-scale=0.4",
            "--crossover-rate=0.6",
            "--tournament=5",
            "--seed=9",
            "--out=club_test_out",
        ])
        .unwrap();

        let Mode::TrainEugenics(args) = cli.mode else {
            panic!("ожидался режим генетического обучения");
        };

        assert_eq!(args.club, "club_test");
        assert_eq!(args.elite, 4);
        assert_eq!(args.offspring, 12);
        assert_eq!(args.generations, 3);
        assert_eq!(args.repeat, 2);
        assert_eq!(args.hidden, 8);
        assert_eq!(args.mutation_rate, 0.2);
        assert_eq!(args.mutation_scale, 0.4);
        assert_eq!(args.crossover_rate, 0.6);
        assert_eq!(args.tournament, 5);
        assert_eq!(args.seed, 9);
        assert_eq!(args.out, "club_test_out");
    }

    /// Значения опций генетического обучения по умолчанию
    #[test]
    fn train_eugenics_defaults() {
        let cli = Cli::try_parse_from(["fencing", "train-eugenics"]).unwrap();

        let Mode::TrainEugenics(args) = cli.mode else {
            panic!("ожидался режим генетического обучения");
        };

        assert_eq!(args.club, crate::eugenics::DEFAULT_CLUB);
        assert_eq!(args.elite, crate::eugenics::DEFAULT_ELITE);
        assert_eq!(args.offspring, crate::eugenics::DEFAULT_OFFSPRING);
        assert_eq!(args.generations, crate::eugenics::DEFAULT_GENERATIONS);
        assert_eq!(args.repeat, crate::eugenics::DEFAULT_REPEAT);
        assert_eq!(args.hidden, 16);
        assert_eq!(args.seed, 20240101);
        assert_eq!(args.out, crate::eugenics::DEFAULT_CLUB);
    }
}
