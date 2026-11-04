//! Интеграционные тесты режима генетического обучения.
//!
//! Программа запускается как настоящий бинарник: проверяем, что файлы
//! клуба появляются, обучение продолжается с прошлого места, а мозг элиты
//! годится для боя.

use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

/// Каталог для теста: уникальный, чтобы тесты не мешали друг другу
fn test_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("fencing_eugenics_cli_{name}"));
    let _ = fs::remove_dir_all(&dir);

    dir
}

/// Запуск программы с набором аргументов
fn run(dir: &Path, args: &[&str]) {
    let output = Command::new(env!("CARGO_BIN_EXE_fencing"))
        .args(args)
        .arg(format!("--out={}", dir.display()))
        .output()
        .expect("программа не запустилась");

    assert!(
        output.status.success(),
        "программа завершилась с ошибкой: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Обучение создаёт файлы клуба
#[test]
fn training_creates_club_files() {
    let dir = test_dir("create");

    run(
        &dir,
        &[
            "train-eugenics",
            "--generations=1",
            "--elite=2",
            "--offspring=2",
            "--hidden=4",
        ],
    );

    let epoch = dir.join("epoch_01");

    assert!(epoch.join("club.json").exists(), "нет файла клуба");
    assert!(epoch.join("club_eugenics_00.json").exists(), "нет мозга элиты");
    assert!(epoch.join("club_eugenics_01.json").exists(), "нет мозга элиты");
    assert!(dir.join("report.md").exists(), "нет отчёта");

    // В отчёте есть таблицы
    let report = fs::read_to_string(dir.join("report.md")).unwrap();
    assert!(report.contains("## Динамика по поколениям"));

    let _ = fs::remove_dir_all(&dir);
}

/// Обучение продолжается с прошлого поколения
#[test]
fn training_resumes_from_saved_generation() {
    let dir = test_dir("resume");

    run(
        &dir,
        &[
            "train-eugenics",
            "--generations=1",
            "--elite=2",
            "--offspring=2",
            "--hidden=4",
        ],
    );

    assert!(dir.join("epoch_01").exists(), "первого поколения нет");

    run(
        &dir,
        &[
            "train-eugenics",
            "--generations=2",
            "--elite=2",
            "--offspring=2",
            "--hidden=4",
        ],
    );

    assert!(dir.join("epoch_02").exists(), "второго поколения нет");

    let _ = fs::remove_dir_all(&dir);
}

/// Мозг элиты годится для боя
#[test]
fn elite_brain_is_usable_in_fight() {
    let dir = test_dir("fight");

    run(
        &dir,
        &[
            "train-eugenics",
            "--generations=1",
            "--elite=2",
            "--offspring=2",
            "--hidden=4",
        ],
    );

    let red = dir.join("epoch_01/club_eugenics_00.json");
    let blue = dir.join("epoch_01/club_eugenics_01.json");

    let output = Command::new(env!("CARGO_BIN_EXE_fencing"))
        .args(["fight", "--no-gui"])
        .arg(format!("--red={}", red.display()))
        .arg(format!("--blue={}", blue.display()))
        .output()
        .expect("программа не запустилась");

    assert!(
        output.status.success(),
        "бой не прошёл: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let _ = fs::remove_dir_all(&dir);
}
