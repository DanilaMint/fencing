use super::*;

#[derive(Clone)]
pub struct Doll;

impl Brain for Doll {
    fn get_actions(&mut self, _: Situation) -> Action {
        Action::default()
    }
}
