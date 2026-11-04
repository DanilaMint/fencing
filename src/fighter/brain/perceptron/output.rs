//! Разбор выходов сети в команду бойца.
//!
//! Сеть выдаёт три группы выходов: направление движения, поворот и тип
//! удара. Каждая группа переводится в своё поле команды `Action`.

use crate::fighter::action::{Action, Direction, MoveDir8, SwordAction};

use super::net::{MOVE_COUNT, SWORD_COUNT, TURN_COUNT};

/// Направление движения по индексу в группе движения
pub fn move_dir(index: usize) -> MoveDir8 {
    MoveDir8::ALL[index % MOVE_COUNT]
}

/// Поворот по индексу в группе поворота
///
/// Порядок: не поворачиваться, влево (против часовой стрелки), вправо.
pub fn turn(index: usize) -> Direction {
    match index % TURN_COUNT {
        0 => Direction::Zero,
        1 => Direction::Positive,
        _ => Direction::Negative,
    }
}

/// Тип удара по индексу в группе удара
pub fn sword(index: usize) -> SwordAction {
    match index % SWORD_COUNT {
        0 => SwordAction::None,
        1 => SwordAction::Prick,
        _ => SwordAction::Chok,
    }
}

/// Команда бойца по индексам выбранных действий
pub fn action(move_index: usize, turn_index: usize, sword_index: usize) -> Action {
    let dir = move_dir(move_index);

    Action {
        forward: dir.forward(),
        side: dir.side(),
        rotate: turn(turn_index),
        sword: sword(sword_index),
        turn_to: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Каждое направление раскладывается в поля команды
    #[test]
    fn move_index_becomes_direction() {
        let forward = action(0, 0, 0);
        assert_eq!(forward.forward, Direction::Positive);
        assert_eq!(forward.side, Direction::Zero);

        let left = action(2, 0, 0);
        assert_eq!(left.forward, Direction::Zero);
        assert_eq!(left.side, Direction::Negative);

        let forward_right = action(5, 0, 0);
        assert_eq!(forward_right.forward, Direction::Positive);
        assert_eq!(forward_right.side, Direction::Positive);
    }

    /// Индексы группы поворота дают три состояния
    #[test]
    fn turn_index_becomes_direction() {
        assert_eq!(turn(0), Direction::Zero);
        assert_eq!(turn(1), Direction::Positive);
        assert_eq!(turn(2), Direction::Negative);
    }

    /// Индексы группы удара дают три состояния
    #[test]
    fn sword_index_becomes_strike() {
        assert_eq!(sword(0), SwordAction::None);
        assert_eq!(sword(1), SwordAction::Prick);
        assert_eq!(sword(2), SwordAction::Chok);
    }

    /// Индексы за пределами группы не ломают разбор
    #[test]
    fn out_of_range_index_is_wrapped() {
        assert_eq!(move_dir(MOVE_COUNT), MoveDir8::ALL[0]);
        assert_eq!(turn(TURN_COUNT), Direction::Zero);
        assert_eq!(sword(SWORD_COUNT), SwordAction::None);
    }

    /// Команда не задаёт угол доворота: сеть управляет поворотом сама
    #[test]
    fn action_has_no_turn_target() {
        assert!(action(0, 1, 1).turn_to.is_none());
    }
}
