use glam::Vec2;
use serde::{Deserialize, Serialize};

#[repr(i8)]
#[derive(Clone, Copy, Default, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Direction {
    Positive = 1,
    #[default]
    Zero = 0,
    Negative = -1,
}

impl Direction {
    pub fn up(&mut self) -> Self {
        *self = match self {
            Self::Positive | Self::Zero => Self::Positive,
            Self::Negative => Self::Zero,
        };

        *self
    }

    pub fn down(&mut self) -> Self {
        *self = match self {
            Self::Negative | Self::Zero => Self::Negative,
            Self::Positive => Self::Zero,
        };

        *self
    }

    pub fn inverse(&mut self) -> Self {
        *self = match self {
            Self::Positive => Self::Negative,
            Self::Zero => Self::Zero,
            Self::Negative => Self::Positive,
        };

        *self
    }

    pub fn as_f32(&self) -> f32 {
        *self as i8 as f32
    }
}

#[derive(Clone, Copy, Default, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum SwordAction {
    #[default]
    None,
    Prick,
    Chok,
}

/// Направление движения: восемь направлений в локальной системе координат бойца.
///
/// Это ровно все ненулевые комбинации `forward` и `side` из `{-1, 0, 1}`,
/// поэтому каждое направление раскладывается в поля команды `Action`.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum MoveDir8 {
    /// Только вперёд
    Forward,
    /// Только назад
    Back,
    /// Только влево
    Left,
    /// Только вправо
    Right,
    /// Вперёд и влево
    ForwardLeft,
    /// Вперёд и вправо
    ForwardRight,
    /// Назад и влево
    BackLeft,
    /// Назад и вправо
    BackRight,
}

impl MoveDir8 {
    /// Все восемь направлений
    pub const ALL: [MoveDir8; 8] = [
        MoveDir8::Forward,
        MoveDir8::Back,
        MoveDir8::Left,
        MoveDir8::Right,
        MoveDir8::ForwardLeft,
        MoveDir8::ForwardRight,
        MoveDir8::BackLeft,
        MoveDir8::BackRight,
    ];

    /// Составляющая «вперёд или назад»
    pub fn forward(self) -> Direction {
        match self {
            MoveDir8::Forward | MoveDir8::ForwardLeft | MoveDir8::ForwardRight => {
                Direction::Positive
            }
            MoveDir8::Back | MoveDir8::BackLeft | MoveDir8::BackRight => Direction::Negative,
            MoveDir8::Left | MoveDir8::Right => Direction::Zero,
        }
    }

    /// Составляющая «вправо или влево»
    pub fn side(self) -> Direction {
        match self {
            MoveDir8::Right | MoveDir8::ForwardRight | MoveDir8::BackRight => Direction::Positive,
            MoveDir8::Left | MoveDir8::ForwardLeft | MoveDir8::BackLeft => Direction::Negative,
            MoveDir8::Forward | MoveDir8::Back => Direction::Zero,
        }
    }
}

#[derive(Default)]
pub struct Action {
    pub forward: Direction,
    pub side: Direction,
    pub rotate: Direction,
    pub sword: SwordAction,
    /// Целевой угол взгляда (стандартный угол, как у `Fighter::facing()`).
    /// Если задан, боец доворачивает к нему пропорциональным серво
    /// и команда `rotate` не учитывается
    pub turn_to: Option<f32>,
}

impl Action {
    /// Вектор движения в локальной системе координат бойца:
    /// `x` — вправо, `y` — вперёд
    pub fn into_vector(&self) -> Vec2 {
        Vec2::new(self.side.as_f32(), self.forward.as_f32())
    }

    pub fn rotation(&self) -> f32 {
        self.rotate.as_f32()
    }
}
