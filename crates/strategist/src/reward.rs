//! Результат реакции: насколько выгодной она оказалась.
//!
//! Результат складывается из двух частей: событий боя (нанесённый и
//! полученный удар, отбитый удар, шок, потраченная энергия) и изменения
//! положения на ринге.

use fencing_core::brain::BrainEvent;

/// Награда за единицу силы нанесённого удара
pub const HIT_DEALT_WEIGHT: f32 = 2.0;
/// Штраф за единицу силы полученного удара
pub const HIT_TAKEN_WEIGHT: f32 = 1.5;
/// Награда за отбитый удар противника
pub const PARRIED_BONUS: f32 = 0.5;
/// Штраф за удар в голову
pub const SHOCK_PENALTY: f32 = 1.0;
/// Штраф за потраченную впустую энергию, за единицу энергии
pub const ENERGY_WASTED_WEIGHT: f32 = 2.0;
/// Штраф за отход от центра ринга, за единицу расстояния
pub const LEFT_CENTER_WEIGHT: f32 = 2.0;
/// Награда за отталкивание противника от центра, за единицу расстояния
pub const PUSHED_ENEMY_WEIGHT: f32 = 2.0;

/// Накопитель результата реакции
#[derive(Clone, Copy, Default, Debug)]
pub struct Reward {
    /// Вклад событий боя
    events: f32,
}

impl Reward {
    /// Пустой накопитель: событий ещё не было
    pub fn new() -> Self {
        Self::default()
    }

    /// Учитывает событие боя
    pub fn add_event(&mut self, event: BrainEvent) {
        self.events += match event {
            // Нанесённый удар — выгодное событие
            BrainEvent::HitDealt { power } => HIT_DEALT_WEIGHT * power,
            // Пропущенный удар — невыгодное
            BrainEvent::HitTaken { power } => -HIT_TAKEN_WEIGHT * power,
            // Отбитый удар противника — выгодное
            BrainEvent::Parried => PARRIED_BONUS,
            // Шок: удар в голову оборвал реакцию. Сам урон уже учтён
            // в `HitTaken`, здесь штраф за потерю контроля над телом
            BrainEvent::Shocked => -SHOCK_PENALTY,
            // Потраченная энергия без результата — невыгодное
            BrainEvent::EnergySpent { amount } => -ENERGY_WASTED_WEIGHT * amount,
        };
    }

    /// Итоговая оценка реакции.
    ///
    /// `center_delta` — насколько боец отошёл от центра ринга за время
    /// реакции (положительное значение — отошёл), `enemy_center_delta` —
    /// насколько противник отошёл от центра (положительное — оттолкнут).
    pub fn score(&self, center_delta: f32, enemy_center_delta: f32) -> f32 {
        let mut score = self.events;

        // Отход от центра ринга — проигрышная позиция
        if center_delta > 0. {
            score -= LEFT_CENTER_WEIGHT * center_delta;
        }

        // Противник оттолкнут от центра — выигрышная позиция
        if enemy_center_delta > 0. {
            score += PUSHED_ENEMY_WEIGHT * enemy_center_delta;
        }

        score
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fencing_core::math::approx_eq;

    /// Нанесённый удар повышает оценку
    #[test]
    fn hit_dealt_increases_score() {
        let mut reward = Reward::new();
        reward.add_event(BrainEvent::HitDealt { power: 0.5 });

        assert!(reward.score(0., 0.) > 0.);
    }

    /// Полученный удар снижает оценку
    #[test]
    fn hit_taken_decreases_score() {
        let mut reward = Reward::new();
        reward.add_event(BrainEvent::HitTaken { power: 0.5 });

        assert!(reward.score(0., 0.) < 0.);
    }

    /// Отбитый удар — выгодное событие
    #[test]
    fn parry_gives_bonus() {
        let mut reward = Reward::new();
        reward.add_event(BrainEvent::Parried);

        assert!(approx_eq(reward.score(0., 0.), PARRIED_BONUS));
    }

    /// Шок снижает оценку
    #[test]
    fn shock_decreases_score() {
        let mut reward = Reward::new();
        reward.add_event(BrainEvent::Shocked);

        assert!(reward.score(0., 0.) < 0.);
    }

    /// Потраченная энергия снижает оценку
    #[test]
    fn wasted_energy_decreases_score() {
        let mut reward = Reward::new();
        reward.add_event(BrainEvent::EnergySpent { amount: 0.1 });

        assert!(reward.score(0., 0.) < 0.);
    }

    /// Отход от центра ринга — проигрышная позиция
    #[test]
    fn leaving_center_decreases_score() {
        let reward = Reward::new();

        assert!(reward.score(0.2, 0.) < 0.);
        // Приближение к центру проигрышем не считается
        assert!(approx_eq(reward.score(-0.2, 0.), 0.));
    }

    /// Отталкивание противника от центра — выигрышная позиция
    #[test]
    fn pushing_enemy_from_center_increases_score() {
        let reward = Reward::new();

        assert!(reward.score(0., 0.2) > 0.);
    }

    /// События складываются
    #[test]
    fn events_accumulate() {
        let mut reward = Reward::new();
        reward.add_event(BrainEvent::HitDealt { power: 0.5 });
        reward.add_event(BrainEvent::Parried);

        let expected = HIT_DEALT_WEIGHT * 0.5 + PARRIED_BONUS;
        assert!(approx_eq(reward.score(0., 0.), expected));
    }
}
