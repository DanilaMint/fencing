//! Признаки обстановки: восемь чисел, которые сеть видит на входе.
//!
//! Пять первых признаков — геометрия: расстояния и углы, нормированные
//! к своим диапазонам, как их нормирует стимул стратега.
//! Три последних — динамика: скорости бойцов. Стратег их не смотрит,
//! а для сети это самая быстро меняющаяся часть обстановки.
//!
//! Абсолютное положение бойца в мире сюда не входит: зеркальные положения
//! с одинаковым взаимным расположением для бойца равнозначны.

use serde::{Deserialize, Serialize};

use fencing_core::{
    fighter::{MAX_MOVE_SPEED, MAX_TURN_SPEED},
    math::PI,
    round::situation::Situation,
};

/// Число признаков на входе сети
pub const FEATURE_COUNT: usize = 12;

/// Максимальная скорость сближения бойцов: каждый боец не быстрее
/// `MAX_MOVE_SPEED`, поэтому навстречу друг другу они сходятся не быстрее
/// удвоенной скорости
pub const MAX_APPROACH_SPEED: f32 = 2. * MAX_MOVE_SPEED;

/// Признаки обстановки, нормированные к своим диапазонам.
#[derive(Clone, Copy, PartialEq, Debug, Default, Serialize, Deserialize)]
pub struct Features {
    /// Расстояние бойца от центра ринга (0..1)
    pub center_distance: f32,
    /// Расстояние до противника (0..1)
    pub enemy_distance: f32,
    /// Угол между взглядом и противником (-1..1)
    pub to_enemy_angle: f32,
    /// Угол под которым противник видит бойца (-1..1)
    pub from_enemy_angle: f32,
    /// Угол между взглядом и центром ринга (-1..1)
    pub facing_delta: f32,
    /// Собственная угловая скорость (-1..1)
    pub angle_velocity: f32,
    /// Угловая скорость противника (-1..1)
    pub enemy_angle_velocity: f32,
    /// Скорость сближения бойцов (-1..1)
    pub approach_velocity: f32,
    /// Готов ли свой меч к удару: 1 — да, 0 — клинок ещё в ходе удара
    pub sword_ready: f32,
    /// Своя энергия (0..1)
    pub energy: f32,
    /// Своя выносливость (0..1)
    pub stamina: f32,
    /// Расстояние противника от центра ринга (0..1)
    pub enemy_center_distance: f32,
}

impl Features {
    /// Признаки по текущей ситуации
    pub fn from_situation(situation: &Situation) -> Self {
        Self {
            center_distance: situation.center_distance,
            // Расстояние между бойцами достигает двух радиусов ринга
            enemy_distance: situation.enemy_distance / 2.,
            to_enemy_angle: situation.to_enemy_angle / PI,
            from_enemy_angle: situation.from_enemy_angle / PI,
            facing_delta: situation.facing_delta / PI,
            angle_velocity: situation.angle_velocity / MAX_TURN_SPEED,
            enemy_angle_velocity: situation.enemy_angle_velocity / MAX_TURN_SPEED,
            approach_velocity: situation.approach_velocity / MAX_APPROACH_SPEED,
            // Свой меч, энергия и выносливость: без них сеть не может
            // ни поймать момент для удара, ни сэкономить силы
            sword_ready: if situation.sword_ready { 1. } else { 0. },
            energy: situation.energy,
            stamina: situation.stamina,
            // Положение противника относительно центра: по нему видно,
            // можно ли вытолкнуть его с ринга
            enemy_center_distance: situation.enemy_center_distance,
        }
    }

    /// Признаки в порядке, в котором их ждёт сеть
    pub fn as_array(&self) -> [f32; FEATURE_COUNT] {
        [
            self.center_distance,
            self.enemy_distance,
            self.to_enemy_angle,
            self.from_enemy_angle,
            self.facing_delta,
            self.angle_velocity,
            self.enemy_angle_velocity,
            self.approach_velocity,
            self.sword_ready,
            self.energy,
            self.stamina,
            self.enemy_center_distance,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fencing_core::math::approx_eq;

    /// Признаки берутся из ситуации и нормируются
    #[test]
    fn features_take_values_from_situation() {
        let situation = Situation {
            center_distance: 0.25,
            enemy_distance: 1.,
            to_enemy_angle: PI / 2.,
            from_enemy_angle: -PI / 4.,
            facing_delta: PI / 3.,
            ..Default::default()
        };

        let features = Features::from_situation(&situation);

        assert!(approx_eq(features.center_distance, 0.25));
        // Расстояние нормируется к единице: два радиуса ринга — это единица
        assert!(approx_eq(features.enemy_distance, 0.5));
        assert!(approx_eq(features.to_enemy_angle, 0.5));
        assert!(approx_eq(features.from_enemy_angle, -0.25));
        assert!(approx_eq(features.facing_delta, 1. / 3.));
    }

    /// Скорости нормируются к своим пределам
    #[test]
    fn speeds_are_normalized() {
        let situation = Situation {
            angle_velocity: MAX_TURN_SPEED,
            enemy_angle_velocity: -MAX_TURN_SPEED,
            approach_velocity: MAX_APPROACH_SPEED,
            ..Default::default()
        };

        let features = Features::from_situation(&situation);

        assert!(approx_eq(features.angle_velocity, 1.));
        assert!(approx_eq(features.enemy_angle_velocity, -1.));
        assert!(approx_eq(features.approach_velocity, 1.));
    }

    /// Пустая ситуация даёт конечные признаки
    #[test]
    fn empty_situation_gives_finite_features() {
        let features = Features::from_situation(&Situation::default());

        for value in features.as_array() {
            assert!(value.is_finite(), "признак не конечен: {value}");
        }
    }

    /// Признаки укладываются в заявленные диапазоны
    #[test]
    fn features_stay_in_range() {
        let situation = Situation {
            center_distance: 1.,
            enemy_distance: 2.,
            to_enemy_angle: PI,
            from_enemy_angle: -PI,
            facing_delta: PI,
            angle_velocity: MAX_TURN_SPEED,
            enemy_angle_velocity: -MAX_TURN_SPEED,
            approach_velocity: -MAX_APPROACH_SPEED,
            ..Default::default()
        };

        let features = Features::from_situation(&situation);

        assert!(approx_eq(features.enemy_distance, 1.));
        assert!(approx_eq(features.to_enemy_angle, 1.));
        assert!(approx_eq(features.from_enemy_angle, -1.));
        assert!(approx_eq(features.facing_delta, 1.));
        assert!(approx_eq(features.approach_velocity, -1.));
    }

    /// Порядок признаков в массиве совпадает с порядком полей
    #[test]
    fn array_order_matches_fields() {
        let features = Features {
            center_distance: 0.1,
            enemy_distance: 0.2,
            to_enemy_angle: 0.3,
            from_enemy_angle: 0.4,
            facing_delta: 0.5,
            angle_velocity: 0.6,
            enemy_angle_velocity: 0.7,
            approach_velocity: 0.8,
            sword_ready: 0.9,
            energy: 0.6,
            stamina: 0.5,
            enemy_center_distance: 0.4,
        };

        let array = features.as_array();

        assert_eq!(array.len(), FEATURE_COUNT);
        assert!(approx_eq(array[0], 0.1));
        assert!(approx_eq(array[4], 0.5));
        assert!(approx_eq(array[7], 0.8));
        assert!(approx_eq(array[8], 0.9));
        assert!(approx_eq(array[9], 0.6));
        assert!(approx_eq(array[10], 0.5));
        assert!(approx_eq(array[11], 0.4));
    }

    /// Готовность меча и состояние бойца попадают в признаки
    #[test]
    fn own_state_reaches_features() {
        let situation = Situation {
            sword_ready: true,
            energy: 0.4,
            stamina: 0.7,
            enemy_center_distance: 0.5,
            ..Default::default()
        };

        let features = Features::from_situation(&situation);

        assert!(approx_eq(features.sword_ready, 1.));
        assert!(approx_eq(features.energy, 0.4));
        assert!(approx_eq(features.stamina, 0.7));
        assert!(approx_eq(features.enemy_center_distance, 0.5));
    }
}
