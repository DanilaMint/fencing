use crate::{approach::approach_fighters, fighter::Fighter, math::normalize_angle};

pub struct Situation {
    /// Дистанция между бойцом и центром ринга (0..1)
    pub center_distance: f32,
    /// Угол между направлением взгляда и лучом от бойца в центр ринга (-PI..PI)
    pub facing_delta: f32,
    /// Расстояние между бойцами (0..2)
    pub enemy_distance: f32,
    /// Угол между направлением взгляда и отрезком между бойцами (-PI..PI)
    pub to_enemy_angle: f32,
    /// Угол между направлением взгляда оппонента и отрезком между бойцами (-PI..PI)
    pub from_enemy_angle: f32,
    /// Текущая угловая скорость
    pub angle_velocity: f32,
    /// Текущая угловая скорость оппонента
    pub enemy_angle_velocity: f32,
    /// Скорость сближения бойцов
    pub approach_velocity: f32,
}

pub fn analyze_situation(subject: &Fighter, object: &Fighter, ring_radius: f32) -> Situation {
    let position_atan = subject.position().to_angle();
    let enemy_vector = object.position() - subject.position();

    let center_distance = subject.position().length() / ring_radius;

    let facing_delta = normalize_angle((subject.facing()) + position_atan);

    let enemy_distance = enemy_vector.length() / ring_radius;

    let to_enemy_angle = normalize_angle(enemy_vector.to_angle() - facing_delta);

    let from_enemy_angle = normalize_angle(-enemy_vector.to_angle() - object.facing());

    let angle_velocity = subject.angle_speed();

    let enemy_angle_velocity = object.angle_speed();

    let approach_velocity = approach_fighters(subject, object);

    Situation {
        center_distance,
        facing_delta,
        enemy_distance,
        to_enemy_angle,
        from_enemy_angle,
        angle_velocity,
        approach_velocity,
        enemy_angle_velocity,
    }
}
