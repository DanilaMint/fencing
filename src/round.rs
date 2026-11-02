use crate::{
    fighter::Fighter,
    math::{PI, TAU, normalize_angle},
};

pub struct Round {
    red: Fighter,
    blue: Fighter,

    time: u32,
}

pub struct Situation {
    /// Дистанция между бойцом и центром ринга (0..1)
    pub center_distance: f32,
    /// Угол между направлением взгляда и лучом от бойца в центр ринга (-PI..PI)
    pub facing_delta: f32,
    /// Расстояние между бойцами (0..2)
    pub enemy_distance: f32,
    /// Угол между направлением взгляда и отрезком между бойцами (-PI..PI)
    pub enemy_angle: f32,
}

fn analyze_situation(subject: &Fighter, object: &Fighter, ring_radius: f32) {
    let position_atan = subject.position().to_angle();
    let enemy_vector = object.position() - subject.position();

    // Дистанция от центра ринга
    let center_distance = subject.position().length() / ring_radius;

    let facing_delta = normalize_angle((subject.facing() + 90.0f32.to_radians()) + position_atan);

    let enemy_distance = enemy_vector.length() / ring_radius;

    let enemy_angle = normalize_angle(enemy_vector.to_angle() - facing_delta);
}
