use crate::{
    approach::approach_fighters,
    fighter::Fighter,
    math::{PI, TAU, normalize_angle},
};
use glam::Vec2;

#[derive(Clone, Copy, Default)]
pub struct Situation {
    /// Позиция бойца в мире
    pub position: Vec2,
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
    /// Дистанция между противником и центром ринга (0..1)
    pub enemy_center_distance: f32,
    /// Готов ли меч бойца к удару: клинок в стойке
    pub sword_ready: bool,
    /// Энергия бойца (0..1)
    pub energy: f32,
    /// Выносливость бойца (0..1)
    pub stamina: f32,
}

pub fn analyze_situation(subject: &Fighter, object: &Fighter, ring_radius: f32) -> Situation {
    let position_atan = subject.position().to_angle();
    let enemy_vector = object.position() - subject.position();

    let center_distance = subject.position().length() / ring_radius;

    // Луч от бойца в центр ринга направлен противоположно его положению
    let facing_delta = normalize_angle(subject.facing() - position_atan - PI);

    let enemy_distance = enemy_vector.length() / ring_radius;

    // Угол между взглядом бойца и отрезком к противнику
    let to_enemy_angle = normalize_angle(enemy_vector.to_angle() - subject.facing());

    // Угол между взглядом противника и отрезком к нему
    let from_enemy_angle = normalize_angle(enemy_vector.to_angle() + PI - object.facing());

    let angle_velocity = subject.angle_speed();

    let enemy_angle_velocity = object.angle_speed();

    let approach_velocity = approach_fighters(subject, object);

    // Расстояние противника от центра ринга: по нему видно, отодвинут ли
    // противник от центра
    let enemy_center_distance = object.position().length() / ring_radius;

    Situation {
        position: subject.position(),
        center_distance,
        facing_delta,
        enemy_distance,
        to_enemy_angle,
        from_enemy_angle,
        angle_velocity,
        approach_velocity,
        enemy_angle_velocity,
        enemy_center_distance,
        sword_ready: subject.is_sword_ready(),
        energy: subject.energy(),
        stamina: subject.stamina(),
    }
}

pub fn compare_situations(s1: &Situation, s2: &Situation) -> f32 {
    (s1.center_distance - s2.center_distance).powi(2)
        + ((s1.facing_delta - s2.facing_delta) / TAU).powi(2)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        brain::doll::Doll,
        fighter::Fighter,
        math::approx_eq,
    };
    use glam::Vec2;

    /// Радиус ринга для тестов, мировые единицы
    const RING_RADIUS: f32 = 5.;

    /// Боец, смотрящий точно на противника, имеет нулевой угол до него
    #[test]
    fn to_enemy_angle_is_zero_when_looking_at_enemy() {
        let mut subject = Fighter::new(Box::new(Doll));
        let mut object = Fighter::new(Box::new(Doll));

        subject.set_position(Vec2::ZERO);
        object.set_position(Vec2::new(2., 0.));

        // Взгляд направлен вправо, на противника
        subject.rotate(-PI / 2.);

        let situation = analyze_situation(&subject, &object, RING_RADIUS);

        assert!(approx_eq(situation.to_enemy_angle, 0.));
        assert!(approx_eq(situation.enemy_distance, 2. / RING_RADIUS));
    }

    /// Боец на краю ринга, смотрящий в центр, имеет нулевой угол до центра
    #[test]
    fn facing_delta_is_zero_when_looking_at_center() {
        let mut subject = Fighter::new(Box::new(Doll));
        let object = Fighter::new(Box::new(Doll));

        subject.set_position(Vec2::new(3., 0.));

        // Взгляд направлен влево, в центр ринга
        subject.rotate(PI / 2.);

        let situation = analyze_situation(&subject, &object, RING_RADIUS);

        assert!(approx_eq(situation.facing_delta, 0.));
        assert!(approx_eq(situation.center_distance, 3. / RING_RADIUS));
    }

    /// Угол, под которым противник видит бойца, совпадает с его углом до бойца
    #[test]
    fn from_enemy_angle_is_opponents_view() {
        let mut subject = Fighter::new(Box::new(Doll));
        let mut object = Fighter::new(Box::new(Doll));

        subject.set_position(Vec2::new(-1., 0.5));
        object.set_position(Vec2::new(1.5, -0.5));

        subject.rotate(0.4);
        object.rotate(-1.1);

        let subject_situation = analyze_situation(&subject, &object, RING_RADIUS);
        let object_situation = analyze_situation(&object, &subject, RING_RADIUS);

        assert!(approx_eq(
            subject_situation.from_enemy_angle,
            object_situation.to_enemy_angle
        ));
    }
}
