use glam::Vec2;

use crate::{
    fighter::{LOCAL_FORWARD_ANGLE, sword::Sword::*},
    math::{DT, shorten_vector, step_to_zero},
};

/// Длина клинка от рукояти до острия, мировые единицы
pub const BLADE_LENGTH: f32 = 2.0;

/// Толщина клинка, мировые единицы
pub const BLADE_WIDTH: f32 = 0.05;

// ----- Колющий удар -----

mod prick {
    /// Дальность выдвижения клинка, мировые единицы
    pub const REACH: f32 = 1.5;
    /// Ускорение клинка при выдвижении, мировых единиц в секунду в квадрате
    pub const ACCELERATION: f32 = 15.;
    /// Начальная скорость клинка, мировых единиц в секунду
    pub const START_SPEED: f32 = 3.5;
}

// ----- Рубящий удар -----

mod chok {
    use crate::math::{sqrt_const, square};

    /// Начальная скорость разведения клинка, мировых единиц в секунду
    pub const START_RADIUS_SPEED: f32 = 2.5;
    /// Ускорение разведения клинка, мировых единиц в секунду в квадрате
    pub const RADIUS_ACCELERATION: f32 = 4.;

    /// Максимальный радиус дуги удара, мировые единицы
    pub const MAX_RADIUS: f32 = 1.3;
    /// Максимальный разворот клинка, 45°
    pub const MAX_ANGLE: f32 = 45_f32.to_radians();
    /// Начальный разворот клинка, -105°
    pub const START_ANGLE: f32 = (-105_f32).to_radians();
    /// Начальная угловая скорость клинка, радиан в секунду
    pub const START_ANGLE_SPEED: f32 = 60_f32.to_radians();
    /// Длительность рубящего удара, секунды
    pub const DURATION: f32 = (-START_RADIUS_SPEED
        + sqrt_const(square(START_RADIUS_SPEED) + 2. * RADIUS_ACCELERATION * MAX_RADIUS))
        / RADIUS_ACCELERATION;
    /// Угловое ускорение клинка, радиан в секунду в квадрате
    pub const ANGLE_ACCELERATION: f32 =
        2. * (MAX_ANGLE - START_ANGLE - START_ANGLE_SPEED * DURATION) / square(DURATION);
}

// ----- Возврат меча после удара

/// Скорость возврата клинка в стойку, мировых единиц в секунду
pub const RETRACT_SPEED: f32 = 3.;

/// Угловая скорость возврата клинка в стойку, радиан в секунду
pub const RETRACT_ANGLE_SPEED: f32 = 60_f32.to_radians();

#[derive(Clone, PartialEq, Default)]
pub enum Sword {
    /// Стоит ровно, готов нанести удар
    #[default]
    Stay,
    /// В процессе нанесения колющего удара
    Prick {
        /// Смещение меча
        dy: f32,
        /// Скорость смещения
        velocity: f32,
    },
    /// В процессе нанесения рубящего удара
    Chok {
        /// Смещение меча от условного центра руки
        radius: f32,
        /// Разворот от базового положения
        alpha: f32,
        /// Скорость удлинения `radius`
        velocity: f32,
        /// Угловая скорость
        angle_velocity: f32,
    },
    /// Отходит к положению `Stay`
    Retract {
        /// Разворот от базового положения
        alpha: f32,
        /// Позиция смещения рукояти
        position: Vec2,
    },
}

impl Sword {
    /// Создать новый меч в состоянии Stay, аналогично `Sword::default()`
    pub fn new() -> Self {
        Stay
    }

    /// Просчитать изменения, которые произойдут в текущем такте симуляции
    pub fn tick(&mut self) {
        match self {
            Stay => {}
            Prick { dy, velocity } => {
                *dy += *velocity * DT;
                *velocity += prick::ACCELERATION * DT;

                if *dy > prick::REACH {
                    *dy = dy.min(prick::REACH);
                    self.retract();

                    return;
                }
            }
            Chok {
                radius,
                alpha,
                velocity,
                angle_velocity,
            } => {
                *radius += *velocity * DT;
                *velocity += chok::RADIUS_ACCELERATION * DT;
                *alpha += *angle_velocity * DT;
                *angle_velocity += chok::ANGLE_ACCELERATION * DT;

                if *radius > chok::MAX_RADIUS || *alpha > chok::MAX_ANGLE {
                    *radius = radius.min(chok::MAX_RADIUS);
                    *alpha = alpha.min(chok::MAX_ANGLE);
                    self.retract();

                    return;
                }
            }
            Retract { alpha, position } => {
                *position = shorten_vector(*position, RETRACT_SPEED * DT);

                *alpha = step_to_zero(*alpha, RETRACT_ANGLE_SPEED * DT);

                if *position == Vec2::ZERO && *alpha == 0. {
                    *self = Stay;
                }
            }
        }
    }

    pub fn local_sword_position(&self) -> Vec2 {
        match self {
            Stay => Vec2::ZERO,
            Prick { dy, .. } => Vec2::new(0., *dy),
            Chok { radius, alpha, .. } => Vec2::from_angle(*alpha + LOCAL_FORWARD_ANGLE) * *radius,
            Retract { position, .. } => *position,
        }
    }

    /// Угол наклона от базовой позиции
    pub fn angle(&self) -> f32 {
        match self {
            Stay => 0.,
            Prick { .. } => 0.,
            Chok { alpha, .. } => *alpha,
            Retract { alpha, .. } => *alpha,
        }
    }

    /// Скорость изменения положения меча в локальной системе координат бойца
    pub fn local_velocity(&self) -> Vec2 {
        match self {
            Stay => Vec2::ZERO,
            Prick { velocity, .. } => Vec2::new(0., *velocity),
            Chok {
                radius,
                alpha,
                velocity,
                angle_velocity,
            } => {
                let dir = Vec2::from_angle(*alpha + LOCAL_FORWARD_ANGLE);

                dir * *velocity + dir.perp() * (*radius * *angle_velocity)
            }
            Retract { position, .. } => {
                if *position == Vec2::ZERO {
                    Vec2::ZERO
                } else {
                    -position.normalize() * RETRACT_SPEED
                }
            }
        }
    }

    /// Скорость изменения угла меча (в радианах в секунду)
    pub fn angle_velocity(&self) -> f32 {
        match self {
            Stay | Prick { .. } => 0.,
            Chok { angle_velocity, .. } => *angle_velocity,
            Retract { alpha, .. } => {
                if alpha.abs() <= RETRACT_ANGLE_SPEED * DT {
                    0.
                } else {
                    -alpha.signum() * RETRACT_ANGLE_SPEED
                }
            }
        }
    }

    /// Переключить меч в режим `Sword::Retract` из состояния удара
    pub fn retract(&mut self) {
        match self {
            Prick { dy, .. } => {
                let pos = Vec2::new(0., *dy);
                *self = Retract {
                    alpha: 0.,
                    position: pos,
                }
            }
            Chok { radius, alpha, .. } => {
                let position = Vec2::from_angle(*alpha + LOCAL_FORWARD_ANGLE) * *radius;

                *self = Retract {
                    // Угол сохраняем: в Retract он будет стремиться к нулю
                    alpha: *alpha,
                    position,
                };
            }
            _ => {}
        }
    }

    /// Нанести колющий удар, если меч готов
    pub fn prick(&mut self) {
        match self {
            Stay => {
                *self = Prick {
                    dy: 0.,
                    velocity: prick::START_SPEED,
                };
            }
            _ => {}
        }
    }

    /// Нанести рубящий удар, если меч готов
    pub fn chok(&mut self) {
        match self {
            Stay => {
                *self = Chok {
                    radius: 0.,
                    alpha: chok::START_ANGLE,
                    velocity: chok::START_RADIUS_SPEED,
                    angle_velocity: chok::START_ANGLE_SPEED,
                }
            }
            _ => {}
        }
    }

    /// Готов ли меч нанести удар
    pub fn is_ready(&self) -> bool {
        match self {
            Stay => true,
            _ => false,
        }
    }

    /// Наносит ли меч удар в данный момент
    pub fn is_attacking(&self) -> bool {
        matches!(self, Prick { .. } | Chok { .. })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Рубящий удар к концу своей длительности достигает максимального угла
    #[test]
    fn chok_reaches_max_angle() {
        let mut sword = Sword::new();
        sword.chok();

        let mut ticks = 0;
        while matches!(sword, Chok { .. }) && ticks < 100 {
            sword.tick();
            ticks += 1;
        }

        // Удар закончился переходом меча в возврат
        assert!(matches!(sword, Retract { .. }));
        // Угол клинка дошёл до максимального (дискретная интеграция даёт небольшую погрешность)
        assert!((sword.angle() - chok::MAX_ANGLE).abs() < 0.1);

        // Длительность удара совпадает с расчётной
        let duration = ticks as f32 * DT;
        assert!((duration - chok::DURATION).abs() < 3. * DT);
    }

    /// Меч возвращается в стойку после удара
    #[test]
    fn sword_returns_to_stance() {
        let mut sword = Sword::new();
        sword.chok();

        for _ in 0..200 {
            sword.tick();
        }

        assert!(matches!(sword, Stay));
    }

    /// Колющий удар выдвигает меч вдоль его клинка
    #[test]
    fn prick_moves_along_blade() {
        let mut sword = Sword::new();
        sword.prick();

        assert!(matches!(sword, Prick { .. }));

        let blade = Vec2::from_angle(sword.angle() + LOCAL_FORWARD_ANGLE);
        let motion = sword.local_velocity();

        assert!((motion.normalize_or_zero() - blade).length() < 1e-4);
    }
}
