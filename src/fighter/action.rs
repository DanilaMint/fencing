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
