pub mod situation;

use crate::{
    approach::{self, approach_fighters},
    fighter::Fighter,
    math::{PI, TAU, normalize_angle},
};

pub struct Round {
    red: Fighter,
    blue: Fighter,

    time: u32,
}
