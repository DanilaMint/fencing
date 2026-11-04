//! Арена: бойцы одного поколения дерутся между собой.
//!
//! Каждый боец проводит бой с каждым: у поколения из 64 бойцов это 2016 боёв.
//! Бой идёт в отдельном раунде, мозги на каждый бой создаются заново, поэтому
//! состояние боя не протекает в следующий.
//!
//! Бойи идут параллельно: правила не зависят от порядка, поэтому потоки не
//! влияют на результат.

use std::{
    cell::Cell,
    rc::Rc,
};

use rayon::prelude::*;
use serde::{Deserialize, Serialize};

use fencing_core::{
    brain::{Brain, BrainEvent},
    fighter::action::Action,
    round::{LoseReason, Round, RoundResult, situation::Situation},
};
use fencing_perceptron::{Perceptron, net::Net};

use super::scoring::{BASE_SCORE, Scoring, points_for};

/// Счётчики бойца за поколение
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct FighterStats {
    /// Сыграно боёв
    pub fights: u32,
    /// Побед
    pub wins: u32,
    /// Ничьих
    pub draws: u32,
    /// Поражений
    pub losses: u32,
    /// Побед нокаутом от удара
    pub wins_by_defeat: u32,
    /// Побед нокаутом от перерасхода энергии
    pub wins_by_exhaustion: u32,
    /// Побед выходом противника за ринг
    pub wins_by_ring: u32,
    /// Поражений нокаутом от удара
    pub losses_by_defeat: u32,
    /// Поражений нокаутом от перерасхода энергии
    pub losses_by_exhaustion: u32,
    /// Поражений выходом за ринг
    pub losses_by_ring: u32,
    /// Нанесённых ударов
    pub hits_dealt: u64,
    /// Полученных ударов
    pub hits_taken: u64,
    /// Всего тиков в боях
    pub ticks: u64,
}

/// Пара бойцов: они встречаются в бою
#[derive(Clone, Copy, Debug)]
pub struct Pair {
    /// Индекс первого бойца
    pub first: usize,
    /// Индекс второго бойца
    pub second: usize,
}

/// Итог одного боя
#[derive(Clone, Copy, Debug)]
pub struct FightOutcome {
    /// Индекс красного бойца
    pub red: usize,
    /// Индекс синего бойца
    pub blue: usize,
    /// Результат боя
    pub result: RoundResult,
    /// Причина поражения красного
    pub red_lose: Option<LoseReason>,
    /// Причина поражения синего
    pub blue_lose: Option<LoseReason>,
    /// Длительность боя, тиков
    pub ticks: u32,
    /// Удары, нанесённые красным
    pub red_hits_dealt: u32,
    /// Удары, нанесённые синим
    pub blue_hits_dealt: u32,
}

/// Итоги арены: очки и статистика всех бойцов поколения
#[derive(Clone, Debug, Default)]
pub struct ArenaResult {
    /// Очки бойцов: по индексу в популяции
    pub scores: Vec<i32>,
    /// Статистика бойцов: по индексу в популяции
    pub stats: Vec<FighterStats>,
    /// Всего боёв
    pub fights: u32,
    /// Боёв, закончившихся ничьёй
    pub draws: u32,
    /// Боёв, закончившихся выходом за ринг
    pub out_of_ring: u32,
    /// Боёв, закончившихся нокаутом от перерасхода энергии
    pub exhausted: u32,
    /// Боёв, закончившихся нокаутом от удара
    pub defeated: u32,
    /// Всего тиков во всех боях
    pub total_ticks: u64,
}

/// Мозг боя, считающий удары
///
/// Перцептрон события боя не разбирает, а в отчёте интересно, сколько ударов
/// нанесено: считаем их тут. Счётчик общий с тем, кто ждёт конца боя.
struct CountingBrain {
    /// Мозг бойца
    brain: Perceptron,
    /// Нанесённых ударов
    hits_dealt: Rc<Cell<u32>>,
}

impl CountingBrain {
    /// Мозг боя по заданному геному
    fn new(net: Net, hits_dealt: Rc<Cell<u32>>) -> Self {
        Self {
            brain: Perceptron::new(net, fastrand::u64(..)),
            hits_dealt,
        }
    }
}

impl Brain for CountingBrain {
    fn get_actions(&mut self, situation: Situation) -> Action {
        self.brain.get_actions(situation)
    }

    fn on_event(&mut self, event: BrainEvent) {
        if matches!(event, BrainEvent::HitDealt { .. }) {
            self.hits_dealt.set(self.hits_dealt.get() + 1);
        }

        self.brain.on_event(event);
    }

    fn on_round_end(&mut self) {
        self.brain.on_round_end();
    }
}

/// Все пары бойцов: каждый с каждым ровно один раз
pub fn build_pairs(count: usize) -> Vec<Pair> {
    let mut pairs = Vec::with_capacity(count * count.saturating_sub(1) / 2);

    for first in 0..count {
        for second in first + 1..count {
            pairs.push(Pair { first, second });
        }
    }

    pairs
}

/// Проводит все бои поколения
///
/// `population` — геномы бойцов, `repeat` — число боёв на пару.
pub fn run_arena(population: &[Net], repeat: u32, scoring: Scoring) -> ArenaResult {
    let pairs = build_pairs(population.len());

    // Задачи боёв: каждая пара встречается заданное число раз
    let tasks: Vec<Pair> = pairs
        .iter()
        .flat_map(|pair| std::iter::repeat_n(*pair, repeat.max(1) as usize))
        .collect();

    let outcomes: Vec<FightOutcome> = tasks
        .par_iter()
        .map(|pair| {
            run_fight(
                pair,
                &population[pair.first],
                &population[pair.second],
            )
        })
        .collect();

    aggregate(&outcomes, population.len(), scoring)
}

/// Один бой: два бойца с заданными геномами
fn run_fight(pair: &Pair, first: &Net, second: &Net) -> FightOutcome {
    let fight = fight_nets(first, second);

    // Стороны в бою выбираются случайно, поэтому индексы восстанавливаем
    let (red, blue) = if fight.first_is_red {
        (pair.first, pair.second)
    } else {
        (pair.second, pair.first)
    };

    FightOutcome {
        red,
        blue,
        result: fight.result,
        red_lose: fight.red_lose,
        blue_lose: fight.blue_lose,
        ticks: fight.ticks,
        red_hits_dealt: fight.red_hits_dealt,
        blue_hits_dealt: fight.blue_hits_dealt,
    }
}

/// Бой двух геномов
///
/// Первый боец не обязательно красный: стороны меняются местами, чтобы
/// никто не играл «всегда слева».
struct NetFight {
    /// Результат боя
    result: RoundResult,
    /// Причина поражения красного
    red_lose: Option<LoseReason>,
    /// Причина поражения синего
    blue_lose: Option<LoseReason>,
    /// Длительность боя, тиков
    ticks: u32,
    /// Удары красного
    red_hits_dealt: u32,
    /// Удары синего
    blue_hits_dealt: u32,
    /// Первый боец был красным
    first_is_red: bool,
}

/// Бой двух геномов
fn fight_nets(first: &Net, second: &Net) -> NetFight {
    // Стороны меняются местами: никто не играет «всегда слева»
    let first_is_red = !fastrand::bool();

    let (red_net, blue_net) = if first_is_red {
        (first, second)
    } else {
        (second, first)
    };

    let red_hits = Rc::new(Cell::new(0));
    let blue_hits = Rc::new(Cell::new(0));

    let mut round = Round::new(
        Box::new(CountingBrain::new(red_net.clone(), red_hits.clone())),
        Box::new(CountingBrain::new(blue_net.clone(), blue_hits.clone())),
    );

    while !round.is_finished() {
        round.tick();
    }

    NetFight {
        result: round.result().unwrap_or(RoundResult::Draw),
        red_lose: round.red_lose_reason(),
        blue_lose: round.blue_lose_reason(),
        ticks: round.ticks(),
        red_hits_dealt: red_hits.get(),
        blue_hits_dealt: blue_hits.get(),
        first_is_red,
    }
}

/// Проверка силы: бойцы против опорной панели
///
/// Опорная панель — замороженная элита первого поколения. Бойцы текущего
/// поколения дерутся с ней по очереди: так видно, насколько поколение
/// сильнее самого первого.
#[derive(Clone, Debug, Default)]
pub struct Benchmark {
    /// Всего боёв
    pub fights: u32,
    /// Побед проверяемых бойцов
    pub wins: u32,
    /// Ничьих
    pub draws: u32,
    /// Поражений проверяемых бойцов
    pub losses: u32,
    /// Очки, набранные проверяемыми бойцами
    pub points: i32,
}

impl Benchmark {
    /// Доля побед среди всех боёв
    pub fn win_rate(&self) -> f32 {
        if self.fights == 0 {
            return 0.;
        }

        self.wins as f32 / self.fights as f32
    }
}

/// Проверяет силу бойцов против опорной панели
pub fn benchmark(elite: &[Net], reference: &[Net], repeat: u32, scoring: Scoring) -> Benchmark {
    let mut tasks: Vec<(usize, usize)> = Vec::new();

    for fighter in 0..elite.len() {
        for opponent in 0..reference.len() {
            for _ in 0..repeat.max(1) {
                tasks.push((fighter, opponent));
            }
        }
    }

    let fights: Vec<NetFight> = tasks
        .par_iter()
        .map(|(fighter, opponent)| fight_nets(&elite[*fighter], &reference[*opponent]))
        .collect();

    let mut result = Benchmark {
        fights: fights.len() as u32,
        ..Default::default()
    };

    for fight in fights {
        // Очки считаем со стороны проверяемого бойца
        let (red_points, blue_points) = points_for(
            scoring,
            fight.result,
            fight.red_lose,
            fight.blue_lose,
        );
        let (own_points, other_points) = if fight.first_is_red {
            (red_points, blue_points)
        } else {
            (blue_points, red_points)
        };

        result.points += own_points - other_points;

        match fight.result {
            RoundResult::Draw => result.draws += 1,
            RoundResult::RedWin if fight.first_is_red => result.wins += 1,
            RoundResult::BlueWin if !fight.first_is_red => result.wins += 1,
            _ => result.losses += 1,
        }
    }

    result
}

/// Сводит результаты боёв в очки и статистику
fn aggregate(outcomes: &[FightOutcome], population: usize, scoring: Scoring) -> ArenaResult {
    let mut result = ArenaResult {
        scores: vec![BASE_SCORE; population],
        stats: vec![FighterStats::default(); population],
        fights: outcomes.len() as u32,
        ..Default::default()
    };

    for outcome in outcomes {
        let (red_points, blue_points) = points_for(
            scoring,
            outcome.result,
            outcome.red_lose,
            outcome.blue_lose,
        );

        result.scores[outcome.red] += red_points;
        result.scores[outcome.blue] += blue_points;

        // Каждый удар стоит очко нанесшему и столько же отнимает у получившего:
        // сумма очков в бою от этого не меняется
        let bonus = scoring.hit_bonus() as i64;
        result.scores[outcome.red] += (outcome.red_hits_dealt as i64 * bonus) as i32;
        result.scores[outcome.blue] -= (outcome.red_hits_dealt as i64 * bonus) as i32;
        result.scores[outcome.blue] += (outcome.blue_hits_dealt as i64 * bonus) as i32;
        result.scores[outcome.red] -= (outcome.blue_hits_dealt as i64 * bonus) as i32;

        // Счётчики бойцов обновляем по отдельности: заимствовать две ячейки
        // одного вектора сразу нельзя
        let mut red = result.stats[outcome.red].clone();
        let mut blue = result.stats[outcome.blue].clone();

        red.fights += 1;
        blue.fights += 1;
        red.ticks += outcome.ticks as u64;
        blue.ticks += outcome.ticks as u64;
        red.hits_dealt += outcome.red_hits_dealt as u64;
        blue.hits_dealt += outcome.blue_hits_dealt as u64;
        red.hits_taken += outcome.blue_hits_dealt as u64;
        blue.hits_taken += outcome.red_hits_dealt as u64;

        match outcome.result {
            RoundResult::RedWin => {
                red.wins += 1;
                blue.losses += 1;

                match outcome.blue_lose {
                    Some(LoseReason::OutOfRing) => {
                        red.wins_by_ring += 1;
                        blue.losses_by_ring += 1;
                        result.out_of_ring += 1;
                    }
                    Some(LoseReason::Exhausted) => {
                        red.wins_by_exhaustion += 1;
                        blue.losses_by_exhaustion += 1;
                        result.exhausted += 1;
                    }
                    Some(LoseReason::Defeated) => {
                        red.wins_by_defeat += 1;
                        blue.losses_by_defeat += 1;
                        result.defeated += 1;
                    }
                    None => {}
                }
            }
            RoundResult::BlueWin => {
                blue.wins += 1;
                red.losses += 1;

                match outcome.red_lose {
                    Some(LoseReason::OutOfRing) => {
                        blue.wins_by_ring += 1;
                        red.losses_by_ring += 1;
                        result.out_of_ring += 1;
                    }
                    Some(LoseReason::Exhausted) => {
                        blue.wins_by_exhaustion += 1;
                        red.losses_by_exhaustion += 1;
                        result.exhausted += 1;
                    }
                    Some(LoseReason::Defeated) => {
                        blue.wins_by_defeat += 1;
                        red.losses_by_defeat += 1;
                        result.defeated += 1;
                    }
                    None => {}
                }
            }
            RoundResult::Draw => {
                red.draws += 1;
                blue.draws += 1;
                result.draws += 1;
            }
        }

        result.total_ticks += outcome.ticks as u64;

        result.stats[outcome.red] = red;
        result.stats[outcome.blue] = blue;
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use fencing_perceptron::net::Net;

    /// Каждый боец встречается с каждым ровно один раз
    #[test]
    fn every_pair_meets_once() {
        let pairs = build_pairs(64);

        assert_eq!(pairs.len(), 64 * 63 / 2);

        let mut seen = std::collections::HashSet::new();

        for pair in &pairs {
            assert!(pair.first < pair.second, "пара упорядочена неверно");
            assert!(seen.insert((pair.first, pair.second)), "пара повторилась");
        }
    }

    /// У каждого бойца ровно 63 боя
    #[test]
    fn every_fighter_has_same_number_of_fights() {
        let pairs = build_pairs(64);
        let mut counts = vec![0usize; 64];

        for pair in &pairs {
            counts[pair.first] += 1;
            counts[pair.second] += 1;
        }

        assert!(counts.iter().all(|count| *count == 63));
    }

    /// Арена считает очки и ставит всем начальный счёт
    #[test]
    fn arena_scores_every_fighter() {
        let population: Vec<Net> = (0..4).map(|seed| Net::random(4, seed)).collect();

        let result = run_arena(&population, 1, Scoring::Classic);

        assert_eq!(result.scores.len(), 4);
        assert_eq!(result.fights, 6);

        for (index, stats) in result.stats.iter().enumerate() {
            assert_eq!(stats.fights, 3, "боец {index}");
            assert_eq!(
                stats.wins + stats.draws + stats.losses,
                3,
                "боец {index}: исходы не сходятся"
            );
        }
    }

    /// В решительных боях очки в сумме дают ноль, в ничьих — минус два
    #[test]
    fn arena_points_are_consistent() {
        let population: Vec<Net> = (0..4).map(|seed| Net::random(4, seed)).collect();

        let result = run_arena(&population, 1, Scoring::Classic);

        let total: i32 = result.scores.iter().sum();
        let expected = BASE_SCORE as i64 * population.len() as i64 - 2 * result.draws as i64;

        assert_eq!(total as i64, expected);
    }
}
