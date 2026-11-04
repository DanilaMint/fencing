//! Сеть мозга: один скрытый слой и три группы выходов.
//!
//! Выходы сети разбиты на группы по смыслу: направление движения, поворот
//! и тип удара. Внутри каждой группы вероятности нормируются отдельно,
//! поэтому каждая группа — это распределение, из которого можно выбрать
//! действие.
//!
//! Промежутки прямого прохода (`Forward`) возвращаются наружу: без них
//! не посчитать градиент, когда появится обучение.

use fastrand::Rng;
use serde::{Deserialize, Serialize};

use super::features::FEATURE_COUNT;

/// Число выходов, отвечающих за направление движения
pub const MOVE_COUNT: usize = 8;
/// Число выходов, отвечающих за поворот
pub const TURN_COUNT: usize = 3;
/// Число выходов, отвечающих за тип удара
pub const SWORD_COUNT: usize = 3;
/// Всего выходов сети
pub const OUTPUT_COUNT: usize = MOVE_COUNT + TURN_COUNT + SWORD_COUNT;

/// Начало группы направления движения в выходах
pub const MOVE_START: usize = 0;
/// Начало группы поворота в выходах
pub const TURN_START: usize = MOVE_START + MOVE_COUNT;
/// Начало группы удара в выходах
pub const SWORD_START: usize = TURN_START + TURN_COUNT;

/// Ширина скрытого слоя по умолчанию
pub const DEFAULT_HIDDEN: usize = 16;

/// Сеть мозга: восемь входов, скрытый слой, четырнадцать выходов.
///
/// Веса хранятся построчно: `w1[i * FEATURE_COUNT + j]` — вес признака `j`
/// у скрытого нейрона `i`, `w2[j * hidden + i]` — вес скрытого нейрона `i`
/// у выхода `j`.
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Net {
    /// Ширина скрытого слоя
    pub hidden: usize,
    /// Веса скрытого слоя
    pub w1: Vec<f32>,
    /// Смещения скрытого слоя
    pub b1: Vec<f32>,
    /// Веса выходного слоя
    pub w2: Vec<f32>,
    /// Смещения выходного слоя
    pub b2: Vec<f32>,
}

/// Промежутки прямого прохода сети.
///
/// Хранятся целиком: по ним считается градиент, когда появится обучение.
#[derive(Clone, PartialEq, Debug)]
pub struct Forward {
    /// Признаки на входе
    pub input: [f32; FEATURE_COUNT],
    /// Значения скрытого слоя
    pub hidden: Vec<f32>,
    /// Логиты выходного слоя
    pub logits: Vec<f32>,
    /// Вероятности выходов: внутри каждой группы сумма равна единице
    pub probs: Vec<f32>,
}

impl Forward {
    /// Вероятности направления движения
    pub fn move_probs(&self) -> &[f32] {
        &self.probs[MOVE_START..TURN_START]
    }

    /// Вероятности поворота
    pub fn turn_probs(&self) -> &[f32] {
        &self.probs[TURN_START..SWORD_START]
    }

    /// Вероятности типа удара
    pub fn sword_probs(&self) -> &[f32] {
        &self.probs[SWORD_START..]
    }
}

impl Net {
    /// Сеть заданной ширины с нулевыми весами
    ///
    /// При нулевых весах все группы выходов дают равномерное распределение:
    /// боец действует наугад, но одинаково при любой обстановке.
    pub fn new(hidden: usize) -> Self {
        Self {
            hidden,
            w1: vec![0.; hidden * FEATURE_COUNT],
            b1: vec![0.; hidden],
            w2: vec![0.; OUTPUT_COUNT * hidden],
            b2: vec![0.; OUTPUT_COUNT],
        }
    }

    /// Сеть со случайными весами
    ///
    /// Одинаковое зерно даёт одинаковую сеть: обучение можно повторить.
    pub fn random(hidden: usize, seed: u64) -> Self {
        let mut rng = Rng::with_seed(seed);
        let mut net = Self::new(hidden);

        // Предел весов — по Ксавьеру: так активации не залипают на краях
        let hidden_limit = (6. / (FEATURE_COUNT + hidden) as f32).sqrt();
        let output_limit = (6. / (hidden + OUTPUT_COUNT) as f32).sqrt();

        for weight in net.w1.iter_mut() {
            *weight = uniform(&mut rng, hidden_limit);
        }

        for weight in net.w2.iter_mut() {
            *weight = uniform(&mut rng, output_limit);
        }

        // Смещения — такие же гены, как и веса: нулевыми они оставались бы
        // навсегда, а при скрещивании нулевые смещения родителей всегда дают
        // нулевое смещение потомка
        for bias in net.b1.iter_mut() {
            *bias = uniform(&mut rng, hidden_limit);
        }

        for bias in net.b2.iter_mut() {
            *bias = uniform(&mut rng, output_limit);
        }

        net
    }

    /// Число весов сети
    pub fn weight_count(&self) -> usize {
        self.w1.len() + self.b1.len() + self.w2.len() + self.b2.len()
    }

    /// Проверяет, что размеры весов согласованы с шириной сети
    pub fn check(&self) -> Result<(), String> {
        let expected_w1 = self.hidden * FEATURE_COUNT;
        let expected_w2 = OUTPUT_COUNT * self.hidden;

        if self.w1.len() != expected_w1 {
            return Err(format!(
                "веса скрытого слоя: ожидалось {expected_w1}, а их {}",
                self.w1.len()
            ));
        }

        if self.b1.len() != self.hidden {
            return Err(format!(
                "смещения скрытого слоя: ожидалось {}, а их {}",
                self.hidden,
                self.b1.len()
            ));
        }

        if self.w2.len() != expected_w2 {
            return Err(format!(
                "веса выходного слоя: ожидалось {expected_w2}, а их {}",
                self.w2.len()
            ));
        }

        if self.b2.len() != OUTPUT_COUNT {
            return Err(format!(
                "смещения выходного слоя: ожидалось {OUTPUT_COUNT}, а их {}",
                self.b2.len()
            ));
        }

        Ok(())
    }

    /// Прямой проход: признаки на входе, вероятности действий на выходе
    pub fn forward(&self, input: &[f32; FEATURE_COUNT]) -> Forward {
        let hidden: Vec<f32> = (0..self.hidden)
            .map(|i| {
                let sum: f32 = (0..FEATURE_COUNT)
                    .map(|j| self.w1[i * FEATURE_COUNT + j] * input[j])
                    .sum::<f32>()
                    + self.b1[i];

                sum.tanh()
            })
            .collect();

        let logits: Vec<f32> = (0..OUTPUT_COUNT)
            .map(|j| {
                let sum: f32 = (0..self.hidden)
                    .map(|i| self.w2[j * self.hidden + i] * hidden[i])
                    .sum::<f32>()
                    + self.b2[j];

                sum
            })
            .collect();

        // Вероятности нормируются внутри каждой группы отдельно
        let mut probs = Vec::with_capacity(OUTPUT_COUNT);
        probs.extend_from_slice(&softmax(&logits[MOVE_START..TURN_START]));
        probs.extend_from_slice(&softmax(&logits[TURN_START..SWORD_START]));
        probs.extend_from_slice(&softmax(&logits[SWORD_START..]));

        Forward {
            input: *input,
            hidden,
            logits,
            probs,
        }
    }
}

/// Случайное число из промежутка `[-limit, limit]`
fn uniform(rng: &mut Rng, limit: f32) -> f32 {
    (rng.f32() * 2. - 1.) * limit
}

/// Вероятности из логитов: сдвиг на максимум защищает от переполнения
fn softmax(logits: &[f32]) -> Vec<f32> {
    if logits.is_empty() {
        return Vec::new();
    }

    let max = logits.iter().copied().fold(f32::NEG_INFINITY, f32::max);

    let exps: Vec<f32> = logits.iter().map(|logit| (logit - max).exp()).collect();
    let sum: f32 = exps.iter().sum();

    if sum <= 0. || !sum.is_finite() {
        // Вырожденное распределение: делим поровну
        let share = 1. / logits.len() as f32;

        return vec![share; logits.len()];
    }

    exps.iter().map(|exp| exp / sum).collect()
}

/// Выбор индекса из распределения
///
/// Индекс выбирается пропорционально вероятности: так поведение сети
/// стохастично, иначе учиться нечему.
pub fn sample_index(probs: &[f32], rng: &mut Rng) -> usize {
    if probs.is_empty() {
        return 0;
    }

    let total: f32 = probs.iter().sum();

    if total <= 0. || !total.is_finite() {
        return rng.usize(0..probs.len());
    }

    let mut pick = rng.f32() * total;

    for (index, prob) in probs.iter().enumerate() {
        pick -= prob;

        if pick <= 0. {
            return index;
        }
    }

    probs.len() - 1
}

/// Индекс с наибольшей вероятностью
pub fn argmax(probs: &[f32]) -> usize {
    let mut best = 0;
    let mut best_value = f32::NEG_INFINITY;

    for (index, prob) in probs.iter().enumerate() {
        if *prob > best_value {
            best_value = *prob;
            best = index;
        }
    }

    best
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math::approx_eq;

    /// Нулевая сеть: у каждой группы равномерное распределение
    #[test]
    fn zero_weights_give_uniform_distribution() {
        let net = Net::new(DEFAULT_HIDDEN);
        let forward = net.forward(&[0.; FEATURE_COUNT]);

        for group in [
            forward.move_probs(),
            forward.turn_probs(),
            forward.sword_probs(),
        ] {
            let sum: f32 = group.iter().sum();
            assert!(approx_eq(sum, 1.), "сумма вероятностей: {sum}");

            for prob in group {
                assert!(
                    approx_eq(*prob, 1. / group.len() as f32),
                    "вероятность не равномерна: {prob}"
                );
            }
        }
    }

    /// Прямой проход даёт нужные размеры
    #[test]
    fn forward_has_expected_sizes() {
        let net = Net::random(DEFAULT_HIDDEN, 7);
        let forward = net.forward(&[0.1; FEATURE_COUNT]);

        assert_eq!(forward.hidden.len(), DEFAULT_HIDDEN);
        assert_eq!(forward.logits.len(), OUTPUT_COUNT);
        assert_eq!(forward.probs.len(), OUTPUT_COUNT);
        assert_eq!(forward.input.len(), FEATURE_COUNT);
    }

    /// Скрытый слой ограничен единицей по модулю
    #[test]
    fn hidden_activations_are_bounded() {
        let net = Net::random(DEFAULT_HIDDEN, 3);
        let forward = net.forward(&[1.; FEATURE_COUNT]);

        for value in forward.hidden {
            assert!(value.abs() <= 1., "активация вне (-1, 1): {value}");
        }
    }

    /// Вероятности в группах нормированы
    #[test]
    fn probabilities_are_normalized_per_group() {
        let net = Net::random(DEFAULT_HIDDEN, 11);
        let forward = net.forward(&[0.3, 0.5, -0.2, 0.9, 0.1, 0., 0.4, -0.6, 1., 0.5, 0.8, 0.3]);

        for group in [
            forward.move_probs(),
            forward.turn_probs(),
            forward.sword_probs(),
        ] {
            let sum: f32 = group.iter().sum();
            assert!(approx_eq(sum, 1.), "сумма вероятностей: {sum}");
        }
    }

    /// Одинаковое зерно даёт одинаковую сеть
    #[test]
    fn same_seed_gives_same_net() {
        let first = Net::random(DEFAULT_HIDDEN, 42);
        let second = Net::random(DEFAULT_HIDDEN, 42);

        assert_eq!(first, second);
        // 16·12 весов скрытого слоя, 16 смещений, 14·16 весов выходного, 14 смещений
        assert_eq!(first.weight_count(), 446);
    }

    /// Разные зерна дают разные сети
    #[test]
    fn different_seeds_give_different_nets() {
        let first = Net::random(DEFAULT_HIDDEN, 1);
        let second = Net::random(DEFAULT_HIDDEN, 2);

        assert_ne!(first, second);
    }

    /// Смещения случайной сети не нулевые: иначе при скрещивании они
    /// никогда не оживают
    #[test]
    fn random_net_has_nonzero_biases() {
        let net = Net::random(DEFAULT_HIDDEN, 13);

        assert!(
            net.b1.iter().any(|bias| *bias != 0.),
            "смещения скрытого слоя нулевые"
        );
        assert!(
            net.b2.iter().any(|bias| *bias != 0.),
            "смещения выходного слоя нулевые"
        );
    }

    /// Проверка размеров весов ловит испорченную сеть
    #[test]
    fn check_rejects_broken_net() {
        let mut net = Net::new(DEFAULT_HIDDEN);
        net.w1.pop();

        assert!(net.check().is_err());
    }

    /// Выбор из распределения попадает в его границы
    #[test]
    fn sampling_stays_in_range() {
        let mut rng = Rng::with_seed(5);
        let probs = [0.1, 0.2, 0.3, 0.4];

        for _ in 0..100 {
            let index = sample_index(&probs, &mut rng);
            assert!(index < probs.len());
        }
    }

    /// Выбор из вырожденного распределения не падает
    #[test]
    fn sampling_survives_degenerate_distribution() {
        let mut rng = Rng::with_seed(6);

        assert_eq!(sample_index(&[], &mut rng), 0);
        assert!(sample_index(&[0., 0., 0.], &mut rng) < 3);
    }

    /// Argmax находит индекс с наибольшей вероятностью
    #[test]
    fn argmax_finds_biggest_probability() {
        assert_eq!(argmax(&[0.1, 0.5, 0.2]), 1);
        assert_eq!(argmax(&[0.9, 0.1]), 0);
    }
}
