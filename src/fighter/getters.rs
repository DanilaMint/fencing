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
