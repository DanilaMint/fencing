pub trait Brain {
    fn get_actions(&mut self);
}

#[derive(Clone)]
pub struct Doll;

impl Brain for Doll {
    fn get_actions(&mut self) {}
}
