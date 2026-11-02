use glam::Vec2;

#[repr(i8)]
#[derive(Clone, Copy, Default)]
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

#[derive(Clone, Copy, Default)]
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
}

impl Action {
    pub fn into_vector(&self) -> Vec2 {
        Vec2::new(self.side.as_f32(), -self.forward.as_f32())
    }

    pub fn rotation(&self) -> f32 {
        self.rotate.as_f32()
    }
}
