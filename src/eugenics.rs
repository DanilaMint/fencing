//! Генетическое обучение клуба бойцов.
//!
//! Клуб состоит из элиты — самых сильных бойцов прошлого поколения. В каждом
//! поколении элита оставляет наследников: их геномы скрещиваются и мутируют.
//! Получившийся отряд (элита плюс наследники) проводит круговой турнир: каждый
//! дерётся с каждым, за победу начисляются очки. В следующее поколение
//! проходят восемь лучших.
//!
//! Геном бойца — сеть перцептрона: восемь входов, скрытый слой и три группы
//! выходов. Отбор идёт по очкам, поэтому побеждает не тот, кто дольше стоит,
//! а тот, кто укладывает противников в нокаут.

pub mod arena;
pub mod genome;
pub mod report;
pub mod scoring;

use std::{
    cmp::Ordering,
    io,
    path::Path,
    time::Instant,
};

use fastrand::Rng;
use serde::{Deserialize, Serialize};

use crate::{
    cli::TrainEugenicsArgs,
    club::{CLUB_FILE, epoch_dir, find_latest_backup, load_json, write_json},
    fighter::brain::perceptron::{Perceptron, net::Net},
};

use self::{
    arena::{ArenaResult, FighterStats},
    genome::{crossover, difference, mutate},
    scoring::Scoring,
};

/// Имя клуба по умолчанию
pub const DEFAULT_CLUB: &str = "club_eugenics";
/// Размер элиты по умолчанию
pub const DEFAULT_ELITE: usize = 8;
/// Число наследников за поколение по умолчанию
pub const DEFAULT_OFFSPRING: usize = 56;
/// Число поколений по умолчанию
pub const DEFAULT_GENERATIONS: u32 = 20;
/// Число боёв на пару за поколение по умолчанию
pub const DEFAULT_REPEAT: u32 = 1;
/// Версия формата состояния клуба
pub const FORMAT_VERSION: u32 = 1;
/// Модель клуба: та же схема, что и у мозгов, — по ней файл опознаётся
pub const MODEL_NAME: &str = "eugenics";

/// Число боёв на пару при проверке силы по умолчанию
fn default_benchmark_repeat() -> u32 {
    3
}

/// Настройки генетического алгоритма
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GaConfig {
    /// Имя клуба
    pub club: String,
    /// Размер элиты
    pub elite: usize,
    /// Число наследников за поколение
    pub offspring: usize,
    /// Число поколений
    pub generations: u32,
    /// Число боёв на пару за поколение
    pub repeat: u32,
    /// Ширина скрытого слоя сети
    pub hidden: usize,
    /// Вероятность мутации гена
    pub mutation_rate: f32,
    /// Величина мутации: доля от предела слоя
    pub mutation_scale: f32,
    /// Доля генов, берущихся от первого родителя
    pub crossover_rate: f32,
    /// Размер турнира при выборе родителей
    pub tournament: usize,
    /// Расценка очков
    pub scoring: Scoring,
    /// Число боёв на пару при проверке силы
    #[serde(default = "default_benchmark_repeat")]
    pub benchmark_repeat: u32,
    /// Базовое зерно
    pub seed: u64,
}

impl GaConfig {
    /// Настройки по аргументам командной строки
    pub fn from_args(args: &TrainEugenicsArgs) -> Self {
        Self {
            club: args.club.clone(),
            elite: args.elite.max(2),
            offspring: args.offspring,
            generations: args.generations,
            repeat: args.repeat.max(1),
            hidden: args.hidden,
            mutation_rate: args.mutation_rate,
            mutation_scale: args.mutation_scale,
            crossover_rate: args.crossover_rate,
            tournament: args.tournament.max(1),
            scoring: args.scoring,
            benchmark_repeat: args.benchmark_repeat.max(1),
            seed: args.seed,
        }
    }

    /// Число бойцов в поколении: элита плюс наследники
    pub fn population_size(&self) -> usize {
        self.elite + self.offspring
    }
}

/// Боец элиты: его геном, имя и результат последнего поколения
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Elite {
    /// Имя бойца
    pub name: String,
    /// Геном: сеть перцептрона
    pub net: Net,
    /// Очки в последнем поколении
    pub score: i32,
    /// Статистика последнего поколения
    pub stats: FighterStats,
}

/// Сводка по поколению
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GenerationReport {
    /// Номер поколения
    pub generation: u32,
    /// Число боёв
    pub fights: u32,
    /// Время поколения, секунды
    pub seconds: f32,
    /// Средняя длина боя, тиков
    pub avg_ticks: f32,
    /// Лучший счёт в поколении
    pub best_score: i32,
    /// Средний счёт по всем бойцам
    pub mean_score: f32,
    /// Худший счёт в поколении
    pub worst_score: i32,
    /// Боёв, закончившихся ничьёй
    pub draws: u32,
    /// Боёв, закончившихся выходом за ринг
    pub out_of_ring: u32,
    /// Боёв, закончившихся нокаутом от перерасхода энергии
    pub exhausted: u32,
    /// Боёв, закончившихся нокаутом от удара
    pub defeated: u32,
    /// Разнообразие элиты: средняя доля различающихся генов
    pub diversity: f32,
    /// Доля побед элиты над замороженной панелью первого поколения
    #[serde(default)]
    pub strength: f32,
    /// Число боёв в проверке силы
    #[serde(default)]
    pub strength_fights: u32,
}

/// Клуб бойцов: элита и история поколений
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Club {
    /// Модель клуба: по ней файл опознаётся
    pub model: String,
    /// Версия формата
    pub version: u32,
    /// Имя клуба
    pub name: String,
    /// Номер последнего завершённого поколения
    pub generation: u32,
    /// Настройки обучения
    pub config: GaConfig,
    /// Элита текущего поколения
    pub elites: Vec<Elite>,
    /// Опорная панель: замороженная элита первого поколения.
    ///
    /// С ней меряется сила текущего поколения: победы над ней показывают,
    /// что эволюция идёт, а не топчется на месте.
    #[serde(default)]
    pub reference: Vec<Net>,
    /// Сводки по прошлым поколениям
    pub history: Vec<GenerationReport>,
}

impl Club {
    /// Новый клуб: элита со случайными геномами
    fn new(config: GaConfig) -> Self {
        let elites: Vec<Elite> = (0..config.elite)
            .map(|slot| Elite {
                name: format!("{}_{slot:02}", config.club),
                net: Net::random(config.hidden, fastrand::u64(..)),
                score: 0,
                stats: FighterStats::default(),
            })
            .collect();

        // Первое поколение становится опорной панелью: с ним будут сравнивать
        // все следующие
        let reference = elites.iter().map(|elite| elite.net.clone()).collect();

        Self {
            model: MODEL_NAME.to_string(),
            version: FORMAT_VERSION,
            name: config.club.clone(),
            generation: 0,
            config,
            elites,
            reference,
            history: Vec::new(),
        }
    }
}

/// Обучает клуб и сохраняет результаты
pub fn run(args: &TrainEugenicsArgs) -> io::Result<()> {
    let config = GaConfig::from_args(args);

    fastrand::seed(config.seed);

    let dir = Path::new(&args.out);
    let mut club = load_club(&config, dir)?;

    println!(
        "Клуб «{}»: элита {}, наследников {}, всего бойцов в поколении {}",
        club.config.club,
        club.config.elite,
        club.config.offspring,
        club.config.population_size()
    );
    println!(
        "Поколений: {}   ширина сети: {}   боёв на пару: {}",
        club.config.generations, club.config.hidden, club.config.repeat
    );

    if club.generation > 0 {
        println!(
            "Продолжаем с поколения {}\n",
            club.generation + 1
        );
    }

    while club.generation < club.config.generations {
        let generation = club.generation + 1;
        let report = run_generation(&mut club, generation)?;

        club.history.push(report.clone());

        save_generation(dir, &club)?;
        print_generation(&club, &report);
    }

    report::write_report(dir, &club)?;

    println!("\nОтчёт обучения: {}", dir.join("report.md").display());

    Ok(())
}

/// Загружает клуб из последнего бэкапа или создаёт новый
fn load_club(config: &GaConfig, dir: &Path) -> io::Result<Club> {
    let Some((generation, path)) = find_latest_backup(dir) else {
        return Ok(Club::new(config.clone()));
    };

    let club: Club = load_json(&path)?;

    if club.model != MODEL_NAME {
        eprintln!(
            "Внимание: бэкап сохранён для модели \"{}\", а загружен будет \"{}\"",
            club.model, MODEL_NAME
        );
    }

    // Настройки берём из текущего запуска: командуда может поменять параметры
    let mut club = club;
    club.config = config.clone();
    club.generation = generation;

    // Размер элиты тоже мог поменяться: лишних отбрасываем, они и так
    // отсортированы по счёту
    if club.elites.len() > club.config.elite {
        eprintln!(
            "Внимание: в бэкапе {} бойцов элиты, а взято {}. Лишние отброшены",
            club.elites.len(),
            club.config.elite
        );

        club.elites.truncate(club.config.elite);
    }

    // Опорной панели в старых бэкапах нет: берём первое поколение из каталога
    if club.reference.is_empty() {
        let first = crate::club::epoch_dir(dir, 1);

        club.reference = crate::benchmark::load_nets(&first).unwrap_or_default();
    }

    Ok(club)
}

/// Обучает одно поколение и обновляет элиту
fn run_generation(club: &mut Club, generation: u32) -> io::Result<GenerationReport> {
    let started = Instant::now();

    let population = build_population(&club.config, &club.elites)?;

    let result = arena::run_arena(&population, club.config.repeat, club.config.scoring);

    // Отбор: лучшие по очкам, потом по победам, потом по нокаутам от удара
    let mut order: Vec<usize> = (0..population.len()).collect();
    order.sort_by(|a, b| compare(&result, *a, *b));

    let mut elites = Vec::with_capacity(club.config.elite);

    for (slot, index) in order.iter().take(club.config.elite).enumerate() {
        elites.push(Elite {
            name: format!("{}_{slot:02}", club.config.club),
            net: population[*index].clone(),
            score: result.scores[*index],
            stats: result.stats[*index].clone(),
        });
    }

    let diversity = diversity(&elites);

    // Проверка силы: элита против замороженной панели первого поколения
    let reference: Vec<Net> = if club.reference.is_empty() {
        club.elites.iter().map(|elite| elite.net.clone()).collect()
    } else {
        club.reference.clone()
    };

    let elite_nets: Vec<Net> = elites.iter().map(|elite| elite.net.clone()).collect();
    let check = arena::benchmark(
        &elite_nets,
        &reference,
        club.config.benchmark_repeat,
        club.config.scoring,
    );

    let best_score = result.scores.iter().copied().max().unwrap_or(0);
    let worst_score = result.scores.iter().copied().min().unwrap_or(0);
    let mean_score = if result.scores.is_empty() {
        0.
    } else {
        result.scores.iter().sum::<i32>() as f32 / result.scores.len() as f32
    };

    let report = GenerationReport {
        generation,
        fights: result.fights,
        seconds: started.elapsed().as_secs_f32(),
        avg_ticks: if result.fights == 0 {
            0.
        } else {
            result.total_ticks as f32 / result.fights as f32
        },
        best_score,
        mean_score,
        worst_score,
        draws: result.draws,
        out_of_ring: result.out_of_ring,
        exhausted: result.exhausted,
        defeated: result.defeated,
        diversity,
        strength: check.win_rate(),
        strength_fights: check.fights,
    };

    club.elites = elites;
    club.generation = generation;

    Ok(report)
}

/// Собирает популяцию поколения: элита и её наследники
fn build_population(config: &GaConfig, elites: &[Elite]) -> io::Result<Vec<Net>> {
    let mut population: Vec<Net> = elites.iter().map(|elite| elite.net.clone()).collect();

    let scores: Vec<i32> = elites.iter().map(|elite| elite.score).collect();
    let mut rng = Rng::new();

    for _ in 0..config.offspring {
        let mother = pick_parent(config, &scores, &mut rng);
        let mut father = pick_parent(config, &scores, &mut rng);

        // Родители должны быть разными: иначе наследник — копия одного
        let mut attempts = 0;
        while father == mother && attempts < 16 {
            father = pick_parent(config, &scores, &mut rng);
            attempts += 1;
        }

        let mut child = crossover(
            &elites[mother].net,
            &elites[father].net,
            config.crossover_rate,
            &mut rng,
        )
        .map_err(|reason| io::Error::new(io::ErrorKind::InvalidInput, reason))?;

        mutate(
            &mut child,
            config.mutation_rate,
            config.mutation_scale,
            &mut rng,
        );

        population.push(child);
    }

    Ok(population)
}

/// Выбор родителя: турнир по очкам
///
/// Очки могут быть отрицательными, поэтому рулетка по очкам не годится:
/// турнир устойчив к любой шкале и оставляет слабому бойцу шанс.
fn pick_parent(config: &GaConfig, scores: &[i32], rng: &mut Rng) -> usize {
    if scores.len() == 1 {
        return 0;
    }

    let size = config.tournament.min(scores.len());

    let mut best = rng.usize(0..scores.len());

    for _ in 1..size {
        let candidate = rng.usize(0..scores.len());

        if scores[candidate] > scores[best] {
            best = candidate;
        }
    }

    best
}

/// Порядок бойцов: лучший — первый
fn compare(result: &ArenaResult, first: usize, second: usize) -> Ordering {
    result.scores[second]
        .cmp(&result.scores[first])
        .then_with(|| result.stats[second].wins.cmp(&result.stats[first].wins))
        .then_with(|| {
            result.stats[second]
                .wins_by_defeat
                .cmp(&result.stats[first].wins_by_defeat)
        })
        .then_with(|| result.stats[first].losses.cmp(&result.stats[second].losses))
        .then_with(|| first.cmp(&second))
}

/// Разнообразие элиты: средняя доля различающихся генов
fn diversity(elites: &[Elite]) -> f32 {
    if elites.len() < 2 {
        return 1.;
    }

    let mut sum = 0.;
    let mut pairs = 0usize;

    for first in 0..elites.len() {
        for second in first + 1..elites.len() {
            sum += difference(&elites[first].net, &elites[second].net);
            pairs += 1;
        }
    }

    if pairs == 0 {
        return 1.;
    }

    sum / pairs as f32
}

/// Сохраняет состояние клуба и мозги элиты
fn save_generation(dir: &Path, club: &Club) -> io::Result<()> {
    let epoch = epoch_dir(dir, club.generation);

    std::fs::create_dir_all(&epoch)?;

    write_json(&epoch.join(CLUB_FILE), club)?;

    for elite in &club.elites {
        let brain = Perceptron::new(elite.net.clone(), fastrand::u64(..));
        brain.save_to_file(epoch.join(format!("{}.json", elite.name)), &elite.name)?;
    }

    Ok(())
}

/// Печатает сводку по поколению
fn print_generation(club: &Club, report: &GenerationReport) {
    println!(
        "\nПоколение {}: боёв {}   время {:.1} с   средний бой {:.0} тиков",
        report.generation, report.fights, report.seconds, report.avg_ticks
    );
    println!(
        "Исходы: нокаут от удара {} | нокаут от энергии {} | из кольца {} | ничьи {}",
        report.defeated, report.exhausted, report.out_of_ring, report.draws
    );
    println!(
        "Очки: лучший {} | средний {:.1} | худший {}   разнообразие элиты {:.1}%",
        report.best_score,
        report.mean_score,
        report.worst_score,
        report.diversity * 100.
    );
    println!(
        "Сила против первого поколения: {:.1}% побед из {} боёв",
        report.strength * 100.,
        report.strength_fights
    );

    println!(
        "\n{:<16} {:>7} {:>5} {:>5} {:>5} {:>7} {:>7}",
        "Боец", "Очки", "Поб", "Нич", "Праж", "Нокаут", "Из кольца"
    );

    for elite in &club.elites {
        println!(
            "{:<16} {:>7} {:>5} {:>5} {:>5} {:>7} {:>7}",
            elite.name,
            elite.score,
            elite.stats.wins,
            elite.stats.draws,
            elite.stats.losses,
            elite.stats.wins_by_defeat + elite.stats.wins_by_exhaustion,
            elite.stats.wins_by_ring,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::TrainEugenicsArgs;

    fn config(elite: usize, offspring: usize) -> GaConfig {
        GaConfig {
            club: "club_test".to_string(),
            elite,
            offspring,
            generations: 1,
            repeat: 1,
            hidden: 4,
            mutation_rate: 0.1,
            mutation_scale: 0.3,
            crossover_rate: 0.5,
            tournament: 3,
            scoring: Scoring::Classic,
            benchmark_repeat: 1,
            seed: 1,
        }
    }

    /// В поколении столько бойцов, сколько элита и наследников вместе
    #[test]
    fn population_size_is_elite_plus_offspring() {
        let config = config(8, 56);

        assert_eq!(config.population_size(), 64);
    }

    /// Популяция состоит из элиты и её наследников
    #[test]
    fn population_holds_elite_and_offspring() {
        let config = config(2, 3);
        let club = Club::new(config.clone());

        let population = build_population(&config, &club.elites).unwrap();

        assert_eq!(population.len(), 5);
        // Первые два — сама элита, без изменений
        assert_eq!(population[0], club.elites[0].net);
        assert_eq!(population[1], club.elites[1].net);
    }

    /// После поколения элита остаётся нужного размера
    #[test]
    fn generation_keeps_elite_size() {
        let mut club = Club::new(config(2, 2));

        run_generation(&mut club, 1).unwrap();

        assert_eq!(club.elites.len(), 2);
        assert_eq!(club.history.len(), 0, "история пополняется отдельно");
    }

    /// Лучший боец стоит раньше худшего
    #[test]
    fn better_fighter_comes_first() {
        let mut result = ArenaResult {
            scores: vec![1000, 1010, 990],
            stats: vec![FighterStats::default(); 3],
            ..Default::default()
        };

        result.stats[1].wins = 5;
        result.stats[0].wins = 3;
        result.stats[2].wins = 1;

        let mut order: Vec<usize> = (0..3).collect();
        order.sort_by(|a, b| compare(&result, *a, *b));

        assert_eq!(order, vec![1, 0, 2]);
    }

    /// При равных очках решают победы, а при равных победах - нокауты от удара
    #[test]
    fn equal_scores_are_decided_by_wins() {
        let mut result = ArenaResult {
            scores: vec![1000, 1000],
            stats: vec![FighterStats::default(); 2],
            ..Default::default()
        };

        result.stats[0].wins = 4;
        result.stats[1].wins = 2;
        result.stats[0].wins_by_defeat = 0;
        result.stats[1].wins_by_defeat = 1;

        let mut order: Vec<usize> = (0..2).collect();
        order.sort_by(|a, b| compare(&result, *a, *b));

        assert_eq!(order, vec![0, 1], "победы важнее нокаутов от удара");
    }

    /// Клуб переживает сохранение и чтение
    #[test]
    fn club_survives_save_and_load() {
        let dir = std::env::temp_dir().join("fencing_eugenics_club_test");
        let _ = std::fs::remove_dir_all(&dir);

        let mut club = Club::new(config(2, 2));
        run_generation(&mut club, 1).unwrap();

        save_generation(&dir, &club).unwrap();

        let (generation, path) = find_latest_backup(&dir).unwrap();
        let loaded: Club = load_json(&path).unwrap();

        assert_eq!(generation, 1);
        assert_eq!(loaded.elites.len(), 2);
        assert_eq!(loaded.elites[0].net, club.elites[0].net);
        assert_eq!(loaded.config.club, "club_test");

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Файлы мозгов элиты читаются загрузчиком
    #[test]
    fn elite_brain_file_is_loadable() {
        let dir = std::env::temp_dir().join("fencing_eugenics_brain_test");
        let _ = std::fs::remove_dir_all(&dir);

        let mut club = Club::new(config(2, 2));
        run_generation(&mut club, 1).unwrap();
        save_generation(&dir, &club).unwrap();

        let path = epoch_dir(&dir, 1).join(format!("{}.json", club.elites[0].name));
        let saved = Perceptron::load_brain(&path).unwrap();

        assert_eq!(saved.net, club.elites[0].net, "мозг элиты другой");

        // Файл читается и загрузчиком боя
        let _brain = crate::fighter::brain::loader::load_brain(&path).unwrap();

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Аргументы командной строки превращаются в настройки
    #[test]
    fn config_comes_from_args() {
        let args = TrainEugenicsArgs {
            club: "club_eugenics".to_string(),
            elite: 8,
            offspring: 56,
            generations: 3,
            repeat: 1,
            hidden: 16,
            mutation_rate: 0.1,
            mutation_scale: 0.2,
            crossover_rate: 0.5,
            tournament: 3,
            scoring: Scoring::Classic,
            benchmark_repeat: 3,
            seed: 7,
            out: "club_eugenics".to_string(),
        };

        let config = GaConfig::from_args(&args);

        assert_eq!(config.population_size(), 64);
        assert_eq!(config.seed, 7);
    }
}
