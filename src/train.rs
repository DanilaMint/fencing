//! Обучение клуба бойцов.
//!
//! Клуб состоит из бойцов-стратегов: каждый спарит с каждым, и после каждого
//! боя его таблица реакций становится чуть лучше (или стирается, если реакция
//! оказалась провальной). За эпоху каждая пара встречается много раз, причём
//! спаринги разных пар перемешаны: так бойцы не привыкают к одному
//! противнику.
//!
//! Спарсинги идут параллельно, но в пределах одной волны боец участвует не
//! более чем в одном бою — иначе состояния двух реакций перемешались бы в
//! общем стратеге.
//!
//! После каждой эпохи состояние клуба сохраняется: общий бэкап и отдельный
//! файл с таблицей каждого бойца. В конце пишется отчёт обучения.

use std::{
    cell::RefCell,
    cmp::Ordering,
    fs, io,
    path::{Path, PathBuf},
    rc::Rc,
    sync::{Arc, Mutex},
    time::Instant,
};

use fastrand::Rng;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

use crate::{
    cli::TrainStrategistArgs,
    club::{data_error, find_latest_backup, write_json},
    fighter::{
        action::{Action, Direction, SwordAction},
        brain::{
            Brain, BrainEvent,
            strategist::{
                FORMAT_VERSION, MODEL_NAME, Strategist,
                reaction::{MoveDir8, Reaction},
                stimulus::Stimulus,
                table::{Entry, Table},
            },
        },
    },
    math::DT,
    round::{LoseReason, Round, RoundResult},
};

/// Имя клуба по умолчанию
pub const DEFAULT_CLUB: &str = "club_firsts";
/// Число бойцов в клубе по умолчанию
pub const DEFAULT_FIGHTERS: usize = 16;
/// Число спарингов на пару за эпоху по умолчанию
pub const DEFAULT_SPARINGS: u32 = 100000;
/// Число эпох обучения по умолчанию
pub const DEFAULT_EPOCHS: u32 = 8;
/// Базовое зерно обучения по умолчанию
pub const DEFAULT_SEED: u64 = 20240101;
/// Имя файла отчёта обучения
pub const REPORT_NAME: &str = "report.md";

// ----- Клуб -----

/// Боец клуба
struct ClubFighter {
    /// Имя бойца для отчётов
    name: String,
    /// Зерно генератора стратега
    seed: u64,
    /// Стратег: общий для всех спарингов этого бойца
    brain: Arc<Mutex<Strategist>>,
}

/// Клуб бойцов
struct Club {
    /// Имя клуба
    name: String,
    /// Базовое зерно обучения
    seed: u64,
    /// Бойцы клуба
    fighters: Vec<ClubFighter>,
    /// Итоги бойцов за все пройденные эпохи
    stats: Vec<FighterStats>,
    /// Сводки по пройденным эпохам
    epoch_stats: Vec<EpochReport>,
}

// ----- Счётчики -----

/// Счётчики событий одного боя
#[derive(Default, Clone, Copy, Debug)]
struct FightStats {
    /// Нанесённых ударов
    hits_dealt: u32,
    /// Полученных ударов
    hits_taken: u32,
    /// Отбитых ударов
    parries: u32,
    /// Полученных шоков
    shocks: u32,
    /// Потрачено энергии
    energy_spent: f32,
}

impl FightStats {
    /// Учитывает событие боя
    fn add(&mut self, event: BrainEvent) {
        match event {
            BrainEvent::HitDealt { .. } => self.hits_dealt += 1,
            BrainEvent::HitTaken { .. } => self.hits_taken += 1,
            BrainEvent::Parried => self.parries += 1,
            BrainEvent::Shocked => self.shocks += 1,
            BrainEvent::EnergySpent { amount } => self.energy_spent += amount,
        }
    }
}

/// Итоги бойца за все эпохи
#[derive(Default, Clone, Debug, Serialize, Deserialize)]
struct FighterStats {
    /// Сыграно боёв
    fights: u32,
    /// Побед
    wins: u32,
    /// Ничьих
    draws: u32,
    /// Поражений
    losses: u32,
    /// Побед нокаутом
    wins_by_ko: u32,
    /// Поражений выходом из кольца
    losses_by_ring: u32,
    /// Нанесённых ударов
    hits_dealt: u64,
    /// Полученных ударов
    hits_taken: u64,
    /// Отбитых ударов
    parries: u64,
    /// Полученных шоков
    shocks: u64,
    /// Потрачено энергии
    energy_spent: f64,
    /// Всего тиков в боях
    ticks: u64,
}

/// Исход боя с точки зрения его длины
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum EndKind {
    /// Боец отправлен в нокаут
    Knockout,
    /// Боец вышел за пределы кольца
    OutOfRing,
    /// Время вышло
    ByTime,
}

/// Сводка по эпохе
#[derive(Default, Clone, Debug, Serialize, Deserialize)]
struct EpochReport {
    /// Номер эпохи
    epoch: u32,
    /// Число спарингов
    fights: u32,
    /// Время эпохи, секунды
    seconds: f32,
    /// Средняя длина боя, тиков
    avg_ticks: f32,
    /// Боёв, закончившихся нокаутом
    knockouts: u32,
    /// Боёв, закончившихся выходом из кольца
    out_of_ring: u32,
    /// Боёв, закончившихся по времени
    by_time: u32,
    /// Всего тиков в боях эпохи
    total_ticks: u64,
    /// Нанесённых ударов
    hits_dealt: u64,
    /// Полученных ударов
    hits_taken: u64,
    /// Отбитых ударов
    parries: u64,
    /// Полученных шоков
    shocks: u64,
    /// Средний рейтинг записей во всех таблицах
    avg_rating: f32,
    /// Занятых записей в таблицах
    filled: usize,
    /// Всего записей в таблицах
    capacity: usize,
}

// ----- Мозг для потоков rayon -----

/// Мозг боя, работающий с общим стратегом.
///
/// Стратег один на весь клуб, а мозг нужен в каждом бою, поэтому состояние
/// стратега закрыто мьютексом, а счётчики событий — локальные для боя.
struct SharedBrain {
    /// Стратег, общий для всех спарингов этого бойца
    brain: Arc<Mutex<Strategist>>,
    /// Счётчики событий текущего боя
    stats: Rc<RefCell<FightStats>>,
}

impl Brain for SharedBrain {
    fn get_actions(&mut self, situation: crate::round::situation::Situation) -> Action {
        // Отравленный мьютекс не должен ронять весь бой
        self.brain
            .lock()
            .unwrap_or_else(|err| err.into_inner())
            .get_actions(situation)
    }

    fn on_event(&mut self, event: BrainEvent) {
        self.stats.borrow_mut().add(event);

        self.brain
            .lock()
            .unwrap_or_else(|err| err.into_inner())
            .on_event(event);
    }
}

// ----- Задачи эпохи -----

/// Задача эпохи: один спаринг
#[derive(Clone, Copy, Debug)]
struct SparringTask {
    /// Индекс первого бойца
    red: usize,
    /// Индекс второго бойца
    blue: usize,
    /// Номер пары в общем списке пар
    pair_id: u32,
    /// Номер повтора этой пары внутри эпохи
    repetition: u32,
}

/// Итог одного спаринга
struct SparringOutcome {
    /// Индекс победителя (или первого бойца при ничьей)
    red: usize,
    /// Индекс второго бойца
    blue: usize,
    /// Результат боя
    result: RoundResult,
    /// Причина поражения красного
    red_lose: Option<LoseReason>,
    /// Причина поражения синего
    blue_lose: Option<LoseReason>,
    /// Длительность боя, тиков
    ticks: u32,
    /// События красного
    red_stats: FightStats,
    /// События синего
    blue_stats: FightStats,
}

impl SparringOutcome {
    /// Чем закончился бой
    fn end_kind(&self) -> EndKind {
        let reasons = [self.red_lose, self.blue_lose];

        // Нокаут от удара и нокаут от перерасхода энергии — один исход
        if reasons.contains(&Some(LoseReason::Exhausted))
            || reasons.contains(&Some(LoseReason::Defeated))
        {
            EndKind::Knockout
        } else if reasons.contains(&Some(LoseReason::OutOfRing)) {
            EndKind::OutOfRing
        } else {
            EndKind::ByTime
        }
    }
}

/// Все спаринги эпохи: каждая пара встречается заданное число раз.
///
/// Список перемешивается целиком, поэтому сто спарингов одной пары не идут
/// подряд: бойцы видят противников в смешанном порядке.
fn build_tasks(fighters: usize, sparrings_per_pair: u32, rng: &mut Rng) -> Vec<SparringTask> {
    let pairs = fighters * fighters.saturating_sub(1) / 2;
    let mut tasks = Vec::with_capacity(pairs * sparrings_per_pair as usize);

    let mut pair_id = 0u32;
    for red in 0..fighters {
        for blue in red + 1..fighters {
            for repetition in 0..sparrings_per_pair {
                tasks.push(SparringTask {
                    red,
                    blue,
                    pair_id,
                    repetition,
                });
            }
            pair_id += 1;
        }
    }

    rng.shuffle(&mut tasks);

    tasks
}

/// Разбивает задачи на волны.
///
/// В одной волне каждый боец участвует не более чем в одном спаринге: иначе
/// два боя одного бойца перемешали бы состояние его реакции.
fn build_waves(tasks: &[SparringTask]) -> Vec<Vec<SparringTask>> {
    let mut waves: Vec<Vec<SparringTask>> = Vec::new();
    // Битовая маска занятых в текущей волне бойцов
    let mut busy: u64 = 0;

    for task in tasks {
        let mask = (1u64 << task.red) | (1u64 << task.blue);

        if waves.is_empty() || busy & mask != 0 {
            // В текущей волне места нет — открываем следующую
            waves.push(Vec::new());
            busy = 0;
        }

        waves
            .last_mut()
            .expect("волна только что создана")
            .push(*task);
        busy |= mask;
    }

    debug_assert!(waves.iter().all(|wave| {
        let mut seen = 0u64;
        wave.iter().all(|task| {
            let mask = (1u64 << task.red) | (1u64 << task.blue);
            let free = seen & mask == 0;
            seen |= mask;
            free
        })
    }));

    waves
}

// ----- Спарринг -----

/// Один спаринг: два стратега дерутся по правилам игры
fn run_sparring(
    task: &SparringTask,
    brains: &[Arc<Mutex<Strategist>>],
    seed: u64,
    epoch: u32,
) -> SparringOutcome {
    // Своё зерно для расстановки: под rayon иначе спаринги неповторимы
    fastrand::seed(derive_seed(seed, epoch, task.pair_id, task.repetition));

    let red_stats = Rc::new(RefCell::new(FightStats::default()));
    let blue_stats = Rc::new(RefCell::new(FightStats::default()));

    // Стороны меняются местами: никто не играет «всегда слева»
    let (red, blue) = if fastrand::bool() {
        (task.blue, task.red)
    } else {
        (task.red, task.blue)
    };

    // Реакция предыдущего боя не должна продолжаться в этом:
    // её результат уходит в рейтинг, а новый бой начинается с чистого листа
    for index in [red, blue] {
        brains[index]
            .lock()
            .unwrap_or_else(|err| err.into_inner())
            .end_reaction();
    }

    let mut round = Round::new(
        Box::new(SharedBrain {
            brain: brains[red].clone(),
            stats: red_stats.clone(),
        }),
        Box::new(SharedBrain {
            brain: brains[blue].clone(),
            stats: blue_stats.clone(),
        }),
    );

    while !round.is_finished() {
        round.tick();
    }

    SparringOutcome {
        red,
        blue,
        result: round.result().unwrap_or(RoundResult::Draw),
        red_lose: round.red_lose_reason(),
        blue_lose: round.blue_lose_reason(),
        ticks: round.ticks(),
        red_stats: *red_stats.borrow(),
        blue_stats: *blue_stats.borrow(),
    }
}

// ----- Клуб и эпохи -----

/// Создаёт клуб: у каждого бойца своя таблица и своё зерно
fn build_club(name: &str, fighters: usize, seed: u64) -> Club {
    let club_fighters: Vec<ClubFighter> = (0..fighters)
        .map(|index| {
            let fighter_seed = derive_seed(seed, 0, index as u32, 0);

            ClubFighter {
                name: format!("{name}_{index:02}"),
                seed: fighter_seed,
                brain: Arc::new(Mutex::new(Strategist::with_seed(fighter_seed))),
            }
        })
        .collect();

    Club {
        name: name.to_string(),
        seed,
        stats: vec![FighterStats::default(); fighters],
        epoch_stats: Vec::new(),
        fighters: club_fighters,
    }
}

/// Обучает клуб одну эпоху
fn run_epoch(club: &mut Club, epoch: u32, sparrings_per_pair: u32) -> EpochReport {
    let mut rng = Rng::with_seed(derive_seed(club.seed, epoch, u32::MAX, 0));
    let tasks = build_tasks(club.fighters.len(), sparrings_per_pair, &mut rng);
    let waves = build_waves(&tasks);

    // Копии мьютексов стратегов: потоки rayon работают только с ними
    let brains: Vec<Arc<Mutex<Strategist>>> = club
        .fighters
        .iter()
        .map(|fighter| fighter.brain.clone())
        .collect();
    let base_seed = club.seed;

    let mut report = EpochReport {
        epoch,
        fights: tasks.len() as u32,
        ..Default::default()
    };

    let started = Instant::now();
    let mut done = 0u32;

    for wave in &waves {
        let outcomes: Vec<SparringOutcome> = wave
            .par_iter()
            .map(|task| run_sparring(task, &brains, base_seed, epoch))
            .collect();

        for outcome in &outcomes {
            apply_outcome(club, outcome, &mut report);
        }

        done += wave.len() as u32;

        // Прогресс: печатаем изредка, чтобы не утонуть в выводе
        if done % 1000 < wave.len() as u32 {
            println!(
                "  эпоха {epoch}: {}/{} спарингов, {:.1} с",
                done,
                report.fights,
                started.elapsed().as_secs_f32()
            );
        }
    }

    report.seconds = started.elapsed().as_secs_f32();
    report.avg_ticks = if report.fights > 0 {
        report.total_ticks as f32 / report.fights as f32
    } else {
        0.
    };

    // Состояние таблиц: сколько записей занято и какой у них рейтинг
    let (filled, capacity, rating_sum) = club
        .fighters
        .iter()
        .map(|fighter| {
            let table = fighter
                .brain
                .lock()
                .unwrap_or_else(|err| err.into_inner())
                .table()
                .clone();
            let filled = table.filled();
            let sum: f32 = (0..table.len())
                .filter_map(|index| table.rating(index))
                .sum();

            (filled, table.len(), sum)
        })
        .fold((0, 0, 0.), |acc, (filled, len, sum)| {
            (acc.0 + filled, acc.1 + len, acc.2 + sum)
        });

    report.filled = filled;
    report.capacity = capacity;
    report.avg_rating = if filled > 0 {
        rating_sum / filled as f32
    } else {
        0.
    };

    club.epoch_stats.push(report.clone());

    report
}

/// Две ссылки на статистику разных бойцов
fn stats_pair(
    stats: &mut [FighterStats],
    first: usize,
    second: usize,
) -> (&mut FighterStats, &mut FighterStats) {
    if first < second {
        let (left, right) = stats.split_at_mut(second);
        (&mut left[first], &mut right[0])
    } else {
        let (left, right) = stats.split_at_mut(first);
        (&mut right[0], &mut left[second])
    }
}

/// Учитывает результат боя в статистике клуба и эпохи
fn apply_outcome(club: &mut Club, outcome: &SparringOutcome, report: &mut EpochReport) {
    let (red, blue) = stats_pair(&mut club.stats, outcome.red, outcome.blue);

    red.fights += 1;
    blue.fights += 1;
    red.ticks += outcome.ticks as u64;
    blue.ticks += outcome.ticks as u64;

    red.hits_dealt += outcome.red_stats.hits_dealt as u64;
    red.hits_taken += outcome.red_stats.hits_taken as u64;
    red.parries += outcome.red_stats.parries as u64;
    red.shocks += outcome.red_stats.shocks as u64;
    red.energy_spent += outcome.red_stats.energy_spent as f64;

    blue.hits_dealt += outcome.blue_stats.hits_dealt as u64;
    blue.hits_taken += outcome.blue_stats.hits_taken as u64;
    blue.parries += outcome.blue_stats.parries as u64;
    blue.shocks += outcome.blue_stats.shocks as u64;
    blue.energy_spent += outcome.blue_stats.energy_spent as f64;

    report.hits_dealt += outcome.red_stats.hits_dealt as u64 + outcome.blue_stats.hits_dealt as u64;
    report.hits_taken += outcome.red_stats.hits_taken as u64 + outcome.blue_stats.hits_taken as u64;
    report.parries += outcome.red_stats.parries as u64 + outcome.blue_stats.parries as u64;
    report.shocks += outcome.red_stats.shocks as u64 + outcome.blue_stats.shocks as u64;

    match outcome.end_kind() {
        EndKind::Knockout => report.knockouts += 1,
        EndKind::OutOfRing => report.out_of_ring += 1,
        EndKind::ByTime => report.by_time += 1,
    }

    report.total_ticks += outcome.ticks as u64;

    match outcome.result {
        RoundResult::RedWin => {
            red.wins += 1;
            blue.losses += 1;

            if outcome.blue_lose == Some(LoseReason::Exhausted)
                || outcome.blue_lose == Some(LoseReason::Defeated)
            {
                red.wins_by_ko += 1;
            }
            if outcome.red_lose == Some(LoseReason::OutOfRing) {
                blue.losses_by_ring += 1;
            }
        }
        RoundResult::BlueWin => {
            blue.wins += 1;
            red.losses += 1;

            if outcome.red_lose == Some(LoseReason::Exhausted)
                || outcome.red_lose == Some(LoseReason::Defeated)
            {
                blue.wins_by_ko += 1;
            }
            if outcome.blue_lose == Some(LoseReason::OutOfRing) {
                red.losses_by_ring += 1;
            }
        }
        RoundResult::Draw => {
            red.draws += 1;
            blue.draws += 1;
        }
    }
}

// ----- Сохранение -----

/// Бэкап клуба: общее состояние после эпохи
#[derive(Serialize, Deserialize)]
struct ClubBackup {
    /// Модель мозга, которой принадлежат таблицы
    model: String,
    /// Версия формата
    version: u32,
    /// Имя клуба
    name: String,
    /// Номер последней пройденной эпохи
    epoch: u32,
    /// Базовое зерно обучения
    seed: u64,
    /// Сводки по пройденным эпохам
    epoch_stats: Vec<EpochReport>,
    /// Бойцы клуба
    fighters: Vec<FighterBackup>,
}

/// Состояние одного бойца в бэкапе
#[derive(Serialize, Deserialize)]
struct FighterBackup {
    /// Имя бойца
    name: String,
    /// Зерно генератора
    seed: u64,
    /// Итоги бойца
    stats: FighterStats,
    /// Обученная таблица
    table: Table,
}

/// Сохраняет эпоху: общий бэкап и отдельный файл с таблицей каждого бойца
fn save_epoch(dir: &Path, epoch: u32, club: &Club) -> io::Result<()> {
    let epoch_dir = dir.join(format!("epoch_{epoch:02}"));
    fs::create_dir_all(&epoch_dir)?;

    let backup = ClubBackup {
        model: MODEL_NAME.to_string(),
        version: FORMAT_VERSION,
        name: club.name.clone(),
        epoch,
        seed: club.seed,
        epoch_stats: club.epoch_stats.clone(),
        fighters: club
            .fighters
            .iter()
            .enumerate()
            .map(|(index, fighter)| FighterBackup {
                name: fighter.name.clone(),
                seed: fighter.seed,
                stats: club.stats[index].clone(),
                table: fighter
                    .brain
                    .lock()
                    .unwrap_or_else(|err| err.into_inner())
                    .table()
                    .clone(),
            })
            .collect(),
    };

    write_json(&epoch_dir.join("club.json"), &backup)?;

    // Отдельный файл с таблицей каждого бойца: в нём сразу видно,
    // какой моделью мозга он создан
    for fighter in &club.fighters {
        fighter
            .brain
            .lock()
            .unwrap_or_else(|err| err.into_inner())
            .save_to_file(
                epoch_dir.join(format!("{}.json", fighter.name)),
                &fighter.name,
            )?;
    }

    Ok(())
}

/// Загружает клуб из бэкапа
fn club_from_backup(backup: ClubBackup) -> Club {
    if backup.model != MODEL_NAME {
        // Бэкап чужой модели: сообщаем об этом при загрузке, а не молча
        eprintln!(
            "Внимание: бэкап сохранён для модели мозга \"{}\", а загружен будет \"{}\"",
            backup.model, MODEL_NAME
        );
    }

    let fighters: Vec<ClubFighter> = backup
        .fighters
        .iter()
        .map(|fighter| ClubFighter {
            name: fighter.name.clone(),
            seed: fighter.seed,
            brain: Arc::new(Mutex::new(Strategist::with_seed_and_table(
                fighter.seed,
                fighter.table.clone(),
            ))),
        })
        .collect();

    let stats = backup
        .fighters
        .iter()
        .map(|fighter| fighter.stats.clone())
        .collect();

    Club {
        name: backup.name,
        seed: backup.seed,
        fighters,
        stats,
        epoch_stats: backup.epoch_stats,
    }
}

// ----- Сиды -----

/// Перемешиватель битов: детерминированный и без зависимости от std
fn splitmix64(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = x;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);

    z ^ (z >> 31)
}

/// Сид спаринга: зависит от базы, эпохи, пары и номера повтора
fn derive_seed(base: u64, epoch: u32, pair_id: u32, repetition: u32) -> u64 {
    splitmix64(
        base ^ (epoch as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
            ^ (pair_id as u64).wrapping_mul(0xBF58_476D_1CE4_E5B9)
            ^ (repetition as u64).wrapping_mul(0x94D0_49BB_1331_11EB),
    )
}

// ----- Отчёт -----

/// Описание стимула словами
fn describe_stimulus(stimulus: &Stimulus) -> String {
    format!(
        "противник на {:.0}% радиуса, отклонение {:.0}° против часовой стрелки от нашего взгляда; \
         он видит нас под {:.0}° против часовой стрелки; мы на {:.0}% радиуса от центра, \
         взгляд отклонён от центра на {:.0}°",
        stimulus.enemy_distance * 100.,
        stimulus.to_enemy_angle.abs() * 180.,
        stimulus.from_enemy_angle.abs() * 180.,
        stimulus.center_distance * 100.,
        stimulus.facing_delta.abs() * 180.,
    )
}

/// Описание направления движения словами
fn describe_move(dir: MoveDir8) -> &'static str {
    match dir {
        MoveDir8::Forward => "вперёд",
        MoveDir8::Back => "назад",
        MoveDir8::Left => "влево",
        MoveDir8::Right => "вправо",
        MoveDir8::ForwardLeft => "вперёд-влево",
        MoveDir8::ForwardRight => "вперёд-вправо",
        MoveDir8::BackLeft => "назад-влево",
        MoveDir8::BackRight => "назад-вправо",
    }
}

/// Описание реакции словами: что боец делает и в каком порядке
fn describe_reaction(reaction: &Reaction) -> String {
    let mut stages: Vec<String> = Vec::new();

    for (index, stage) in reaction.stages.iter().enumerate() {
        let mut parts: Vec<String> = Vec::new();

        if let Some((dir, ticks)) = stage.move_command {
            parts.push(format!("{} {:.2} с", describe_move(dir), ticks as f32 * DT));
        }
        if let Some((dir, ticks)) = stage.turn_command {
            let side = if dir == Direction::Negative {
                "вправо"
            } else {
                "влево"
            };
            parts.push(format!("поворот {side} {:.2} с", ticks as f32 * DT));
        }
        if let Some(strike) = stage.strike {
            let kind = if strike == SwordAction::Prick {
                "колющий"
            } else {
                "рубящий"
            };
            parts.push(format!("{kind} удар"));
        }

        if !parts.is_empty() {
            stages.push(format!("этап {}: {}", index + 1, parts.join(", ")));
        }
    }

    if stages.is_empty() {
        "ничего не делает".to_string()
    } else {
        stages.join("; ")
    }
}

/// Лучшие записи таблицы бойца
fn best_entries(table: &Table, count: usize) -> Vec<&Entry> {
    let mut entries: Vec<&Entry> = (0..table.len())
        .filter_map(|index| table.entry(index))
        .collect();

    entries.sort_by(|a, b| b.rating.partial_cmp(&a.rating).unwrap_or(Ordering::Equal));

    entries.truncate(count);
    entries
}

/// Пишет отчёт обучения
fn write_report(dir: &Path, club: &Club) -> io::Result<()> {
    let mut text = String::new();

    // Каждый бой учтён у обоих участников, поэтому считаем пополам
    let total_fights: u32 = club.stats.iter().map(|stats| stats.fights).sum::<u32>() / 2;
    let total_time: f32 = club.epoch_stats.iter().map(|report| report.seconds).sum();

    text.push_str(&format!("# Отчёт обучения клуба «{}»\n\n", club.name));
    text.push_str(&format!(
        "- бойцов: {}\n- эпох: {}\n- спарингов всего: {}\n- базовое зерно: {}\n- суммарное время: {:.1} с\n\n",
        club.fighters.len(),
        club.epoch_stats.len(),
        total_fights,
        club.seed,
        total_time
    ));

    // Динамика по эпохам
    text.push_str("## Динамика по эпохам\n\n");
    text.push_str(
        "| Эпоха | Спарингов | Время, с | Средний бой, тиков | Нокаут | Из кольца | По времени \
         | Удар+ | Удар- | Парирования | Шоки | Записей | Ср. рейтинг |\n",
    );
    text.push_str("|---|---|---|---|---|---|---|---|---|---|---|---|---|\n");

    for report in &club.epoch_stats {
        text.push_str(&format!(
            "| {} | {} | {:.1} | {:.0} | {} | {} | {} | {} | {} | {} | {} | {}/{} | {:.2} |\n",
            report.epoch,
            report.fights,
            report.seconds,
            report.avg_ticks,
            report.knockouts,
            report.out_of_ring,
            report.by_time,
            report.hits_dealt,
            report.hits_taken,
            report.parries,
            report.shocks,
            report.filled,
            report.capacity,
            report.avg_rating
        ));
    }
    text.push('\n');

    // Лидерборд
    let mut order: Vec<usize> = (0..club.fighters.len()).collect();
    order.sort_by(|a, b| club.stats[*b].wins.cmp(&club.stats[*a].wins));

    text.push_str("## Итоговый лидерборд\n\n");
    text.push_str(
        "| Боец | Бои | Победы | Ничьи | Поражения | Нокаутом | Удар+ | Удар- | Парирования \
         | Шоки | Ср. рейтинг | Записей |\n",
    );
    text.push_str("|---|---|---|---|---|---|---|---|---|---|---|---|\n");

    for index in &order {
        let stats = &club.stats[*index];
        let table = club.fighters[*index]
            .brain
            .lock()
            .unwrap_or_else(|err| err.into_inner())
            .table()
            .clone();
        let avg = table_average(&table);

        text.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {:.2} | {}/{} |\n",
            club.fighters[*index].name,
            stats.fights,
            stats.wins,
            stats.draws,
            stats.losses,
            stats.wins_by_ko,
            stats.hits_dealt,
            stats.hits_taken,
            stats.parries,
            stats.shocks,
            avg,
            table.filled(),
            table.len()
        ));
    }
    text.push('\n');

    // Чему научились бойцы
    text.push_str("## Чему научились бойцы\n\n");

    for fighter in &club.fighters {
        let table = fighter
            .brain
            .lock()
            .unwrap_or_else(|err| err.into_inner())
            .table()
            .clone();

        text.push_str(&format!("### {}\n\n", fighter.name));

        if table.filled() == 0 {
            text.push_str("Таблица пуста: ни одна реакция не удержалась.\n\n");
            continue;
        }

        for entry in best_entries(&table, 3) {
            text.push_str(&format!(
                "- **рейтинг {:.2}**\n  - ситуация: {}\n  - реакция: {}\n",
                entry.rating,
                describe_stimulus(&entry.stimulus),
                describe_reaction(&entry.reaction),
            ));
        }
        text.push('\n');
    }

    // Выводы
    text.push_str("## Выводы\n\n");

    let first = club.epoch_stats.first();
    let last = club.epoch_stats.last();

    if let (Some(first), Some(last)) = (first, last) {
        text.push_str(&format!(
            "- средний рейтинг записей: {:.2} в начале, {:.2} в конце\n",
            first.avg_rating, last.avg_rating
        ));
        text.push_str(&format!(
            "- ударов на бой: {:.2} в начале, {:.2} в конце\n",
            hits_per_fight(first),
            hits_per_fight(last)
        ));
        text.push_str(&format!(
            "- средняя длина боя: {:.0} тиков в начале, {:.0} в конце\n",
            first.avg_ticks, last.avg_ticks
        ));

        // Как изменилось поведение бойцов
        let hits_delta = hits_per_fight(last) - hits_per_fight(first);
        if hits_delta < -0.05 {
            text.push_str(
                "- бойцы стали осторожнее: ударов на бой стало меньше. За проигрыш \
                 (пропущенный удар) снимается больше очков, чем за удачный удар\n",
            );
        } else if hits_delta > 0.05 {
            text.push_str("- бойцы стали активнее: ударов на бой стало больше\n");
        } else {
            text.push_str("- число ударов на бой почти не изменилось\n");
        }
    }

    let empty: Vec<&str> = club
        .fighters
        .iter()
        .enumerate()
        .filter(|(index, _)| {
            club.stats[*index].hits_dealt == 0 && club.stats[*index].hits_taken == 0
        })
        .map(|(_, fighter)| fighter.name.as_str())
        .collect();

    if empty.is_empty() {
        text.push_str("- все бойцы участвовали в ударах: сигнал для обучения есть\n");
    } else {
        text.push_str(&format!("- без ударов остались: {}\n", empty.join(", ")));
    }

    let degenerate: Vec<&str> = club
        .fighters
        .iter()
        .enumerate()
        .filter(|(index, _)| {
            let table = club.fighters[*index]
                .brain
                .lock()
                .unwrap_or_else(|err| err.into_inner())
                .table()
                .clone();
            table.filled() <= 1
        })
        .map(|(_, fighter)| fighter.name.as_str())
        .collect();

    if !degenerate.is_empty() {
        text.push_str(&format!(
            "- таблица выродилась почти в пустую у: {}\n",
            degenerate.join(", ")
        ));
    }

    fs::write(dir.join(REPORT_NAME), text)
}

/// Средний рейтинг записей таблицы
fn table_average(table: &Table) -> f32 {
    let mut sum = 0.;
    let mut count = 0;

    for index in 0..table.len() {
        if let Some(rating) = table.rating(index) {
            sum += rating;
            count += 1;
        }
    }

    if count > 0 { sum / count as f32 } else { 0. }
}

/// Ударов на бой по сводке эпохи
fn hits_per_fight(report: &EpochReport) -> f32 {
    if report.fights > 0 {
        (report.hits_dealt + report.hits_taken) as f32 / report.fights as f32
    } else {
        0.
    }
}

// ----- Запуск -----

/// Запускает обучение клуба бойцов
pub fn run(args: &TrainStrategistArgs) -> io::Result<()> {
    // Каталог клуба: внутри — папки эпох, бэкапы и отчёт
    let dir = PathBuf::from(&args.out);
    fs::create_dir_all(&dir)?;

    // Возобновление: если последняя эпоха уже сохранена, продолжаем с неё
    let (mut club, mut epoch) = match find_latest_backup(&dir) {
        Some((done, path)) => {
            println!("Продолжаем обучение с {}", path.display());
            (club_from_backup(load_backup(&path)?), done + 1)
        }
        None => (build_club(&args.club, args.fighters, args.seed), 1),
    };

    while epoch <= args.epochs {
        println!(
            "===== Клуб \"{}\" | эпоха {}/{} =====",
            club.name, epoch, args.epochs
        );

        let report = run_epoch(&mut club, epoch, args.sparring);
        print_epoch(&club, &report, args.epochs);

        save_epoch(&dir, epoch, &club)?;

        epoch += 1;
    }

    write_report(&dir, &club)?;

    println!("\nОтчёт обучения: {}", dir.join(REPORT_NAME).display());

    Ok(())
}

/// Загружает бэкап клуба
fn load_backup(path: &Path) -> io::Result<ClubBackup> {
    let text = fs::read_to_string(path)?;

    serde_json::from_str(&text).map_err(data_error)
}

/// Печатает сводку по эпохе
fn print_epoch(club: &Club, report: &EpochReport, total_epochs: u32) {
    println!(
        "Спарингов: {}   время: {:.1} с   средняя длина боя: {:.0} тиков",
        report.fights, report.seconds, report.avg_ticks
    );
    println!(
        "Исходы: нокаут {} | выход из кольца {} | по времени {}",
        report.knockouts, report.out_of_ring, report.by_time
    );
    println!(
        "Ударов на бой: {:.2} нанесено / {:.2} получено | парирований {:.2} | шоков {:.2}",
        report.hits_dealt as f32 / report.fights.max(1) as f32,
        report.hits_taken as f32 / report.fights.max(1) as f32,
        report.parries as f32 / report.fights.max(1) as f32,
        report.shocks as f32 / report.fights.max(1) as f32,
    );

    println!(
        "\n{:<10} {:>5} {:>5} {:>5} {:>6} {:>6} {:>6} {:>5} {:>8}",
        "Боец", "Поб", "Нич", "Праж", "Удар+", "Удар-", "Пар", "Шок", "Ср.рейт"
    );

    let mut order: Vec<usize> = (0..club.fighters.len()).collect();
    order.sort_by(|a, b| club.stats[*b].wins.cmp(&club.stats[*a].wins));

    for index in &order {
        let stats = &club.stats[*index];
        let table = club.fighters[*index]
            .brain
            .lock()
            .unwrap_or_else(|err| err.into_inner())
            .table()
            .clone();

        println!(
            "{:<10} {:>5} {:>5} {:>5} {:>6} {:>6} {:>6} {:>5} {:>8}",
            club.fighters[*index].name,
            stats.wins,
            stats.draws,
            stats.losses,
            stats.hits_dealt,
            stats.hits_taken,
            stats.parries,
            stats.shocks,
            table_average(&table),
        );
    }

    println!(
        "\nЭпоха {}/{} закончена. Бэкап сохранён в каталоге epoch_{:02}\n",
        report.epoch, total_epochs, report.epoch
    );
}
