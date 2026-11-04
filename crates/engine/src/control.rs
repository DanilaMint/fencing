use macroquad::prelude::*;

use fencing_core::{
    brain::Brain,
    fighter::action::{Action, Direction, SwordAction},
    round::situation::Situation,
};

use crate::render::screen_position_to_world;

/// Управление бойцом игроком
pub struct PlayerControl;

impl Brain for PlayerControl {
    fn get_actions(&mut self, situation: Situation) -> Action {
        let keys = get_keys_down();

        // Движение вдоль осей бойца: W — вперёд, S — назад, D — вправо, A — влево
        let forward = match (keys.contains(&KeyCode::W), keys.contains(&KeyCode::S)) {
            (true, false) => Direction::Positive,
            (false, true) => Direction::Negative,
            _ => Direction::Zero,
        };
        let side = match (keys.contains(&KeyCode::D), keys.contains(&KeyCode::A)) {
            (true, false) => Direction::Positive,
            (false, true) => Direction::Negative,
            _ => Direction::Zero,
        };

        // Боец доворачивает в сторону курсора
        let cursor = screen_position_to_world(Vec2::from(mouse_position()));
        let to_cursor = cursor - situation.position;
        // Если курсор совпал с центром бойца, направление не определено
        let turn_to = if to_cursor.length_squared() > f32::EPSILON {
            Some(to_cursor.to_angle())
        } else {
            None
        };

        // ЛКМ — колющий удар, ПКМ — рубящий
        let sword = if is_mouse_button_pressed(MouseButton::Left) {
            SwordAction::Prick
        } else if is_mouse_button_pressed(MouseButton::Right) {
            SwordAction::Chok
        } else {
            SwordAction::None
        };

        Action {
            forward,
            side,
            rotate: Direction::Zero,
            sword,
            turn_to,
        }
    }
}
