use glam::{Mat2, Vec2};

use crate::fighter::{
    FIST_POS, Fighter, HEAD_POS, SWORD_POS,
    brain::Brain,
    sword::{SWORD_LENGTH, Sword},
};

impl Fighter {
    /// Возвращает позиции рукояти меча и его острия
    pub fn sword_position(&self) -> (Vec2, Vec2) {
        let pos = self.facing_to_matrix() * (SWORD_POS + self.sword.local_sword_position())
            + self.position;
        let peak = pos
            + Vec2::from_angle(self.sword.angle() + self.facing + 90.0f32.to_radians())
                * SWORD_LENGTH;

        (pos, peak)
    }

    pub fn head_position(&self) -> Vec2 {
        self.position + self.facing_to_matrix() * HEAD_POS
    }

    pub fn fist_position(&self) -> Vec2 {
        self.position + self.facing_to_matrix() * FIST_POS
    }

    pub fn torso_position(&self) -> Vec2 {
        self.position
    }

    pub fn position(&self) -> Vec2 {
        self.position
    }

    pub fn facing(&self) -> f32 {
        self.facing
    }

    pub fn velocity(&self) -> Vec2 {
        self.velocity
    }

    /// Угловая скорость (в радианах в секунду)
    pub fn angle_speed(&self) -> f32 {
        self.angle_speed
    }

    /// Скорость точки тела, заданной её смещением в локальной системе координат бойца
    fn local_point_velocity(&self, local: Vec2) -> Vec2 {
        self.velocity + (self.facing_to_matrix() * local.perp()) * self.angle_speed
    }

    /// Скорость головы бойца
    pub fn head_velocity(&self) -> Vec2 {
        self.local_point_velocity(HEAD_POS)
    }

    /// Скорость левой руки бойца
    pub fn fist_velocity(&self) -> Vec2 {
        self.local_point_velocity(FIST_POS)
    }

    /// Скорость острия меча бойца
    pub fn sword_peak_velocity(&self) -> Vec2 {
        let matrix = self.facing_to_matrix();

        // Положение рукояти в локальной системе координат
        let hilt_local = SWORD_POS + self.sword.local_sword_position();
        // Направление клинка в мировой системе координат
        let blade = Vec2::from_angle(self.sword.angle() + self.facing + 90.0f32.to_radians());

        self.velocity
            // Поворот бойца вместе с мечом
            + (matrix * hilt_local.perp()) * self.angle_speed
            // Движение меча относительно бойца
            + matrix * self.sword.local_velocity()
            // Поворот клинка вокруг рукояти
            + blade.perp() * (self.sword.angle_velocity() + self.angle_speed) * SWORD_LENGTH
    }

    pub fn extract_brain(self) -> Box<dyn Brain> {
        self.brain
    }

    fn facing_to_matrix(&self) -> Mat2 {
        Mat2::from_angle(self.facing)
    }

    pub fn is_sword_retracts(&self) -> bool {
        match self.sword {
            Sword::Retract { .. } => true,
            _ => false,
        }
    }

    pub fn is_knockout(&self) -> bool {
        self.stamina <= 0.
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        fighter::{
            FIST_POS, HEAD_POS, SWORD_POS,
            brain::Doll,
            sword::{Sword, SWORD_LENGTH},
        },
        math::SPT,
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
        next.position = f.position + f.velocity * SPT * ticks as f32;
        next.facing += f.angle_speed * SPT * ticks as f32;
        next.sword = f.sword.clone();

        for _ in 0..ticks {
            next.sword.tick();
        }

        (point(&next) - p0) / (SPT * ticks as f32)
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

    #[test]
    fn constants_used() {
        // Чтобы не было неиспользуемых импортов в тестах
        let _ = (SWORD_POS, HEAD_POS, FIST_POS, SWORD_LENGTH);
    }

    /// Значения посчитаны вручную по формулам движения точек тела:
    /// v_точки = v + omega * perp(M(facing) * смещение)
    #[test]
    fn reference_values_test() {
        let f = fighter(Vec2::new(1., 2.), 0.7, 0.3);

        assert!((f.head_velocity() - Vec2::new(0.665632, 1.896568)).length() < 1e-4);
        assert!((f.fist_velocity() - Vec2::new(0.836652, 1.106836)).length() < 1e-4);

        // Меч покоится: острие вращается вместе с бойцом
        let mut f = f;
        f.sword = Sword::Stay;

        assert!((f.sword_peak_velocity() - Vec2::new(-0.868358, 2.118138)).length() < 1e-4);

        // Клинок выдвинут: добавляется поступательное движение меча вдоль луча
        f.sword = Sword::Prick {
            dy: 0.5,
            velocity: 3.5,
        };

        assert!((f.sword_peak_velocity() - Vec2::new(-2.237046, 5.358382)).length() < 1e-4);
    }
}
