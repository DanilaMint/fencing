//! Стимул: признаки обстановки, по которым подбирается реакция.

use serde::{Deserialize, Serialize};

use fencing_core::{
    math::{PI, normalize_angle},
    round::situation::Situation,
};

/// Стимул: снимок обстановки, в которой реакция была найдена или создана.
///
/// Все признаки нормированы к своему диапазону: расстояния — к единице,
/// углы — к `PI`. Тогда разности признаков сравнимы между собой.
///
/// Абсолютное положение бойца в мире сюда не входит: зеркальные положения
/// с одинаковым взаимным расположением для бойца равнозначны.
#[derive(Clone, Copy, PartialEq, Debug, Default, Serialize, Deserialize)]
pub struct Stimulus {
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
}

impl Stimulus {
    /// Стимул по текущей ситуации
    pub fn from_situation(situation: &Situation) -> Self {
        Self {
            center_distance: situation.center_distance,
            // Расстояние между бойцами достигает двух радиусов ринга
            enemy_distance: situation.enemy_distance / 2.,
            to_enemy_angle: situation.to_enemy_angle / PI,
            from_enemy_angle: situation.from_enemy_angle / PI,
            facing_delta: situation.facing_delta / PI,
        }
    }

    /// Число похожести: ноль — полное совпадение, больше — меньше похоже.
    ///
    /// Сумма квадратов разностей признаков. Углы сравниваются с учётом
    /// перехода через `PI`: `+PI` и `-PI` — это один и тот же ракурс.
    pub fn similarity(&self, other: &Self) -> f32 {
        let center = self.center_distance - other.center_distance;
        let enemy = self.enemy_distance - other.enemy_distance;
        let to_enemy = angle_diff(self.to_enemy_angle, other.to_enemy_angle);
        let from_enemy = angle_diff(self.from_enemy_angle, other.from_enemy_angle);
        let facing = angle_diff(self.facing_delta, other.facing_delta);

        center * center
            + enemy * enemy
            + to_enemy * to_enemy
            + from_enemy * from_enemy
            + facing * facing
    }
}

/// Разница нормированных углов с учётом перехода через `PI`
fn angle_diff(a: f32, b: f32) -> f32 {
    normalize_angle((a - b) * PI) / PI
}

#[cfg(test)]
mod tests {
    use super::*;
    use fencing_core::math::approx_eq;

    /// Одинаковые стимулы полностью совпадают
    #[test]
    fn identical_stimuli_have_zero_similarity() {
        let stimulus = Stimulus {
            center_distance: 0.3,
            enemy_distance: 0.5,
            to_enemy_angle: 0.2,
            from_enemy_angle: -0.4,
            facing_delta: 0.1,
        };

        assert!(approx_eq(stimulus.similarity(&stimulus), 0.));
    }

    /// Чем дальше стимулы друг от друга, тем меньше похожи
    #[test]
    fn similarity_grows_with_difference() {
        let base = Stimulus {
            center_distance: 0.3,
            enemy_distance: 0.5,
            to_enemy_angle: 0.,
            from_enemy_angle: 0.,
            facing_delta: 0.,
        };

        let near = Stimulus {
            enemy_distance: 0.6,
            ..base
        };
        let far = Stimulus {
            enemy_distance: 0.9,
            ..base
        };

        assert!(base.similarity(&near) > 0.);
        assert!(near.similarity(&far) > near.similarity(&base));
    }

    /// Углы сравниваются через переход: +PI и -PI — один и тот же ракурс
    #[test]
    fn angles_are_compared_through_pi_wrap() {
        let a = Stimulus {
            to_enemy_angle: 1.,
            ..Default::default()
        };
        let b = Stimulus {
            to_enemy_angle: -1.,
            ..Default::default()
        };

        assert!(approx_eq(a.similarity(&b), 0.));
    }

    /// Стимул берёт признаки из ситуации и нормирует их
    #[test]
    fn stimulus_takes_features_from_situation() {
        let situation = Situation {
            center_distance: 0.25,
            enemy_distance: 1.,
            to_enemy_angle: PI / 2.,
            from_enemy_angle: -PI / 4.,
            facing_delta: PI / 3.,
            ..Default::default()
        };

        let stimulus = Stimulus::from_situation(&situation);

        assert!(approx_eq(stimulus.center_distance, 0.25));
        // Расстояние нормируется к единице: два радиуса ринга — это единица
        assert!(approx_eq(stimulus.enemy_distance, 0.5));
        assert!(approx_eq(stimulus.to_enemy_angle, 0.5));
        assert!(approx_eq(stimulus.from_enemy_angle, -0.25));
    }
}
