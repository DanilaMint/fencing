//! Сборка мозга бойца по значению из командной строки.
//!
//! Значение флага может быть именем встроенного мозга (`player`, `doll`)
//! или путём к файлу с обученной таблицей реакций.

use std::{
    io,
    path::{Path, PathBuf},
    str::FromStr,
};

use super::{Brain, control::PlayerControl, doll::Doll, strategist::Strategist};

/// Источник мозга бойца
#[derive(Clone, Debug, PartialEq)]
pub enum BrainKind {
    /// Мозг под управлением игрока
    Player,
    /// Заглушка: боец ничего не делает
    Doll,
    /// Обученный стратег, сохранённый в файл
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
            path => BrainKind::File(PathBuf::from(path)),
        })
    }
}

impl BrainKind {
    /// Собирает мозга бойца по этому источнику.
    ///
    /// В режиме без окна управлять бойцом нечем: клавиатура и мышь
    /// там недоступны, поэтому такой мозг заменяется заглушкой.
    pub fn build(&self, headless: bool) -> io::Result<Box<dyn Brain>> {
        match self {
            BrainKind::Player if headless => Ok(Box::new(Doll)),
            BrainKind::Player => Ok(Box::new(PlayerControl)),
            BrainKind::Doll => Ok(Box::new(Doll)),
            BrainKind::File(path) => Ok(Box::new(load_strategist(path)?)),
        }
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
    use crate::fighter::brain::strategist::table::TABLE_SIZE;

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
}
