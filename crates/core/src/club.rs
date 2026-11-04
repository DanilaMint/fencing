//! Общие помощники clubs: запись и чтение бэкапов.
//!
//! Оба клуба — стратегов и перцептронов — раскладывают свои находки по
//! одному и тому же принципу: в каталоге появляются подкаталоги
//! `epoch_NN`, в каждом лежит `club.json` со всем состоянием клуба и
//! отдельные файлы с мозгами бойцов. Помощники этого модуля — общие для
//! обоих клубов, чтобы правила хранения не расходились.

use std::{
    fs,
    io,
    path::{Path, PathBuf},
};

use serde::{Serialize, de::DeserializeOwned};

/// Имя файла с общим состоянием клуба
pub const CLUB_FILE: &str = "club.json";
/// Префикс подкаталогов поколений
pub const EPOCH_PREFIX: &str = "epoch_";

/// Пишет JSON во временный файл и переименовывает его:
/// так прерванная запись не оставит битый бэкап
pub fn write_json<T: Serialize>(path: &Path, value: &T) -> io::Result<()> {
    let text = serde_json::to_string_pretty(value).map_err(data_error)?;

    let tmp = path.with_extension("tmp");
    fs::write(&tmp, text)?;
    fs::rename(&tmp, path)
}

/// Читает JSON из файла
pub fn load_json<T: DeserializeOwned>(path: &Path) -> io::Result<T> {
    let text = fs::read_to_string(path)?;

    serde_json::from_str(&text).map_err(data_error)
}

/// Ошибка разбора или записи JSON переводится в ошибку ввода-вывода
pub fn data_error(err: serde_json::Error) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, err)
}

/// Ищет последний сохранённый бэкап клуба
///
/// Возвращает номер поколения и путь к файлу состояния.
pub fn find_latest_backup(dir: &Path) -> Option<(u32, PathBuf)> {
    let mut latest: Option<(u32, PathBuf)> = None;

    for entry in fs::read_dir(dir).ok()?.flatten() {
        let path = entry.path();

        if !path.is_dir() {
            continue;
        }

        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };

        let Some(number) = name
            .strip_prefix(EPOCH_PREFIX)
            .and_then(|rest| rest.parse::<u32>().ok())
        else {
            continue;
        };

        if !path.join(CLUB_FILE).exists() {
            continue;
        }

        if latest.as_ref().is_none_or(|(best, _)| number > *best) {
            latest = Some((number, path.join(CLUB_FILE)));
        }
    }

    latest
}

/// Путь к каталогу поколения
pub fn epoch_dir(dir: &Path, epoch: u32) -> PathBuf {
    dir.join(format!("{EPOCH_PREFIX}{epoch:02}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct Sample {
        name: String,
        value: u32,
    }

    fn temp_dir(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!("fencing_club_test_{name}"));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();

        path
    }

    /// JSON пишется и читается обратно
    #[test]
    fn json_survives_write_and_load() {
        let dir = temp_dir("json");
        let path = dir.join("sample.json");

        let sample = Sample {
            name: "клуб".to_string(),
            value: 42,
        };

        write_json(&path, &sample).unwrap();

        let loaded: Sample = load_json(&path).unwrap();

        assert_eq!(loaded, sample);

        let _ = fs::remove_dir_all(&dir);
    }

    /// После записи не остаётся временных файлов
    #[test]
    fn write_leaves_no_temp_file() {
        let dir = temp_dir("temp");
        let path = dir.join("sample.json");

        write_json(
            &path,
            &Sample {
                name: "клуб".to_string(),
                value: 1,
            },
        )
        .unwrap();

        assert!(!path.with_extension("tmp").exists());

        let _ = fs::remove_dir_all(&dir);
    }

    /// Последний бэкап — самый новый по номеру
    #[test]
    fn latest_backup_is_the_newest_one() {
        let dir = temp_dir("latest");

        for epoch in [1u32, 3, 2] {
            let epoch_dir = epoch_dir(&dir, epoch);
            fs::create_dir_all(&epoch_dir).unwrap();
            write_json(
                &epoch_dir.join(CLUB_FILE),
                &Sample {
                    name: format!("поколение {epoch}"),
                    value: epoch,
                },
            )
            .unwrap();
        }

        // Каталог без файла состояния в поиск не попадает
        fs::create_dir_all(epoch_dir(&dir, 4)).unwrap();

        let (epoch, path) = find_latest_backup(&dir).unwrap();

        assert_eq!(epoch, 3);
        assert_eq!(path, epoch_dir(&dir, 3).join(CLUB_FILE));

        let _ = fs::remove_dir_all(&dir);
    }

    /// Без бэкапов поиск ничего не находит
    #[test]
    fn latest_backup_is_absent_in_empty_dir() {
        let dir = temp_dir("empty");

        assert!(find_latest_backup(&dir).is_none());

        let _ = fs::remove_dir_all(&dir);
    }
}
