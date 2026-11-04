//! Сборка мозга бойца по значению из командной строки.
//!
//! Значение флага может быть именем встроенного мозга (`player`, `doll`,
//! `perceptron`) или путём к файлу с обученным мозгом. В файле сам
//! объявлено, какой моделью мозга он создан, поэтому по одному пути можно
//! загрузить и стратега, и сеть.

use std::{
    io,
    path::{Path, PathBuf},
    str::FromStr,
};

use serde_json::Value;

use fencing_core::brain::{Brain, doll::Doll};
use fencing_engine::control::PlayerControl;
use fencing_perceptron::{MODEL_NAME as PERCEPTRON_MODEL, Perceptron};
use fencing_strategist::{MODEL_NAME as STRATEGIST_MODEL, Strategist};

/// Источник мозга бойца
#[derive(Clone, Debug, PartialEq)]
pub enum BrainKind {
    /// Мозг под управлением игрока
    Player,
    /// Заглушка: боец ничего не делает
    Doll,
    /// Мозг на нейросети со случайными весами
    Perceptron,
    /// Обученный мозг, сохранённый в файл
    File(PathBuf),
}

impl FromStr for BrainKind {
    type Err = String;

    /// Разбирает значение флага.
    ///
    /// Всё, что не является именем встроенного мозга, считается путём
    /// к файлу: существование файла проверяется уже при загрузке.
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Ok(match value {
            "player" => BrainKind::Player,
            "doll" => BrainKind::Doll,
            "perceptron" => BrainKind::Perceptron,
            path => BrainKind::File(PathBuf::from(path)),
        })
    }
}

impl BrainKind {
    /// Собирает мозга бойца по этому источнику.
    ///
    /// В режиме без окна управлять бойцом нечем: клавиатуры и мыши
    /// там недоступны, поэтому такой мозг заменяется заглушкой.
    pub fn build(&self, headless: bool) -> io::Result<Box<dyn Brain>> {
        match self {
            BrainKind::Player if headless => Ok(Box::new(Doll)),
            BrainKind::Player => Ok(Box::new(PlayerControl)),
            BrainKind::Doll => Ok(Box::new(Doll)),
            BrainKind::Perceptron => Ok(Box::new(Perceptron::random(fastrand::u64(..)))),
            BrainKind::File(path) => Ok(load_brain(path)?),
        }
    }
}

/// Загружает обученный мозг из файла.
///
/// Модель мозга указана в самом файле: по ней выбирается, чем этот мозг
/// является — стратегом или сетью.
///
/// В ошибке всегда виден путь к файлу: иначе непонятно, какой из двух
/// мозгов не удалось загрузить.
pub fn load_brain(path: &Path) -> io::Result<Box<dyn Brain>> {
    let text = std::fs::read_to_string(path).map_err(|err| {
        io::Error::new(
            err.kind(),
            format!("не удалось прочитать мозг из \"{}\": {err}", path.display()),
        )
    })?;

    let value: Value = serde_json::from_str(&text).map_err(|err| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("не удалось разобрать мозг из \"{}\": {err}", path.display()),
        )
    })?;

    let model = value.get("model").and_then(Value::as_str).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("в файле \"{}\" нет метки модели мозга", path.display()),
        )
    })?;

    match model {
        STRATEGIST_MODEL => Ok(Box::new(load_strategist(path)?)),
        PERCEPTRON_MODEL => {
            let saved = Perceptron::load_brain(path)?;

            Ok(Box::new(Perceptron::from_saved(saved)?))
        }
        other => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "неизвестная модель мозга \"{other}\" в файле \"{}\"",
                path.display()
            ),
        )),
    }
}

/// Загружает обученного стратега из файла.
///
/// В ошибке всегда виден путь к файлу: иначе непонятно, какой из двух
/// мозгов не удалось загрузить.
pub fn load_strategist(path: &Path) -> io::Result<Strategist> {
    let saved = Strategist::load_brain(path).map_err(|err| {
        io::Error::new(
            err.kind(),
            format!("не удалось загрузить мозг из \"{}\": {err}", path.display()),
        )
    })?;

    Ok(Strategist::with_seed_and_table(saved.seed, saved.table))
}

#[cfg(test)]
mod tests {
    use super::*;
    use fencing_strategist::table::TABLE_SIZE;

    /// Временный файл с обученным мозгом: путь уникален для каждого теста
    fn temp_path(name: &str) -> PathBuf {
        let mut path = std::env::temp_dir();
        path.push(format!("fencing-brain-{}-{}", std::process::id(), name));

        path
    }

    /// Ошибка из результата: мозги не обязаны уметь показываться отладчиком,
    /// поэтому `unwrap_err` здесь не годится
    fn expect_err<T>(result: io::Result<T>) -> io::Error {
        match result {
            Ok(_) => panic!("ожидалась ошибка"),
            Err(err) => err,
        }
    }

    #[test]
    fn player_is_parsed_as_player_brain() {
        assert_eq!("player".parse(), Ok(BrainKind::Player));
    }

    #[test]
    fn doll_is_parsed_as_dummy_brain() {
        assert_eq!("doll".parse(), Ok(BrainKind::Doll));
    }

    #[test]
    fn perceptron_is_parsed_as_builtin_brain() {
        assert_eq!("perceptron".parse(), Ok(BrainKind::Perceptron));
    }

    #[test]
    fn path_is_parsed_as_file() {
        let kind: BrainKind = "firsts/epoch_08/firsts_00.json".parse().unwrap();

        assert_eq!(
            kind,
            BrainKind::File(PathBuf::from("firsts/epoch_08/firsts_00.json"))
        );
    }

    #[test]
    fn player_brain_is_built() {
        assert!("player".parse::<BrainKind>().unwrap().build(false).is_ok());
    }

    #[test]
    fn dummy_brain_is_built() {
        assert!("doll".parse::<BrainKind>().unwrap().build(false).is_ok());
    }

    #[test]
    fn perceptron_brain_is_built() {
        assert!(
            "perceptron"
                .parse::<BrainKind>()
                .unwrap()
                .build(false)
                .is_ok()
        );
    }

    #[test]
    fn player_brain_in_headless_falls_back_to_dummy() {
        // Клавиатуры в режиме без окна нет, но ошибкой это не считается:
        // боец просто ничего не делает
        assert!("player".parse::<BrainKind>().unwrap().build(true).is_ok());
    }

    #[test]
    fn missing_file_is_reported_with_path() {
        let kind = BrainKind::File(PathBuf::from("no/such/brain.json"));

        let err = expect_err(kind.build(false));

        assert!(err.to_string().contains("brain.json"), "{err}");
    }

    #[test]
    fn saved_brain_is_loaded() {
        let path = temp_path("load");

        let strategist = Strategist::with_seed(42);
        strategist.save_to_file(&path, "test").unwrap();

        let loaded = load_strategist(&path).unwrap();

        assert_eq!(loaded.seed(), 42);
        assert_eq!(loaded.table().len(), TABLE_SIZE);

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn brain_of_another_model_is_rejected() {
        let path = temp_path("foreign");

        // Файл с чужой меткой модели: такой мозг принимать нельзя
        let text = r#"{"model":"doll","version":2,"name":"x","seed":1,"table":{"entries":[]}}"#;
        std::fs::write(&path, text).unwrap();

        let err = expect_err(load_strategist(&path));

        assert!(err.to_string().contains("doll"), "{err}");

        let _ = std::fs::remove_file(&path);
    }

    /// Файл стратега грузится загрузчиком мозгов
    #[test]
    fn strategist_file_is_loaded_by_model() {
        let path = temp_path("strategist-by-model");

        let strategist = Strategist::with_seed(42);
        strategist.save_to_file(&path, "test").unwrap();

        assert!(load_brain(&path).is_ok());

        let _ = std::fs::remove_file(&path);
    }

    /// Файл сети грузится загрузчиком мозгов
    #[test]
    fn perceptron_file_is_loaded_by_model() {
        let path = temp_path("perceptron-by-model");

        let brain = Perceptron::random(7);
        brain.save_to_file(&path, "test").unwrap();

        assert!(load_brain(&path).is_ok());

        let _ = std::fs::remove_file(&path);
    }

    /// Неизвестная модель мозга в файле — понятная ошибка
    #[test]
    fn unknown_model_is_reported() {
        let path = temp_path("unknown-model");

        std::fs::write(&path, r#"{"model":"что-то ещё","version":1}"#).unwrap();

        let err = expect_err(load_brain(&path));

        assert!(err.to_string().contains("что-то ещё"), "{err}");
        assert!(err.to_string().contains("unknown-model"), "{err}");

        let _ = std::fs::remove_file(&path);
    }

    /// Файл без метки модели — понятная ошибка
    #[test]
    fn file_without_model_is_reported() {
        let path = temp_path("no-model");

        std::fs::write(&path, r#"{"version":1}"#).unwrap();

        let err = expect_err(load_brain(&path));

        assert!(err.to_string().contains("метки модели"), "{err}");

        let _ = std::fs::remove_file(&path);
    }
}
