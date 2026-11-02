use glam::{Mat2, Vec2};

use crate::{
    fighter::{action::Action, brain::Brain, sword::Sword},
    math::{PI, SPT, TAU, add_vector_with_min, normalize_angle, shorten_vector},
    round::situation::{self, Situation},
};

pub mod action;
pub mod brain;
pub mod getters;
pub mod sword;

pub const TORSO_RADIUS: f32 = 1.0;
pub const HEAD_RADIUS: f32 = 0.8;
pub const FIST_RADIUS: f32 = 0.2;

pub const HEAD_DX: f32 = 0.;
pub const HEAD_DY: f32 = 0.5;
pub const HEAD_POS: Vec2 = Vec2::new(HEAD_DX, HEAD_DY);

pub const FIST_DX: f32 = -1.15;
pub const FIST_DY: f32 = 0.6;
pub const FIST_POS: Vec2 = Vec2::new(FIST_DX, FIST_DY);

pub const SWORD_DX: f32 = 0.95;
pub const SWORD_DY: f32 = 0.5;
pub const SWORD_POS: Vec2 = Vec2::new(SWORD_DX, SWORD_DY);

pub const ENERGY_RESTORATION_SPEED: f32 = 0.08;

pub const ACCELERATION: f32 = 5.;
pub const MAX_SPEED: f32 = 3.;
pub const ACCELERATION_ENERGY: f32 = 0.01;
pub const FRICTION: f32 = 0.4 * 9.8;

pub const ANGLE_ACCELERATION: f32 = PI;
pub const MAX_ANGLE_SPEED: f32 = PI * 0.75;
pub const ANGLE_ACCELERATION_ENERGY: f32 = 0.01;
pub const ANGLE_FRICTION: f32 = TAU * 0.67;

pub struct Fighter {
    brain: Box<dyn Brain>,
    sword: Sword,
    /// Позиция
    position: Vec2,
    /// Скорость (мировые единицы в секунду)
    velocity: Vec2,
    /// Направление взгляда в радианах, идет против часовой стрелки, при нуле направлен вверх
    facing: f32,
    /// Угловая скорость в радианах в секунду
    angle_speed: f32,

    energy: f32,
    stamina: f32,
    shock: f32,
}

impl Fighter {
    pub fn new(brain: Box<dyn Brain>) -> Self {
        Self {
            brain,
            sword: Sword::new(),
            position: Vec2::ZERO,
            facing: 0.,
            velocity: Vec2::ZERO,
            angle_speed: 0.,

            energy: 1.,
            stamina: 1.,
            shock: 0.,
        }
    }

    pub fn tick(&mut self) {
        self.sword.tick();
        self.unshock();

        self.position += self.velocity * SPT;
        self.facing += self.angle_speed * SPT;
    }

    pub fn get_actions(&mut self, situation: Situation) -> Option<Action> {
        if self.shock == 0. {
            Some(self.brain.get_actions(situation))
        } else {
            None
        }
    }

    pub fn do_actions(&mut self, action: Action) {
        let acceleration_vector =
            Mat2::from_angle(self.facing()) * action.into_vector() * ACCELERATION * SPT;

        if acceleration_vector != Vec2::ZERO {
            self.velocity = add_vector_with_min(self.velocity, acceleration_vector, MAX_SPEED);
            self.energy -= acceleration_vector.length() * ACCELERATION_ENERGY;
        } else {
            // Трение
            self.velocity = shorten_vector(self.velocity, FRICTION * SPT);
        }

        let angle_acceleration = action.rotation() * ANGLE_ACCELERATION * SPT;

        if angle_acceleration != 0. {
            self.angle_speed = (self.angle_speed + ANGLE_ACCELERATION * SPT).max(MAX_ANGLE_SPEED);
            self.energy -= angle_acceleration.abs() * ANGLE_ACCELERATION_ENERGY;
        } else {
            // Угловое трение
            self.angle_speed = todo!()
        }
    }

    pub fn rotate(&mut self, alpha: f32) {
        self.facing += alpha;
        self.facing = normalize_angle(self.facing);
    }

    pub fn prick(&mut self) {
        self.sword.prick();
    }

    pub fn chok(&mut self) {
        self.sword.chok();
    }

    fn unshock(&mut self) {
        self.shock = (self.shock - SPT).max(0.);
    }

    fn heal(&mut self) {
        let mut got_energy = ENERGY_RESTORATION_SPEED * SPT;

        let energy_need = self.stamina - self.energy;
        let energy_used = energy_need.min(got_energy);
        self.energy += energy_used;
        got_energy -= energy_used;

        if got_energy > 0. {
            self.energy += got_energy / 2.;
            self.stamina += got_energy / 2.;
        }

        self.stamina = self.stamina.min(1.);
        self.energy = self.energy.min(1.);
    }

    pub fn get_shock(&mut self, time: f32) {
        self.shock += time;
    }

    pub fn deal_damage(&mut self, damage: f32) {
        self.stamina -= damage;
        self.energy = self.energy.min(self.stamina);
    }
}
