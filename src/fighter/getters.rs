use glam::{Mat2, Vec2};

use crate::fighter::{
    FACING_ANGLE_OFFSET, FIST_LOCAL_OFFSET, Fighter, HEAD_LOCAL_OFFSET, SWORD_LOCAL_OFFSET,
    TORSO_HEIGHT, TORSO_WIDTH,
    brain::Brain,
    sword::{BLADE_LENGTH, Sword},
};

impl Fighter {
    /// Возвращает позиции рукояти меча и его острия
    pub fn sword_position(&self) -> (Vec2, Vec2) {
        let pos = self.facing_to_matrix()
            * (SWORD_LOCAL_OFFSET + self.sword.local_sword_position())
            + self.position;
        let peak = pos
            + Vec2::from_angle(self.sword.angle() + self.facing + FACING_ANGLE_OFFSET)
                * BLADE_LENGTH;

        (pos, peak)
    }

    pub fn head_position(&self) -> Vec2 {
        self.position + self.facing_to_matrix() * HEAD_LOCAL_OFFSET
    }

    pub fn fist_position(&self) -> Vec2 {
        self.position + self.facing_to_matrix() * FIST_LOCAL_OFFSET
    }

    pub fn torso_position(&self) -> Vec2 {
        self.position
    }

    /// Угол поворота тела бойца в мировой системе координат.
    /// В отличие от `facing()`, отсчитывается не от взгляда вправо,
    /// а от взгляда вверх: нулю соответствует направление на север.
    pub fn body_angle(&self) -> f32 {
        self.facing
    }

    /// Опорная функция эллипса туловища: длина проекции туловища
    /// на направление `dir`. Используется, чтобы раздвигать бойцов
    /// так, чтобы их туловища не перекрывались.
    pub fn torso_support_along(&self, dir: Vec2) -> f32 {
        let local = self.facing_to_matrix().transpose() * dir;

        ((TORSO_WIDTH * local.x).powi(2) + (TORSO_HEIGHT * local.y).powi(2)).sqrt()
    }

    pub fn position(&self) -> Vec2 {
        self.position
    }

    pub fn facing(&self) -> f32 {
        self.facing + FACING_ANGLE_OFFSET
    }

    pub fn velocity(&self) -> Vec2 {
        self.velocity
    }

    /// Угловая скорость (в радианах в секунду)
    pub fn angle_speed(&self) -> f32 {
        self.angle_speed
    }

    /// Скорость точки тела, заданной её смещением в локальной системе координат бойца
    pub fn point_velocity(&self, local: Vec2) -> Vec2 {
        self.velocity + (self.facing_to_matrix() * local.perp()) * self.angle_speed
    }

    /// Скорость головы бойца
    pub fn head_velocity(&self) -> Vec2 {
        self.point_velocity(HEAD_LOCAL_OFFSET)
    }

    /// Скорость левой руки бойца
    pub fn fist_velocity(&self) -> Vec2 {
        self.point_velocity(FIST_LOCAL_OFFSET)
    }

    /// Скорость точки клинка на расстоянии `s` от рукояти вдоль клинка
    /// (0 — рукоять, BLADE_LENGTH — остриё)
    pub fn sword_point_velocity(&self, s: f32) -> Vec2 {
        let matrix = self.facing_to_matrix();

        // Положение рукояти в локальной системе координат
        let hilt_local = SWORD_LOCAL_OFFSET + self.sword.local_sword_position();
        // Направление клинка в мировой системе координат
        let blade = Vec2::from_angle(self.sword.angle() + self.facing());

        self.velocity
            // Поворот бойца вместе с мечом
            + (matrix * hilt_local.perp()) * self.angle_speed
            // Движение меча относительно бойца
            + matrix * self.sword.local_velocity()
            // Поворот клинка вокруг рукояти
            + blade.perp() * (self.sword.angle_velocity() + self.angle_speed) * s
    }

    /// Скорость острия меча бойца
    pub fn sword_peak_velocity(&self) -> Vec2 {
        self.sword_point_velocity(BLADE_LENGTH)
    }

    /// Наносит ли боец удар в данный момент
    pub fn is_sword_attacking(&self) -> bool {
        self.sword.is_attacking()
    }

    /// Готов ли меч к удару: клинок в стойке и не занят
    pub fn is_sword_ready(&self) -> bool {
        self.sword.is_ready()
    }

    /// Выносливость бойца
    pub fn stamina(&self) -> f32 {
        self.stamina
    }

    /// Энергия бойца
    pub fn energy(&self) -> f32 {
        self.energy
    }

    /// Остаток эффекта шока
    pub fn shock(&self) -> f32 {
        self.shock
    }

    pub fn extract_brain(self) -> Box<dyn Brain> {
        self.brain
    }

    /// Матрица поворота локальной системы координат бойца в мировую
    pub(crate) fn facing_to_matrix(&self) -> Mat2 {
        Mat2::from_angle(self.facing)
    }

    pub fn is_sword_retracts(&self) -> bool {
        match self.sword {
            Sword::Retract { .. } => true,
            _ => false,
        }
    }

    /// Отправлен ли боец в нокаут.
    /// Нокаут необратим: даже если выносливость снова вырастет,
    /// боец уже считается нокаутированным.
    pub fn is_knockout(&self) -> bool {
        self.knocked_out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        fighter::{
            FIST_LOCAL_OFFSET, HEAD_LOCAL_OFFSET, SWORD_LOCAL_OFFSET,
            brain::doll::Doll,
            sword::{BLADE_LENGTH, Sword},
        },
        math::{DT, PI},
    };

    fn fighter(velocity: Vec2, angle_speed: f32, facing: f32) -> Fighter {
        let mut f = Fighter::new(Box::new(Doll));
        f.velocity = velocity;
        f.angle_speed = angle_speed;
        f.facing = facing;

        f
    }

    /// Проверяет, что аналитическая скорость точки близка к конечно-разностной
    fn check_point_velocity(f: &Fighter, point: impl Fn(&Fighter) -> Vec2 + Copy, vel: Vec2) {
        // Конечная разность вычисляется по дискретной модели (поворот тела применяется
        // один раз за тик), поэтому расхождение с мгновенной скоростью составляет
        // величину порядка dt * omega * |v| — порядка 0.05
        let numeric = finite_difference(f, point, 1);

        assert!(
            (numeric - vel).length() < 0.2,
            "numeric: {:?}, analytic: {:?}",
            numeric,
            vel
        );
    }

    /// Конечно-разностная производная по положению точки за `ticks` тиков
    fn finite_difference(f: &Fighter, point: impl Fn(&Fighter) -> Vec2, ticks: u32) -> Vec2 {
        let p0 = point(f);

        let mut next = fighter(f.velocity, f.angle_speed, f.facing);
        next.position = f.position + f.velocity * DT * ticks as f32;
        next.facing += f.angle_speed * DT * ticks as f32;
        next.sword = f.sword.clone();

        for _ in 0..ticks {
            next.sword.tick();
        }

        (point(&next) - p0) / (DT * ticks as f32)
    }

    #[test]
    fn head_and_fist_velocity_test() {
        let f = fighter(Vec2::new(1., 2.), 0.7, 0.3);

        check_point_velocity(&f, |x| x.head_position(), f.head_velocity());
        check_point_velocity(&f, |x| x.fist_position(), f.fist_velocity());
    }

    #[test]
    fn sword_peak_velocity_test() {
        for sword in [
            Sword::Stay,
            Sword::Prick {
                dy: 0.5,
                velocity: 3.5,
            },
            Sword::Chok {
                radius: 0.7,
                alpha: -1.0,
                velocity: 2.5,
                angle_velocity: 1.0,
            },
            Sword::Retract {
                alpha: -0.5,
                position: Vec2::new(0.3, 0.6),
            },
        ] {
            let mut f = fighter(Vec2::new(1., 2.), 0.7, 0.3);
            f.sword = sword;

            check_point_velocity(&f, |x| x.sword_position().1, f.sword_peak_velocity());
        }
    }

    /// Скорость острия — частный случай скорости точки клинка
    #[test]
    fn sword_point_velocity_at_blade_length_equals_peak_velocity() {
        for sword in [
            Sword::Stay,
            Sword::Prick {
                dy: 0.5,
                velocity: 3.5,
            },
            Sword::Chok {
                radius: 0.7,
                alpha: -1.0,
                velocity: 2.5,
                angle_velocity: 1.0,
            },
        ] {
            let mut f = fighter(Vec2::new(1., 2.), 0.7, 0.3);
            f.sword = sword;

            assert!(
                (f.sword_point_velocity(BLADE_LENGTH) - f.sword_peak_velocity()).length() < 1e-5
            );
        }
    }

    /// Скорость рукояти совпадает со скоростью её точки на клинке
    #[test]
    fn sword_point_velocity_at_hilt() {
        let f = fighter(Vec2::new(1., 2.), 0.7, 0.3);

        check_point_velocity(&f, |x| x.sword_position().0, f.sword_point_velocity(0.));
    }

    /// Боец наносит удар, пока меч в ударе
    #[test]
    fn is_sword_attacking_test() {
        let mut f = fighter(Vec2::ZERO, 0., 0.);

        assert!(!f.is_sword_attacking());

        f.prick();
        assert!(f.is_sword_attacking());

        // Удар закончился: меч вернулся в стойку
        for _ in 0..200 {
            f.tick();
        }

        assert!(!f.is_sword_attacking());
    }

    #[test]
    fn constants_used() {
        // Чтобы не было неиспользуемых импортов в тестах
        let _ = (
            SWORD_LOCAL_OFFSET,
            HEAD_LOCAL_OFFSET,
            FIST_LOCAL_OFFSET,
            BLADE_LENGTH,
        );
    }

    /// Опорная функция туловища совпадает с полуосями вдоль локальных осей
    #[test]
    fn torso_support_matches_semi_axes() {
        let f = fighter(Vec2::ZERO, 0., 0.);

        // Вдоль локальной оси X — ширина плеч, вдоль локальной оси Y — глубина корпуса
        assert!((f.torso_support_along(Vec2::X) - TORSO_WIDTH).abs() < 1e-5);
        assert!((f.torso_support_along(Vec2::Y) - TORSO_HEIGHT).abs() < 1e-5);

        // Под углом опорная функция лежит между полуосями
        let support = f.torso_support_along(Vec2::from_angle(PI / 4.));

        assert!(support > TORSO_HEIGHT && support < TORSO_WIDTH);
    }

    /// Значения посчитаны вручную по формулам движения точек тела:
    /// v_точки = v + omega * perp(M(facing) * смещение)
    #[test]
    fn reference_values_test() {
        let f = fighter(Vec2::new(1., 2.), 0.7, 0.3);

        assert!((f.head_velocity() - Vec2::new(0.866253, 1.958627)).length() < 1e-4);
        assert!((f.fist_velocity() - Vec2::new(0.909055, 0.872778)).length() < 1e-4);

        // Меч покоится: острие вращается вместе с бойцом
        let mut f = f;
        f.sword = Sword::Stay;

        assert!((f.sword_peak_velocity() - Vec2::new(-0.961449, 2.419069)).length() < 1e-4);

        // Клинок выдвинут: добавляется поступательное движение меча вдоль луча
        f.sword = Sword::Prick {
            dy: 0.5,
            velocity: 3.5,
        };

        assert!((f.sword_peak_velocity() - Vec2::new(-2.330137, 5.659315)).length() < 1e-4);
    }
}
