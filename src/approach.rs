use glam::Vec2;

use crate::fighter::Fighter;

/// Возвращает скорость сближения бойцов в мировых единицах в секунду
pub fn approach_fighters(f1: &Fighter, f2: &Fighter) -> f32 {
    // r — вектор от первого бойца ко второму
    let r = f2.position() - f1.position();

    let vel_r = f2.velocity() - f1.velocity();

    approach(r, vel_r)
}

/// Возвращает скорость сближения острия меча и тела оппонента в мировых единицах в секунду
pub fn approach_sword_and_fighter(sword_f: &Fighter, f2: &Fighter) -> f32 {
    // r — вектор от острия меча к телу оппонента
    let r = f2.position() - sword_f.sword_position().1;

    let vel_r = f2.velocity() - sword_f.sword_peak_velocity();

    approach(r, vel_r)
}

/// Возвращает скорость сближения острия меча и головы оппонента в мировых единицах в секунду
pub fn approach_sword_and_head(sword_f: &Fighter, f2: &Fighter) -> f32 {
    let r = f2.head_position() - sword_f.sword_position().1;

    let vel_r = f2.head_velocity() - sword_f.sword_peak_velocity();

    approach(r, vel_r)
}

/// Возвращает скорость сближения острия меча и левой руки оппонента в мировых единицах в секунду
pub fn approach_sword_and_fist(sword_f: &Fighter, f2: &Fighter) -> f32 {
    let r = f2.fist_position() - sword_f.sword_position().1;

    let vel_r = f2.fist_velocity() - sword_f.sword_peak_velocity();

    approach(r, vel_r)
}

/// Возвращает скорость сближения двух точек: положительное значение означает сближение
fn approach(r: Vec2, vel_r: Vec2) -> f32 {
    let dist = r.length();

    if dist < f32::EPSILON {
        return 0.;
    }

    // r направлен от источника к цели, поэтому рост проекции скорости означает расхождение
    -(r.dot(vel_r) / dist)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        fighter::{
            Fighter,
            action::{Action, Direction},
            brain::doll::Doll,
        },
        math::PI,
    };

    /// Боец, идущий навстречу противнику, даёт положительную скорость сближения
    #[test]
    fn approaching_fighters_have_positive_speed() {
        let mut f1 = Fighter::new(Box::new(Doll));
        let mut f2 = Fighter::new(Box::new(Doll));

        f1.set_position(Vec2::new(-1., 0.));
        f2.set_position(Vec2::new(1., 0.));

        // f1 смотрит вправо, f2 — влево
        f1.rotate(-PI / 2.);
        f2.rotate(PI / 2.);

        let mut action = Action::default();
        action.forward = Direction::Positive;
        f1.do_actions(action);

        let mut action = Action::default();
        action.forward = Direction::Positive;
        f2.do_actions(action);

        assert!(approach_fighters(&f1, &f2) > 0.);
    }

    /// Расходящиеся бойцы дают отрицательную скорость сближения
    #[test]
    fn separating_fighters_have_negative_speed() {
        let mut f1 = Fighter::new(Box::new(Doll));
        let mut f2 = Fighter::new(Box::new(Doll));

        f1.set_position(Vec2::new(-1., 0.));
        f2.set_position(Vec2::new(1., 0.));

        // f1 смотрит влево, f2 — вправо
        f1.rotate(PI / 2.);
        f2.rotate(-PI / 2.);

        let mut action = Action::default();
        action.forward = Direction::Positive;
        f1.do_actions(action);

        let mut action = Action::default();
        action.forward = Direction::Positive;
        f2.do_actions(action);

        assert!(approach_fighters(&f1, &f2) < 0.);
    }

    /// Совпадающие позиции не дают NaN
    #[test]
    fn zero_distance_gives_finite_speed() {
        let f1 = Fighter::new(Box::new(Doll));
        let f2 = Fighter::new(Box::new(Doll));

        let speed = approach_fighters(&f1, &f2);

        assert!(speed.is_finite());
        assert_eq!(speed, 0.);
    }
}
