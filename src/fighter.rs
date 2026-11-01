use crate::fighter::brain::Brain;

mod brain;
mod sword;

pub struct Fighter {
    brain: Box<dyn Brain>,
}
