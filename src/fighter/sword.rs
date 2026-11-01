use glam::Vec2;

use crate::{fighter::sword::Sword::*, math::SPT};

pub const SWORD_DX: f32 = 0.95;
pub const SWORD_DY: f32 = 0.5;

pub const SWORD_STAY_LOCAL_POS: Vec2 = Vec2::new(SWORD_DX, SWORD_DY);

pub const SWORD_LENGTH: f32 = 1.5;

pub const PRICK_DISTANCE: f32 = 1.5;

pub const SWORD_ACCELERATION: f32 = 3.;

#[derive(Clone)]
pub enum Sword {
    Stay,
    Prick {
        dy: f32,
        velocity: f32,
    },
    Chok {
        radius: f32,
        alpha: f32,
        velocity: f32,
    },
    Retract {
        dx: f32,
        alpha: f32,
        dy: f32,
    },
}

impl Sword {
    pub fn tick(&mut self) {
        match self {
            Stay => {}
            &mut Prick { dy, velocity } => {
                dy += velocity * SPT;
                velocity += SWORD_ACCELERATION * SPT;

                if dy > PRICK_DISTANCE {
                    dy = dy.min(PRICK_DISTANCE);

                    self.retract_after_prick();

                    return;
                }
            }
        }
    }

    pub fn local_sword_pos(&self) -> Vec2 {
        match self {
            Stay => SWORD_STAY_LOCAL_POS,
            Prick { dy, .. } => Vec2::new(SWORD_DX, SWORD_DY + dy),
            Chok { radius, alpha, .. } => {
                let delta = Vec2::from_angle(*alpha) * *radius;

                delta + SWORD_STAY_LOCAL_POS
            }
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

    fn retract_after_prick(&mut self) {
        match self {
            Prick { dy, .. } => {
                *self = Retract {
                    dx: 0.,
                    alpha: 0.,
                    dy: *dy,
                }
            }
            _ => {}
        }
    }
}
