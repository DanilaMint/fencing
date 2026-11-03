//! Стратег: мозг, работающий по правилу «стимул -> реакция».
//!
//! Реакция долгосрочная: восемь этапов, на каждом — параллельные команды
//! (движение, поворот) и, возможно, удар в первом тике этапа. Реакция
//! выбирается из таблицы по сходству стимула с текущей обстановкой,
//! а результат реакции оценивается и записывается в рейтинг записи:
//! удачные реакции помнятся, неудачные стираются.

pub mod reaction;
pub mod reward;
pub mod stimulus;
pub mod table;

use std::{
    collections::VecDeque,
    io,
    path::Path,
};

use fastrand::Rng;
use serde::{Deserialize, Serialize};

use crate::{
    fighter::{
        action::{Action, SwordAction},
        brain::{Brain, BrainEvent},
    },
    round::situation::Situation,
};

use self::{
    reaction::{REACTION_STAGES, Reaction},
    reward::Reward,
    stimulus::Stimulus,
    table::{Table, TABLE_SIZE},
};

/// Сколько тиков отложенный удар может ждать готовности меча.
/// Меч после удара занят около 0.8 секунды, то есть примерно 40 тиков
pub const PENDING_STRIKE_MAX_TICKS: u32 = 100;
/// Сколько ударов может ждать очереди одновременно
pub const PENDING_STRIKES_MAX: usize = 2;

/// Имя модели мозга, которой принадлежит сохранённая таблица
pub const MODEL_NAME: &str = "strategist";
/// Версия формата сохранения обученного мозга
pub const FORMAT_VERSION: u32 = 2;

/// Обученный мозг стратега, сохранённый на диск.
///
/// Файл сам объявляет, какой моделью мозга он создан: обученные мозги можно
/// копить рядом с будущими моделями и разбирать по этой метке.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SavedBrain {
    /// Модель мозга, которой принадлежит таблица
    pub model: String,
    /// Версия формата
    pub version: u32,
    /// Имя бойца
    pub name: String,
    /// Зерно генератора: позволяет продолжить обучение с того же места
    pub seed: u64,
    /// Обученная таблица реакций
    pub table: Table,
}

/// Стратег: мозг с таблицей реакций
pub struct Strategist {
    /// Таблица «стимул -> реакция»
    table: Table,
    /// Генератор случайных чисел
    rng: Rng,
    /// Зерно генератора: сохраняется вместе с обученным мозгом
    seed: u64,
    /// Текущее исполнение реакции
    execution: Option<Execution>,
    /// Последнее известное расстояние бойца от центра ринга
    last_center_distance: f32,
    /// Последнее известное расстояние противника от центра ринга
    last_enemy_center_distance: f32,
}

/// Состояние исполняемой реакции
struct Execution {
    /// Индекс записи в таблице
    entry: usize,
    /// Исполняемая реакция
    reaction: Reaction,
    /// Номер текущего этапа
    stage: usize,
    /// Сколько тиков прошло в текущем этапе
    tick: u32,
    /// Удары, ждущие готовности меча
    pending_strikes: VecDeque<SwordAction>,
    /// Удар запрошен в момент, когда меч был свободен
    strike_armed: bool,
    /// Сколько тиков очередь ударов ждёт свободного меча
    pending_wait: u32,
    /// Накопленный результат реакции
    reward: Reward,
    /// Расстояние до центра ринга в начале реакции
    start_center_distance: f32,
    /// Расстояние противника до центра ринга в начале реакции
    start_enemy_center_distance: f32,
}

impl Execution {
    /// Ставит в очередь удар, задуманный текущим этапом
    fn arm_stage_strike(&mut self) {
        if self.pending_strikes.len() >= PENDING_STRIKES_MAX {
            return;
        }

        if let Some(strike) = self.reaction.stages[self.stage].strike {
            self.pending_strikes.push_back(strike);
        }
    }

    /// Действие текущего этапа
    fn current_action(&mut self, situation: &Situation) -> Action {
        let stage = self.reaction.stages[self.stage];
        let mut action = Action::default();

        // Движение: команда активна, пока не вышла её длительность
        if let Some((dir, duration)) = stage.move_command {
            if self.tick < duration {
                action.forward = dir.forward();
                action.side = dir.side();
            }
        }

        // Поворот: команда активна, пока не вышла её длительность
        if let Some((dir, duration)) = stage.turn_command {
            if self.tick < duration {
                action.rotate = dir;
            }
        }

        // Отложенный удар: наносится, как только меч освободился
        if let Some(strike) = self.pending_strikes.front().copied() {
            if situation.sword_ready {
                // Меч в стойке: удар уходит в этот тик
                action.sword = strike;
                self.strike_armed = true;
                self.pending_wait = 0;
            } else if self.strike_armed {
                // Клинок ушёл в атаку: удар нанесён, очередь сдвигается
                self.pending_strikes.pop_front();
                self.strike_armed = false;
            } else {
                // Меч занят: ждём, пока освободится
                self.pending_wait += 1;

                if self.pending_wait > PENDING_STRIKE_MAX_TICKS {
                    // Ждать дольше бессмысленно: удар срывается
                    self.pending_strikes.pop_front();
                    self.pending_wait = 0;
                }
            }
        }

        action
    }
}

impl Strategist {
    /// Новый стратег с пустой таблицей
    pub fn new() -> Self {
        Self::with_seed(fastrand::u64(..))
    }

    /// Новый стратег с фиксированным зерном генератора: поведение повторяемо
    pub fn with_seed(seed: u64) -> Self {
        Self::with_seed_and_table(seed, Table::new(TABLE_SIZE))
    }

    /// Стратег с готовой таблицей: накопленное обучение переносится в новый бой
    pub fn with_table(table: Table) -> Self {
        Self::with_seed_and_table(fastrand::u64(..), table)
    }

    /// Стратег с фиксированным зерном и готовой таблицей:
    /// обучение продолжается с сохранённого места и остаётся повторяемым
    pub fn with_seed_and_table(seed: u64, table: Table) -> Self {
        Self {
            table,
            rng: Rng::with_seed(seed),
            seed,
            execution: None,
            last_center_distance: 0.,
            last_enemy_center_distance: 0.,
        }
    }

    /// Таблица реакций: её можно сохранить и загрузить
    pub fn table(&self) -> &Table {
        &self.table
    }

    /// Зерно генератора стратега
    pub fn seed(&self) -> u64 {
        self.seed
    }

    /// Сохраняет обученный мозг в файл.
    ///
    /// В файле указывается модель мозга, поэтому обученные стратеги можно
    /// отличать от мозгов других моделей.
    pub fn save_to_file(&self, path: impl AsRef<Path>, name: &str) -> io::Result<()> {
        let saved = SavedBrain {
            model: MODEL_NAME.to_string(),
            version: FORMAT_VERSION,
            name: name.to_string(),
            seed: self.seed,
            table: self.table.clone(),
        };

        let text = serde_json::to_string_pretty(&saved).map_err(data_error)?;

        std::fs::write(path, text)
    }

    /// Загружает обученный мозг из файла.
    ///
    /// Если файл сохранён для другой модели мозга, выдаётся ошибка:
    /// молча принимать чужой файл нельзя.
    pub fn load_brain(path: impl AsRef<Path>) -> io::Result<SavedBrain> {
        let text = std::fs::read_to_string(path)?;
        let saved: SavedBrain = serde_json::from_str(&text).map_err(data_error)?;

        if saved.model != MODEL_NAME {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "файл сохранён для другой модели мозга: \"{}\", а не \"{}\"",
                    saved.model, MODEL_NAME
                ),
            ));
        }

        Ok(saved)
    }

    /// Начинает реакцию, подходящую под текущую обстановку
    fn start_reaction(&mut self, situation: &Situation) {
        let stimulus = Stimulus::from_situation(situation);
        let index = self.table.select(&stimulus, &mut self.rng);

        let Some(entry) = self.table.entry(index) else {
            return;
        };

        let mut execution = Execution {
            entry: index,
            reaction: entry.reaction.clone(),
            stage: 0,
            tick: 0,
            pending_strikes: VecDeque::new(),
            strike_armed: false,
            pending_wait: 0,
            reward: Reward::new(),
            start_center_distance: situation.center_distance,
            start_enemy_center_distance: situation.enemy_center_distance,
        };

        // Удар первого этапа встаёт в очередь сразу
        execution.arm_stage_strike();

        self.execution = Some(execution);
    }

    /// Реакция закончилась: результат оценивается и записывается в рейтинг
    fn finish_reaction(&mut self, execution: Execution) {
        self.apply_reward(execution);
    }

    /// Завершает текущую реакцию: её результат уходит в рейтинг записи,
    /// а следующая реакция начнётся с нуля.
    ///
    /// Нужно на границе боёв: иначе реакция, начатая в одном бою, продолжится
    /// в следующем, а её результат смешается с результатом нового боя.
    pub fn end_reaction(&mut self) {
        let Some(execution) = self.execution.take() else {
            return;
        };

        self.apply_reward(execution);
    }

    /// Записывает результат реакции в рейтинг её записи
    fn apply_reward(&mut self, execution: Execution) {
        let center_delta = self.last_center_distance - execution.start_center_distance;
        let enemy_center_delta =
            self.last_enemy_center_distance - execution.start_enemy_center_distance;

        let delta = execution.reward.score(center_delta, enemy_center_delta);

        self.table.apply_rating(execution.entry, delta);
    }
}

impl Default for Strategist {
    fn default() -> Self {
        Self::new()
    }
}

/// Ошибка разбора или записи JSON переводится в ошибку ввода-вывода
fn data_error(err: serde_json::Error) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, err)
}

impl Brain for Strategist {
    fn get_actions(&mut self, situation: Situation) -> Action {
        self.last_center_distance = situation.center_distance;
        self.last_enemy_center_distance = situation.enemy_center_distance;

        if self.execution.is_none() {
            self.start_reaction(&situation);
        }

        // Исполнение забирается на время: чтобы завершить реакцию,
        // нужен доступ к таблице
        let Some(mut execution) = self.execution.take() else {
            return Action::default();
        };

        let action = execution.current_action(&situation);

        // Продвигаем исполнение на один тик
        execution.tick += 1;

        let duration = execution.reaction.stages[execution.stage].duration_ticks();
        if execution.tick >= duration {
            execution.stage += 1;
            execution.tick = 0;

            if execution.stage < REACTION_STAGES {
                execution.arm_stage_strike();
            }
        }

        if execution.stage >= REACTION_STAGES {
            // Реакция отыграна: оцениваем её и сразу выбираем следующую
            self.finish_reaction(execution);
            self.start_reaction(&situation);
        } else {
            self.execution = Some(execution);
        }

        action
    }

    fn on_event(&mut self, event: BrainEvent) {
        let Some(mut execution) = self.execution.take() else {
            return;
        };

        execution.reward.add_event(event);

        if matches!(event, BrainEvent::Shocked) {
            // Шок прерывает выполнение реакции: накопленный результат
            // (включая штраф за шок) уходит в рейтинг
            self.finish_reaction(execution);
        } else {
            self.execution = Some(execution);
        }
    }

    /// Раунд закончился: реакция закрывается, её результат уходит в рейтинг
    fn on_round_end(&mut self) {
        self.end_reaction();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        fighter::action::Direction,
        fighter::brain::strategist::{
            reaction::MoveDir8,
            reward::HIT_DEALT_WEIGHT,
            table::{Entry, INITIAL_RATING},
        },
        math::approx_eq,
    };

    /// Обстановка, в которой меч свободен
    fn situation() -> Situation {
        Situation {
            sword_ready: true,
            ..Default::default()
        }
    }

    /// Реакция: первый этап — движение вперёд, второй — поворот вправо
    fn reaction() -> Reaction {
        let mut reaction = Reaction::empty();
        reaction.stages[0].move_command = Some((MoveDir8::Forward, 20));
        reaction.stages[1].turn_command = Some((Direction::Negative, 10));

        reaction
    }

    /// Стратег с одной заготовленной записью
    fn strategist_with(reaction: Reaction) -> Strategist {
        let mut table = Table::new(2);
        table.set_entry(
            0,
            Entry {
                stimulus: Stimulus::default(),
                reaction,
                rating: INITIAL_RATING,
                uses: 1,
            },
        );

        Strategist::with_table(table)
    }

    /// Первое действие соответствует первому этапу реакции
    #[test]
    fn first_action_follows_first_stage() {
        let mut strategist = strategist_with(reaction());

        let action = strategist.get_actions(situation());

        assert_eq!(action.forward, Direction::Positive);
        assert_eq!(action.side, Direction::Zero);
        assert_eq!(action.rotate, Direction::Zero);
    }

    /// Команда этапа работает, пока не вышла её длительность
    #[test]
    fn stage_command_ends_with_its_duration() {
        let mut strategist = strategist_with(reaction());

        // Первые двадцать тиков боец идёт вперёд
        for _ in 0..20 {
            let action = strategist.get_actions(situation());
            assert_eq!(action.forward, Direction::Positive);
        }

        // Дальше первый этап закончился: начался второй, с поворотом
        let action = strategist.get_actions(situation());
        assert_eq!(action.forward, Direction::Zero);
        assert_eq!(action.rotate, Direction::Negative);
    }

    /// После восьми этапов реакция заканчивается и выбирается следующая
    #[test]
    fn reaction_finishes_after_eight_stages() {
        let mut strategist = strategist_with(Reaction::empty());

        // Пустой этап длится один тик, поэтому реакция из восьми этапов
        // занимает восемь тиков
        for _ in 0..REACTION_STAGES - 1 {
            strategist.get_actions(situation());
        }

        assert!(strategist.execution.is_some());

        strategist.get_actions(situation());

        // Новая реакция началась с первого этапа
        let execution = strategist.execution.as_ref().expect("реакция не начата");
        assert_eq!(execution.stage, 0);
        assert_eq!(execution.tick, 0);
    }

    /// Удар этапа запрашивается в первом тике этого этапа
    #[test]
    fn strike_is_issued_on_first_tick_of_stage() {
        let mut reaction = Reaction::empty();
        reaction.stages[0].strike = Some(SwordAction::Prick);

        let mut strategist = strategist_with(reaction);

        let action = strategist.get_actions(situation());

        assert_eq!(action.sword, SwordAction::Prick);
    }

    /// Занятый меч не мешает удару: боец ждёт готовности и бьёт
    #[test]
    fn strike_waits_for_sword_ready() {
        let mut reaction = Reaction::empty();
        reaction.stages[0].strike = Some(SwordAction::Prick);

        let mut strategist = strategist_with(reaction);

        // Меч занят: удар не выдаётся
        let busy = Situation {
            sword_ready: false,
            ..Default::default()
        };
        let action = strategist.get_actions(busy);
        assert_eq!(action.sword, SwordAction::None);

        // Меч освободился: удар уходит
        let action = strategist.get_actions(situation());
        assert_eq!(action.sword, SwordAction::Prick);
    }

    /// Реакция без ударов никогда не бьёт
    #[test]
    fn reaction_without_strike_never_strikes() {
        let mut strategist = strategist_with(reaction());

        for _ in 0..REACTION_STAGES {
            let action = strategist.get_actions(situation());
            assert_eq!(action.sword, SwordAction::None);
        }
    }

    /// Шок прерывает реакцию и ухудшает рейтинг записи
    #[test]
    fn shock_interrupts_reaction() {
        let mut strategist = strategist_with(reaction());

        strategist.get_actions(situation());
        assert!(strategist.execution.is_some());

        strategist.on_event(BrainEvent::Shocked);

        assert!(strategist.execution.is_none(), "реакция должна быть прервана");
        assert!(
            strategist.table().rating(0).unwrap() < INITIAL_RATING,
            "прерванная реакция должна получить штраф"
        );
    }

    /// Нанесённый удар повышает рейтинг записи
    #[test]
    fn hit_dealt_raises_rating() {
        let mut strategist = strategist_with(Reaction::empty());

        // Пустая реакция занимает восемь тиков
        for _ in 0..REACTION_STAGES - 1 {
            strategist.get_actions(situation());
        }

        strategist.on_event(BrainEvent::HitDealt { power: 1. });
        strategist.get_actions(situation());

        let rating = strategist.table().rating(0).unwrap();
        assert!(rating > INITIAL_RATING, "рейтинг: {rating}");
    }

    /// Пропущенный удар снижает рейтинг записи
    #[test]
    fn hit_taken_lowers_rating() {
        let mut strategist = strategist_with(Reaction::empty());

        for _ in 0..REACTION_STAGES - 1 {
            strategist.get_actions(situation());
        }

        strategist.on_event(BrainEvent::HitTaken { power: 0.5 });
        strategist.get_actions(situation());

        let rating = strategist.table().rating(0).unwrap();
        assert!(rating < INITIAL_RATING, "рейтинг: {rating}");
    }

    /// Стратег без заготовленных записей сам наполняет таблицу
    #[test]
    fn strategist_fills_table_over_time() {
        let mut strategist = Strategist::with_seed(5);

        for _ in 0..100 {
            strategist.get_actions(situation());
        }

        assert!(strategist.table().filled() > 0);
    }

    /// Одинаковое зерно даёт одинаковое поведение
    #[test]
    fn same_seed_gives_same_behaviour() {
        let mut first = Strategist::with_seed(17);
        let mut second = Strategist::with_seed(17);

        for _ in 0..50 {
            let a = first.get_actions(situation());
            let b = second.get_actions(situation());

            assert_eq!(a.forward, b.forward);
            assert_eq!(a.side, b.side);
            assert_eq!(a.rotate, b.rotate);
            assert_eq!(a.sword, b.sword);
        }
    }

    /// Обученный мозг сохраняется и читается обратно
    #[test]
    fn saved_brain_survives_save_and_load() {
        let mut strategist = Strategist::with_seed(99);
        strategist.table.set_entry(
            0,
            Entry {
                stimulus: Stimulus::default(),
                reaction: Reaction::empty(),
                rating: INITIAL_RATING,
                uses: 1,
            },
        );

        let path = std::env::temp_dir().join("fencing_brain_test.json");
        strategist.save_to_file(&path, "tester").unwrap();

        let saved = Strategist::load_brain(&path).unwrap();

        // Файл сам объявляет, какой моделью мозга он создан
        assert_eq!(saved.model, MODEL_NAME);
        assert_eq!(saved.name, "tester");
        assert_eq!(saved.seed, strategist.seed());
        assert_eq!(saved.table, *strategist.table());

        let _ = std::fs::remove_file(&path);
    }

    /// Файл чужой модели мозга не принимается молча
    #[test]
    fn load_brain_rejects_other_model() {
        let path = std::env::temp_dir().join("fencing_brain_other_test.json");
        std::fs::write(
            &path,
            r#"{"model":"doll","version":1,"name":"x","seed":1,"table":{"entries":[]}}"#,
        )
        .unwrap();

        let err = Strategist::load_brain(&path).unwrap_err();

        assert!(
            err.to_string().contains("другой модели мозга"),
            "непонятная ошибка: {err}"
        );

        let _ = std::fs::remove_file(&path);
    }

    /// Награда за удар масштабируется силой удара
    #[test]
    fn stronger_hit_gives_more_rating() {
        let mut weak = strategist_with(Reaction::empty());
        let mut strong = strategist_with(Reaction::empty());

        for _ in 0..REACTION_STAGES - 1 {
            weak.get_actions(situation());
            strong.get_actions(situation());
        }

        weak.on_event(BrainEvent::HitDealt { power: 0.1 });
        strong.on_event(BrainEvent::HitDealt { power: 0.5 });
        weak.get_actions(situation());
        strong.get_actions(situation());

        let weak_rating = weak.table().rating(0).unwrap();
        let strong_rating = strong.table().rating(0).unwrap();

        assert!(strong_rating > weak_rating);
        // Рейтинг — среднее по выполнениям, поэтому разница вдвое меньше
        assert!(approx_eq(
            strong_rating - weak_rating,
            HIT_DEALT_WEIGHT * 0.4 / 2.
        ));
    }
}
