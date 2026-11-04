//! Сравнение двух групп бойцов.
//!
//! Инструмент для проверки эволюции: берём мозги двух поколений и смотрим,
//! кто выигрывает. Если новое поколение побеждает старое заметно чаще, значит
//! отбор работает; если разница в пределах шума, эволюция топчется на месте.

use std::{
    fs,
    io,
    path::Path,
};

use fencing_perceptron::{Perceptron, net::Net};

use crate::{arena::benchmark, scoring::Scoring};

/// Сравнивает две группы бойцов
///
/// `first` и `second` — каталоги с мозгами двух поколений.
pub fn run(
    first_dir: &Path,
    second_dir: &Path,
    repeat: u32,
    scoring: Scoring,
) -> io::Result<()> {
    let first = load_nets(first_dir)?;
    let second = load_nets(second_dir)?;

    if first.is_empty() || second.is_empty() {
        eprintln!("В одной из групп нет бойцов");
        return Ok(());
    }

    let result = benchmark(&first, &second, repeat, scoring);

    println!(
        "Первая группа: {} бойцов ({}), вторая: {} бойцов ({})",
        first.len(),
        first_dir.display(),
        second.len(),
        second_dir.display()
    );
    println!(
        "Боёв: {}   победы первой группы: {}   ничьи: {}   поражения: {}",
        result.fights, result.wins, result.draws, result.losses
    );
    println!(
        "Доля побед первой группы: {:.1}%   разница очков: {}",
        result.win_rate() * 100.,
        result.points
    );

    Ok(())
}

/// Читает мозги из каталога: все файлы `.json` с перцептроном
pub fn load_nets(dir: &Path) -> io::Result<Vec<Net>> {
    let mut nets = Vec::new();

    for entry in fs::read_dir(dir)? {
        let path = entry?.path();

        if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
            continue;
        }

        // club.json — состояние клуба, а не мозг: его пропускаем
        let Some(saved) = Perceptron::load_brain(&path).ok() else {
            continue;
        };

        nets.push(saved.net);
    }

    nets.sort_by_key(|net| net.w1.len());

    Ok(nets)
}
