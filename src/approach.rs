use glam::Vec2;

use crate::{fighter::Fighter, math::*};

/// Возвращает скорость сближения бойцов в мировых единицах в секунду
pub fn approach_fighters(f1: &Fighter, f2: &Fighter) -> f32 {
    let r = f2.position() - f1.position();
    let dist = r.length();

    let vel_r = f2.velocity() - f1.velocity();

    r.dot(vel_r) / dist
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

    r.dot(vel_r) / dist
}
