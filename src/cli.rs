use std::ffi::OsString;

use clap::Parser;

use crate::fighter::brain::loader::BrainKind;

/// Аргументы командной строки
#[derive(Parser, Debug)]
#[command(name = "fencing", version, about = "Симулятор фехтования")]
pub struct Cli {
    /// Не открывать окно: игра работает без отображения
    #[arg(long = "no-gui", short = 'n')]
    pub no_gui: bool,

    /// Мозг красного бойца: player, doll или путь к файлу обученного мозга
    #[arg(long = "red", value_name = "BRAIN", default_value = "doll", value_parser = parse_brain)]
    pub red: BrainKind,

    /// Мозг синего бойца: player, doll или путь к файлу обученного мозга
    #[arg(long = "blue", value_name = "BRAIN", default_value = "doll", value_parser = parse_brain)]
    pub blue: BrainKind,

    /// Обучить клуб бойцов вместо игры
    #[arg(long = "train")]
    pub train: bool,
    /// Имя клуба для обучения
    #[arg(long = "club", default_value = crate::train::DEFAULT_CLUB)]
    pub club: String,
    /// Число эпох обучения
    #[arg(long = "train-epochs", default_value_t = crate::train::DEFAULT_EPOCHS)]
    pub train_epochs: u32,
    /// Число бойцов в клубе
    #[arg(long = "train-fighters", default_value_t = crate::train::DEFAULT_FIGHTERS)]
    pub train_fighters: usize,
    /// Число спарингов на пару за эпоху
    #[arg(long = "train-sparring", default_value_t = crate::train::DEFAULT_SPARINGS)]
    pub train_sparring: u32,
    /// Базовое зерно обучения: одинаковое зерно — одинаковый прогон
    #[arg(long = "train-seed", default_value_t = crate::train::DEFAULT_SEED)]
    pub train_seed: u64,
    /// Каталог, куда складываются бэкапы и отчёт обучения
    #[arg(long = "train-out", default_value = crate::train::DEFAULT_CLUB)]
    pub train_out: String,
}

impl Cli {
    /// Разбирает аргументы командной строки.
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

    #[test]
    fn no_gui_flag_is_parsed() {
        let cli = Cli::try_parse_from(["fencing", "--no-gui"]).unwrap();
        assert!(cli.no_gui);
    }

    #[test]
    fn no_gui_flag_is_absent_by_default() {
        let cli = Cli::try_parse_from(["fencing"]).unwrap();
        assert!(!cli.no_gui);
    }

    #[test]
    fn brains_are_dummies_by_default() {
        let cli = Cli::try_parse_from(["fencing"]).unwrap();

        assert_eq!(cli.red, BrainKind::Doll);
        assert_eq!(cli.blue, BrainKind::Doll);
    }

    #[test]
    fn red_brain_can_be_player() {
        let cli = Cli::try_parse_from(["fencing", "--red=player"]).unwrap();

        assert_eq!(cli.red, BrainKind::Player);
    }

    #[test]
    fn blue_brain_can_be_a_file() {
        let cli =
            Cli::try_parse_from(["fencing", "--blue=firsts/epoch_08/firsts_00.json"]).unwrap();

        assert_eq!(
            cli.blue,
            BrainKind::File(PathBuf::from("firsts/epoch_08/firsts_00.json"))
        );
    }

    #[test]
    fn brains_can_be_set_separately() {
        let cli = Cli::try_parse_from(["fencing", "--red=player", "--blue=doll"]).unwrap();

        assert_eq!(cli.red, BrainKind::Player);
        assert_eq!(cli.blue, BrainKind::Doll);
    }
}
