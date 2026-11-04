//! Перцептрон: мозг бойца, работающий на нейросети.
//!
//! Сеть смотрит на обстановку и выдаёт команду бойцу: куда идти, куда
//! повернуться и бить или нет. Каждый тик обстановка переводится в восемь
//! признаков, сеть считает вероятности действий, и по одной вероятности
//! из каждой группы выбирается действие.
//!
//! Обучение сети — отдельная задача: веса и промежутки прямого прохода
//! доступны наружу (`net_mut`, `last_forward`), поэтому обучать сеть можно
//! снаружи этого мозга.

pub mod features;
pub mod net;
pub mod output;

use std::{io, path::Path};

use fastrand::Rng;
use serde::{Deserialize, Serialize};

use fencing_core::{
    brain::{Brain, BrainEvent},
    fighter::action::Action,
    round::situation::Situation,
};

use self::{
    features::Features,
    net::{DEFAULT_HIDDEN, Forward, Net, argmax, sample_index},
};

/// Имя модели мозга, которой принадлежит сохранённая сеть
pub const MODEL_NAME: &str = "perceptron";
/// Версия формата сохранения обученного мозга
pub const FORMAT_VERSION: u32 = 1;

/// Обученный мозг перцептрона, сохранённый на диск.
///
/// Файл сам объявляет, какой моделью мозга он создан: обученные сети можно
/// копить рядом с мозгами других моделей и разбирать по этой метке.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SavedBrain {
    /// Модель мозга, которой принадлежит сеть
    pub model: String,
    /// Версия формата
    pub version: u32,
    /// Имя бойца
    pub name: String,
    /// Зерно генератора: позволяет воспроизвести поведение
    pub seed: u64,
    /// Обученная сеть
    pub net: Net,
}

/// Мозг бойца на нейросети.
pub struct Perceptron {
    /// Сеть: признаки на входе, вероятности действий на выходе
    net: Net,
    /// Генератор случайных чисел: выбор действия по вероятностям
    rng: Rng,
    /// Зерно генератора: сохраняется вместе с мозгом
    seed: u64,
    /// Выбирается ли действие случайно или по максимальной вероятности
    explore: bool,
    /// Промежутки последнего прямого прохода: по ним считается градиент
    last: Option<Forward>,
}

impl Perceptron {
    /// Мозг с готовой сетью
    ///
    /// По умолчанию действие выбирается случайно, по вероятностям сети:
    /// так поведение стохастично, иначе учиться нечему.
    pub fn new(net: Net, seed: u64) -> Self {
        Self {
            net,
            rng: Rng::with_seed(seed),
            seed,
            explore: true,
            last: None,
        }
    }

    /// Мозг со случайно инициализированной сетью
    pub fn random(seed: u64) -> Self {
        Self::new(Net::random(DEFAULT_HIDDEN, seed), seed)
    }

    /// Мозг со случайно инициализированной сетью заданной ширины
    pub fn random_with_hidden(hidden: usize, seed: u64) -> Self {
        Self::new(Net::random(hidden, seed), seed)
    }

    /// Мозг из сохранённого файла
    pub fn from_saved(saved: SavedBrain) -> io::Result<Self> {
        saved.net.check().map_err(|reason| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("сохранённая сеть повреждена: {reason}"),
            )
        })?;

        Ok(Self {
            net: saved.net,
            rng: Rng::with_seed(saved.seed),
            seed: saved.seed,
            explore: true,
            last: None,
        })
    }

    /// Сеть мозга: её веса можно менять снаружи
    pub fn net(&self) -> &Net {
        &self.net
    }

    /// Сеть мозга для изменения: здесь обучение будет менять веса
    pub fn net_mut(&mut self) -> &mut Net {
        &mut self.net
    }

    /// Промежутки последнего прямого прохода
    pub fn last_forward(&self) -> Option<&Forward> {
        self.last.as_ref()
    }

    /// Зерно генератора мозга
    pub fn seed(&self) -> u64 {
        self.seed
    }

    /// Выбирается ли действие случайно
    pub fn explore(&self) -> bool {
        self.explore
    }

    /// Переключает выбор действия: случайно или по максимальной вероятности
    pub fn set_explore(&mut self, explore: bool) {
        self.explore = explore;
    }

    /// Сохраняет мозг в файл
    pub fn save_to_file(&self, path: impl AsRef<Path>, name: &str) -> io::Result<()> {
        let saved = SavedBrain {
            model: MODEL_NAME.to_string(),
            version: FORMAT_VERSION,
            name: name.to_string(),
            seed: self.seed,
            net: self.net.clone(),
        };

        let text = serde_json::to_string_pretty(&saved).map_err(data_error)?;

        std::fs::write(path, text)
    }

    /// Загружает мозг из файла
    ///
    /// Если файл сохранён для другой модели мозга, выдаётся ошибка:
    /// молча принимать чужой файл нельзя. Метка модели проверяется прежде
    /// разбора остальных полей, поэтому чужой файл отвергается с понятным
    /// сообщением, а не с ошибкой разбора.
    pub fn load_brain(path: impl AsRef<Path>) -> io::Result<SavedBrain> {
        let text = std::fs::read_to_string(path)?;
        let value: serde_json::Value = serde_json::from_str(&text).map_err(data_error)?;

        check_model(&value)?;

        serde_json::from_str(&text).map_err(data_error)
    }

    /// Выбор действия по вероятностям группы
    fn choose(&mut self, probs: &[f32]) -> usize {
        if self.explore {
            sample_index(probs, &mut self.rng)
        } else {
            argmax(probs)
        }
    }
}

impl Default for Perceptron {
    fn default() -> Self {
        Self::random(fastrand::u64(..))
    }
}

/// Мозг печатается без самих весов: они слишком длинные, чтобы смотреть на них
impl std::fmt::Debug for Perceptron {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Perceptron")
            .field("hidden", &self.net.hidden)
            .field("weights", &self.net.weight_count())
            .field("seed", &self.seed)
            .field("explore", &self.explore)
            .finish()
    }
}

/// Ошибка разбора или записи JSON переводится в ошибку ввода-вывода
fn data_error(err: serde_json::Error) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, err)
}

/// Проверяет, что файл сохранён для этой версии формата
///
/// Старые сети умели меньше, поэтому молча принимать их нельзя: сеть
/// со старыми весами просто не совпадёт по размеру.
#[allow(dead_code)]
fn check_version(value: &serde_json::Value) -> io::Result<()> {
    let version = value.get("version").and_then(serde_json::Value::as_u64);

    match version {
        Some(version) if version == FORMAT_VERSION as u64 => Ok(()),
        Some(version) => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("файл сохранён для версии {version}, а читается версия {FORMAT_VERSION}"),
        )),
        None => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "в файле нет версии формата",
        )),
    }
}

/// Проверяет, что файл сохранён для этой модели мозга
fn check_model(value: &serde_json::Value) -> io::Result<()> {
    let model = value.get("model").and_then(serde_json::Value::as_str);

    match model {
        Some(model) if model == MODEL_NAME => Ok(()),
        Some(model) => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("файл сохранён для другой модели мозга: \"{model}\", а не \"{MODEL_NAME}\""),
        )),
        None => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "в файле нет метки модели мозга",
        )),
    }
}

impl Brain for Perceptron {
    fn get_actions(&mut self, situation: Situation) -> Action {
        let features = Features::from_situation(&situation);
        let forward = self.net.forward(&features.as_array());

        let move_index = self.choose(forward.move_probs());
        let turn_index = self.choose(forward.turn_probs());
        let sword_index = self.choose(forward.sword_probs());

        self.last = Some(forward);

        output::action(move_index, turn_index, sword_index)
    }

    /// События боя мозг пока не разбирает: обучение — отдельная задача
    fn on_event(&mut self, _event: BrainEvent) {}

    /// Раунд закончился: обучать сеть — отдельная задача
    fn on_round_end(&mut self) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use fencing_core::{
        fighter::action::{Direction, SwordAction},
        math::approx_eq,
    };

    /// Обстановка, в которой меч свободен
    fn situation() -> Situation {
        Situation {
            sword_ready: true,
            ..Default::default()
        }
    }

    /// Мозг выдаёт осмысленную команду
    #[test]
    fn brain_gives_valid_action() {
        let mut brain = Perceptron::random(1);

        for _ in 0..100 {
            let action = brain.get_actions(situation());

            assert!(matches!(
                action.forward,
                Direction::Positive | Direction::Zero | Direction::Negative
            ));
            assert!(matches!(
                action.sword,
                SwordAction::None | SwordAction::Prick | SwordAction::Chok
            ));
        }
    }

    /// Одинаковое зерно даёт одинаковое поведение
    #[test]
    fn same_seed_gives_same_behaviour() {
        let mut first = Perceptron::random(17);
        let mut second = Perceptron::random(17);

        for _ in 0..50 {
            let a = first.get_actions(situation());
            let b = second.get_actions(situation());

            assert_eq!(a.forward, b.forward);
            assert_eq!(a.side, b.side);
            assert_eq!(a.rotate, b.rotate);
            assert_eq!(a.sword, b.sword);
        }
    }

    /// Без исследования действие выбирается по максимальной вероятности:
    /// одна и та же обстановка всегда даёт одну и ту же команду
    #[test]
    fn without_exploration_action_is_deterministic() {
        let mut brain = Perceptron::random(3);
        brain.set_explore(false);

        let first = brain.get_actions(situation());

        for _ in 0..10 {
            let action = brain.get_actions(situation());

            assert_eq!(action.forward, first.forward);
            assert_eq!(action.side, first.side);
            assert_eq!(action.rotate, first.rotate);
            assert_eq!(action.sword, first.sword);
        }
    }

    /// После прямого прохода остаются его промежутки: по ним считается градиент
    #[test]
    fn last_forward_is_available_after_action() {
        let mut brain = Perceptron::random(5);

        assert!(brain.last_forward().is_none());

        brain.get_actions(situation());

        let forward = brain
            .last_forward()
            .expect("прямой проход должен сохраниться");

        assert_eq!(forward.hidden.len(), DEFAULT_HIDDEN);
        assert_eq!(forward.probs.len(), net::OUTPUT_COUNT);

        // Вероятности каждой группы нормированы
        let sum: f32 = forward.move_probs().iter().sum();
        assert!(approx_eq(sum, 1.));
    }

    /// Сеть двигает бойца: её команды доходят до тела
    #[test]
    fn perceptron_moves_fighter() {
        let mut round = fencing_core::round::Round::new(
            Box::new(Perceptron::random(11)),
            Box::new(fencing_core::brain::doll::Doll),
        );

        let start = round.red_fighter().position();

        for _ in 0..1000 {
            round.tick();
        }

        assert!(
            round.red_fighter().position() != start,
            "перцептрон должен двигать бойца"
        );
    }

    /// Мозг сохраняется и читается обратно
    #[test]
    fn saved_brain_survives_save_and_load() {
        let brain = Perceptron::random(99);
        let path = std::env::temp_dir().join("fencing_perceptron_test.json");

        brain.save_to_file(&path, "tester").unwrap();

        let saved = Perceptron::load_brain(&path).unwrap();

        assert_eq!(saved.model, MODEL_NAME);
        assert_eq!(saved.name, "tester");
        assert_eq!(saved.seed, brain.seed());
        assert_eq!(saved.net, *brain.net());

        let _ = std::fs::remove_file(&path);
    }

    /// Файл чужой модели мозга не принимается молча
    #[test]
    fn load_brain_rejects_other_model() {
        let path = std::env::temp_dir().join("fencing_perceptron_other_test.json");
        std::fs::write(
            &path,
            r#"{"model":"strategist","version":2,"name":"x","seed":1,"table":{"entries":[]}}"#,
        )
        .unwrap();

        let err = Perceptron::load_brain(&path).unwrap_err();

        assert!(
            err.to_string().contains("другой модели мозга"),
            "непонятная ошибка: {err}"
        );

        let _ = std::fs::remove_file(&path);
    }

    /// Испорченная сеть не загружается
    #[test]
    fn broken_net_is_rejected() {
        let mut brain = Perceptron::random(1);
        brain.net_mut().w1.pop();

        let path = std::env::temp_dir().join("fencing_perceptron_broken_test.json");
        brain.save_to_file(&path, "broken").unwrap();

        let saved = Perceptron::load_brain(&path).unwrap();
        let err = Perceptron::from_saved(saved).unwrap_err();

        assert!(err.to_string().contains("повреждена"), "{err}");

        let _ = std::fs::remove_file(&path);
    }
}
