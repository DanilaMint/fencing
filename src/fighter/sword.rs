use std::default;

use glam::Vec2;

use crate::{fighter::sword::Sword::*, math::SPT};

/// Длина меча
pub const SWORD_LENGTH: f32 = 2.0;

pub const SWORD_WIDTH: f32 = 0.05;

// ----- Колющий удар -----

mod prick {
    use crate::math::{sqrt_const, square};

    /// Дальность колющего удара
    pub const DISTANCE: f32 = 1.5;
    /// Ускорение
    pub const ACCELERATION: f32 = 15.;
    /// Начальная скорость
    pub const START_VELOCITY: f32 = 3.5;

    const PRICK_TIME: f32 = (-START_VELOCITY
        + sqrt_const(square(START_VELOCITY) + 2. * ACCELERATION * DISTANCE))
        / ACCELERATION;
}

// ----- Рубящий удар -----

mod chok {
    use crate::math::{sqrt_const, square};

    pub const START_RADIUS_VELOCITY: f32 = 2.5;
    pub const RADIUS_ACCELERATION: f32 = 4.;

    /// Дальность рубящего удара (wu/s)
    pub const CHOK_MAX_RADIUS: f32 = 1.3;
    /// Максимальный разворот (120°)
    pub const CHOK_MAX_ANGLE: f32 = ((45.) as f32).to_radians();
    /// Начальный разворот (-15°)
    pub const CHOK_START_ANGLE: f32 = (-105. as f32).to_radians();
    /// Начальная угловая скорость (°/s)
    pub const START_ANGLE_VELOCITY: f32 = (60. as f32).to_radians();
    /// Время удара (s)
    pub const CHOK_TIME: f32 = (-START_RADIUS_VELOCITY
        + sqrt_const(square(START_RADIUS_VELOCITY) + 2. * RADIUS_ACCELERATION * CHOK_MAX_RADIUS))
        / RADIUS_ACCELERATION;
    /// Угловое ускорение (°/s^2)
    pub const ANGLE_ACCELERAION: f32 = 2.
        * (CHOK_MAX_ANGLE - CHOK_START_ANGLE - START_RADIUS_VELOCITY * CHOK_TIME)
        / square(CHOK_TIME);
}

// ----- Возврат меча после удара

pub const RETRACT_VELOCITY: f32 = 3.;
pub const RETRACT_ANGLE_VELOCITY: f32 = (60. as f32).to_radians();

#[derive(Clone, PartialEq, Default)]
pub enum Sword {
    #[default]
    Stay,
    Prick {
        dy: f32,
        velocity: f32,
    },
    Chok {
        radius: f32,
        alpha: f32,
        velocity: f32,
        angle_velocity: f32,
    },
    Retract {
        alpha: f32,
        position: Vec2,
    },
}

impl Sword {
    pub fn new() -> Self {
        Stay
    }

    pub fn tick(&mut self) {
        match self {
            Stay => {}
            Prick { dy, velocity } => {
                *dy += *velocity * SPT;
                *velocity += prick::ACCELERATION * SPT;

                if *dy > prick::DISTANCE {
                    *dy = dy.min(prick::DISTANCE);
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
                *radius += *velocity * SPT;
                *velocity += chok::RADIUS_ACCELERATION * SPT;
                *alpha += *angle_velocity * SPT;
                *angle_velocity += chok::ANGLE_ACCELERAION * SPT;

                if *radius > chok::CHOK_MAX_RADIUS || *alpha > chok::CHOK_MAX_ANGLE {
                    *radius = radius.min(chok::CHOK_MAX_RADIUS);
                    *alpha = alpha.min(chok::CHOK_MAX_ANGLE);
                    self.retract();

                    return;
                }
            }
            Retract { alpha, position } => {
                let new_length = (position.length() - RETRACT_VELOCITY * SPT).max(0.);
                *position = position.normalize_or_zero() * new_length;

                // Меч возвращается к стойке: угол стремится к нулю и не проходит через него
                let current_alpha = *alpha;
                let step = RETRACT_ANGLE_VELOCITY * SPT;

                let new_alpha = if current_alpha.abs() <= step {
                    0.
                } else {
                    current_alpha - step * current_alpha.signum()
                };

                *alpha = new_alpha;

                if *position == Vec2::ZERO && new_alpha == 0. {
                    *self = Stay;
                }
            }
        }
    }

    pub fn local_sword_position(&self) -> Vec2 {
        match self {
            Stay => Vec2::ZERO,
            Prick { dy, .. } => Vec2::new(0., *dy),
            Chok { radius, alpha, .. } => Vec2::from_angle(*alpha + 90.0f32.to_radians()) * *radius,
            Retract { position, .. } => *position,
        }
    }

    pub fn angle(&self) -> f32 {
        match self {
            Stay => 0.,
            Prick { .. } => 0.,
            Chok { alpha, .. } => *alpha,
            Retract { alpha, .. } => *alpha,
        }
    }

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
                let position = Vec2::from_angle(*alpha + 90.0f32.to_radians()) * *radius;

                *self = Retract {
                    // Угол сохраняем: в Retract он будет стремиться к нулю
                    alpha: *alpha,
                    position,
                };
            }
            _ => {}
        }
    }

    pub fn prick(&mut self) {
        match self {
            Stay => {
                *self = Prick {
                    dy: 0.,
                    velocity: prick::START_VELOCITY,
                };
            }
            _ => {}
        }
    }

    pub fn chok(&mut self) {
        match self {
            Stay => {
                *self = Chok {
                    radius: 0.,
                    alpha: chok::CHOK_START_ANGLE,
                    velocity: chok::START_RADIUS_VELOCITY,
                    angle_velocity: chok::START_ANGLE_VELOCITY,
                }
            }
            _ => {}
        }
    }
}
