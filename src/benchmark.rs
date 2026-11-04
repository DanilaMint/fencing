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

use crate::{
    cli::BenchmarkArgs,
    fighter::brain::perceptron::{Perceptron, net::Net},
};

use crate::eugenics::arena::benchmark;

/// Сравнивает две группы бойцов
pub fn run(args: &BenchmarkArgs) -> io::Result<()> {
    let first = load_nets(&args.first)?;
    let second = load_nets(&args.second)?;

    if first.is_empty() || second.is_empty() {
        eprintln!("В одной из групп нет бойцов");
        return Ok(());
    }

    let result = benchmark(&first, &second, args.repeat, args.scoring);

    println!(
        "Первая группа: {} бойцов ({}), вторая: {} бойцов ({})",
        first.len(),
        args.first.display(),
        second.len(),
        args.second.display()
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
