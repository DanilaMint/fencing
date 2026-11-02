use crate::{fighter::action::Action, round::situation::Situation};

pub trait Brain {
    fn get_actions(&mut self, situation: Situation) -> Action;
}

#[derive(Clone)]
pub struct Doll;

impl Brain for Doll {
    fn get_actions(&mut self, _: Situation) -> Action {
        Action::default()
    }
}
