//! Реакция бойца: восемь этапов, на каждом — параллельные команды.
//!
//! Этап описывает, что боец делает: идёт в одном из восьми направлений,
//! поворачивает и, возможно, наносит удар. Длительности команд задаются
//! целым числом тиков, чтобы границы этапов не уплывали из-за ошибок
//! накопления `f32`.

use fastrand::Rng;
use serde::{Deserialize, Serialize};

use crate::{
    fighter::action::{Direction, SwordAction},
    math::TPS,
};

/// Число этапов в одной реакции
pub const REACTION_STAGES: usize = 8;

/// Максимальная длительность команды в тиках: 25 тиков — это полсекунды при
/// частоте симуляции 50 тиков в секунду
pub const MAX_STAGE_TICKS: u32 = 25;

/// Направление движения: восемь направлений в локальной системе координат бойца.
///
/// Это ровно все ненулевые комбинации `forward` и `side` из `{-1, 0, 1}`,
/// поэтому каждое направление раскладывается в поля команды `Action`.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum MoveDir8 {
    /// Только вперёд
    Forward,
    /// Только назад
    Back,
    /// Только влево
    Left,
    /// Только вправо
    Right,
    /// Вперёд и влево
    ForwardLeft,
    /// Вперёд и вправо
    ForwardRight,
    /// Назад и влево
    BackLeft,
    /// Назад и вправо
    BackRight,
}

impl MoveDir8 {
    /// Все восемь направлений
    pub const ALL: [MoveDir8; 8] = [
        MoveDir8::Forward,
        MoveDir8::Back,
        MoveDir8::Left,
        MoveDir8::Right,
        MoveDir8::ForwardLeft,
        MoveDir8::ForwardRight,
        MoveDir8::BackLeft,
        MoveDir8::BackRight,
    ];

    /// Составляющая «вперёд или назад»
    pub fn forward(self) -> Direction {
        match self {
            MoveDir8::Forward | MoveDir8::ForwardLeft | MoveDir8::ForwardRight => {
                Direction::Positive
            }
            MoveDir8::Back | MoveDir8::BackLeft | MoveDir8::BackRight => Direction::Negative,
            MoveDir8::Left | MoveDir8::Right => Direction::Zero,
        }
    }

    /// Составляющая «вправо или влево»
    pub fn side(self) -> Direction {
        match self {
            MoveDir8::Right | MoveDir8::ForwardRight | MoveDir8::BackRight => Direction::Positive,
            MoveDir8::Left | MoveDir8::ForwardLeft | MoveDir8::BackLeft => Direction::Negative,
            MoveDir8::Forward | MoveDir8::Back => Direction::Zero,
        }
    }
}

/// Переводит длительность из секунд в тики: 0.4 секунды — это 20 тиков.
/// Значение округляется и зажимается в пределах `MAX_STAGE_TICKS`.
pub fn ticks_from_seconds(seconds: f32) -> u32 {
    ((seconds * TPS as f32).round() as u32).min(MAX_STAGE_TICKS)
}

/// Один этап реакции.
///
/// Внутри этапа команды выполняются параллельно: боец идёт и поворачивает
/// одновременно, каждая команда своё время. Этап заканчивается, когда
/// отработали все команды, то есть через максимум из их длительностей.
#[derive(Clone, Copy, Default, PartialEq, Debug, Serialize, Deserialize)]
pub struct Stage {
    /// Команда движения: направление и её длительность в тиках
    pub move_command: Option<(MoveDir8, u32)>,
    /// Команда поворота: направление и её длительность в тиках.
    /// `Positive` — против часовой стрелки, `Negative` — по часовой, то есть вправо
    pub turn_command: Option<(Direction, u32)>,
    /// Удар, выполняемый в первом тике этапа
    pub strike: Option<SwordAction>,
}

impl Stage {
    /// Пустой этап: боец ничего не делает
    pub fn empty() -> Self {
        Self::default()
    }

    /// Длительность этапа в тиках: максимум из длительностей команд
    pub fn duration_ticks(&self) -> u32 {
        self.move_ticks().max(self.turn_ticks())
    }

    /// Длительность команды движения в тиках
    pub fn move_ticks(&self) -> u32 {
        self.move_command.map_or(0, |(_, ticks)| ticks)
    }

    /// Длительность команды поворота в тиках
    pub fn turn_ticks(&self) -> u32 {
        self.turn_command.map_or(0, |(_, ticks)| ticks)
    }

    /// Есть ли в этапе хоть какая-то команда
    pub fn is_empty(&self) -> bool {
        self.move_command.is_none() && self.turn_command.is_none() && self.strike.is_none()
    }
}

/// Реакция: восемь этапов, выполняемых подряд
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Reaction {
    pub stages: [Stage; REACTION_STAGES],
}

impl Reaction {
    /// Реакция из восьми пустых этапов
    pub fn empty() -> Self {
        Self {
            stages: [Stage::empty(); REACTION_STAGES],
        }
    }

    /// Суммарная длительность реакции в тиках
    pub fn total_ticks(&self) -> u32 {
        self.stages.iter().map(Stage::duration_ticks).sum()
    }
}

/// Случайная реакция.
///
/// Непустых этапов немного — один, два или три, остальные пустые и
/// пропускаются мгновенно. Ограничение существенно: пространство реакций
/// (восемь этапов, у каждого восемь направлений и два dozen длительностей)
/// неразмеримо, и случайная реакция, заполненная целиком, почти никогда
/// не оказывается удачной — учиться будет нечему.
pub fn random_reaction(rng: &mut Rng) -> Reaction {
    let mut stages = [Stage::empty(); REACTION_STAGES];

    let busy_stages = rng.usize(1..=3);

    let mut chosen: Vec<usize> = Vec::with_capacity(busy_stages);
    while chosen.len() < busy_stages {
        let index = rng.usize(0..REACTION_STAGES);
        if !chosen.contains(&index) {
            chosen.push(index);
        }
    }

    for index in chosen {
        stages[index] = random_stage(rng);
    }

    Reaction { stages }
}

/// Случайный этап: движение, поворот и удар выбираются независимо
fn random_stage(rng: &mut Rng) -> Stage {
    let mut stage = Stage::empty();

    if rng.bool() {
        let dir = MoveDir8::ALL[rng.usize(0..MoveDir8::ALL.len())];
        stage.move_command = Some((dir, rng.u32(1..=MAX_STAGE_TICKS)));
    }

    if rng.bool() {
        let dir = if rng.bool() {
            Direction::Positive
        } else {
            Direction::Negative
        };
        stage.turn_command = Some((dir, rng.u32(1..=MAX_STAGE_TICKS)));
    }

    if rng.bool() {
        stage.strike = Some(if rng.bool() {
            SwordAction::Prick
        } else {
            SwordAction::Chok
        });
    }

    // Пустой этап ничего не делает, поэтому в реакции он бесполезен:
    // на его месте оказывается движение
    if stage.is_empty() {
        let dir = MoveDir8::ALL[rng.usize(0..MoveDir8::ALL.len())];
        stage.move_command = Some((dir, rng.u32(1..=MAX_STAGE_TICKS)));
    }

    stage
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Длительность этапа — максимум из длительностей его команд
    #[test]
    fn stage_duration_is_max_of_commands() {
        let stage = Stage {
            move_command: Some((MoveDir8::Forward, 20)),
            turn_command: Some((Direction::Negative, 15)),
            strike: None,
        };

        assert_eq!(stage.duration_ticks(), 20);
        assert_eq!(stage.move_ticks(), 20);
        assert_eq!(stage.turn_ticks(), 15);
    }

    /// Пустой этап ничего не занимает
    #[test]
    fn empty_stage_has_zero_duration() {
        let stage = Stage::empty();

        assert_eq!(stage.duration_ticks(), 0);
        assert!(stage.is_empty());
    }

    /// Удар не продлевает этап: он мгновенный
    #[test]
    fn strike_does_not_extend_stage() {
        let stage = Stage {
            move_command: None,
            turn_command: None,
            strike: Some(SwordAction::Prick),
        };

        assert_eq!(stage.duration_ticks(), 0);
        assert!(!stage.is_empty());
    }

    /// Восемь направлений раскладываются в поля команды
    #[test]
    fn move_dir_matches_action_parts() {
        assert_eq!(MoveDir8::Forward.forward(), Direction::Positive);
        assert_eq!(MoveDir8::Forward.side(), Direction::Zero);

        assert_eq!(MoveDir8::Back.forward(), Direction::Negative);
        assert_eq!(MoveDir8::Back.side(), Direction::Zero);

        assert_eq!(MoveDir8::Right.side(), Direction::Positive);
        assert_eq!(MoveDir8::Right.forward(), Direction::Zero);

        assert_eq!(MoveDir8::Left.side(), Direction::Negative);

        assert_eq!(MoveDir8::ForwardRight.forward(), Direction::Positive);
        assert_eq!(MoveDir8::ForwardRight.side(), Direction::Positive);

        assert_eq!(MoveDir8::BackLeft.forward(), Direction::Negative);
        assert_eq!(MoveDir8::BackLeft.side(), Direction::Negative);
    }

    /// Каждое из восьми направлений задаёт ненулевое движение
    #[test]
    fn every_direction_moves_the_fighter() {
        assert_eq!(MoveDir8::ALL.len(), 8);

        for dir in MoveDir8::ALL {
            let moves = dir.forward() != Direction::Zero || dir.side() != Direction::Zero;

            assert!(moves, "направление {dir:?} не задаёт движения");
        }
    }

    /// Длительность в секундах переводится в целые тики
    #[test]
    fn ticks_from_seconds_rounds_to_ticks() {
        assert_eq!(ticks_from_seconds(0.4), 20);
        assert_eq!(ticks_from_seconds(0.5), 25);
        assert_eq!(ticks_from_seconds(0.02), 1);
        // Дольше половины секунды нельзя: длительность команды ограничена
        assert_eq!(ticks_from_seconds(0.6), MAX_STAGE_TICKS);
    }

    /// В случайной реакции восемь этапов и не больше трёх непустых
    #[test]
    fn random_reaction_has_few_busy_stages() {
        let mut rng = Rng::with_seed(7);

        for _ in 0..100 {
            let reaction = random_reaction(&mut rng);

            assert_eq!(reaction.stages.len(), REACTION_STAGES);

            let busy = reaction.stages.iter().filter(|s| !s.is_empty()).count();
            assert!((1..=3).contains(&busy), "непустых этапов: {busy}");
        }
    }

    /// Длительности команд случайной реакции не превышают половину секунды
    #[test]
    fn random_reaction_durations_are_bounded() {
        let mut rng = Rng::with_seed(11);

        for _ in 0..100 {
            let reaction = random_reaction(&mut rng);

            for stage in reaction.stages {
                assert!(stage.duration_ticks() <= MAX_STAGE_TICKS);
            }
        }
    }

    /// Один и тот же генератор даёт одну и ту же реакцию
    #[test]
    fn random_reaction_is_reproducible() {
        let first = random_reaction(&mut Rng::with_seed(42));
        let second = random_reaction(&mut Rng::with_seed(42));

        assert_eq!(first, second);
    }

    /// Длительность реакции равна сумме длительностей этапов
    #[test]
    fn total_ticks_sums_stages() {
        let mut reaction = Reaction::empty();
        reaction.stages[0].move_command = Some((MoveDir8::Forward, 20));
        reaction.stages[3].turn_command = Some((Direction::Positive, 5));

        assert_eq!(reaction.total_ticks(), 25);
    }
}
