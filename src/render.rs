use glam::{Mat2, Vec2};
use macroquad::{miniquad::window::screen_size, prelude::*};

use crate::{
    colors::{RETRACT_SWORD_COLOR, STROKE, SWORD_COLOR},
    fighter::{FIST_RADIUS, Fighter, HEAD_RADIUS, TORSO_RADIUS, sword::SWORD_WIDTH},
};

fn world_position_to_screen(pos: Vec2) -> Vec2 {
    let (x, y) = screen_size();
    let screen_size = Vec2::new(x, y);

    WU_TO_SCREEN_MATRIX * pos + screen_size / 2.
}

// Константа перевода мировых единиц в экранные
const WU_TO_SCREEN: f32 = 64.;

const WU_TO_SCREEN_MATRIX: Mat2 = Mat2 {
    x_axis: Vec2::new(WU_TO_SCREEN, 0.),
    y_axis: Vec2::new(0., -WU_TO_SCREEN),
};

const THICKNESS: f32 = 2.;

fn draw_full_circle(pos: Vec2, radius: f32, fill: Color, stroke: Color, thickness: f32) {
    draw_poly(pos.x, pos.y, 255, radius, 0., fill);
    draw_poly_lines(pos.x, pos.y, 255, radius, 0., thickness, stroke);
}

fn draw_torso(pos: Vec2, color: Color) {
    draw_full_circle(
        world_position_to_screen(pos),
        TORSO_RADIUS * WU_TO_SCREEN,
        color,
        STROKE,
        THICKNESS,
    );
}

fn draw_head(pos: Vec2, color: Color) {
    draw_full_circle(
        world_position_to_screen(pos),
        HEAD_RADIUS * WU_TO_SCREEN,
        color,
        STROKE,
        THICKNESS,
    );
}

fn draw_fist(pos: Vec2, color: Color) {
    draw_full_circle(
        world_position_to_screen(pos),
        FIST_RADIUS * WU_TO_SCREEN,
        color,
        STROKE,
        THICKNESS,
    );
}

fn draw_sword(mut pos: Vec2, mut peak: Vec2, is_retracts: bool) {
    pos = world_position_to_screen(pos);
    peak = world_position_to_screen(peak);
    draw_line(
        pos.x,
        pos.y,
        peak.x,
        peak.y,
        SWORD_WIDTH * WU_TO_SCREEN,
        if !is_retracts {
            SWORD_COLOR
        } else {
            RETRACT_SWORD_COLOR
        },
    );
}

pub fn draw_fighter(fighter: &Fighter, color: Color) {
    let (sword_pos, peak_pos) = fighter.sword_position();

    draw_torso(fighter.torso_position(), color);
    draw_head(fighter.head_position(), color);
    draw_fist(fighter.fist_position(), color);
    draw_sword(sword_pos, peak_pos, fighter.is_sword_retracts());
}
