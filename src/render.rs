use glam::{Mat2, Vec2};
use macroquad::{miniquad::window::screen_size, prelude::*};

use crate::{
    colors::{
        ENERGY_BAR_COLOR, OUTLINE_COLOR, RETRACT_SWORD_COLOR, STAMINA_BAR_COLOR,
        STATE_BAR_BACKGROUND_COLOR, SWORD_COLOR,
    },
    fighter::{FIST_RADIUS, Fighter, HEAD_RADIUS, TORSO_HEIGHT, TORSO_WIDTH, sword::BLADE_WIDTH},
    round::{LoseReason, Round, RoundResult},
};

fn world_position_to_screen(pos: Vec2) -> Vec2 {
    let (x, y) = screen_size();
    let screen_size = Vec2::new(x, y);

    WORLD_TO_SCREEN_MATRIX * pos + screen_size / 2.
}

/// Переводит экранные координаты (пиксели, ось Y направлена вниз) в мировые
pub(crate) fn screen_position_to_world(pos: Vec2) -> Vec2 {
    let (x, y) = screen_size();
    let screen_size = Vec2::new(x, y);

    let centered = pos - screen_size / 2.;

    Vec2::new(
        centered.x / PIXELS_PER_WORLD_UNIT,
        -centered.y / PIXELS_PER_WORLD_UNIT,
    )
}

/// Сколько пикселей экрана соответствует одной мировой единице
const PIXELS_PER_WORLD_UNIT: f32 = 64.;

/// Матрица перевода мировых координат в экранные (ось Y направлена вверх)
const WORLD_TO_SCREEN_MATRIX: Mat2 = Mat2 {
    x_axis: Vec2::new(PIXELS_PER_WORLD_UNIT, 0.),
    y_axis: Vec2::new(0., -PIXELS_PER_WORLD_UNIT),
};

/// Толщина обводки частей тела, пиксели
const OUTLINE_THICKNESS: f32 = 2.;

/// Длина полосы состояния как доля от диаметра головы бойца
const STATE_BAR_LENGTH_RATIO: f32 = 0.8;
/// Высота полосы состояния как доля от радиуса головы бойца
const STATE_BAR_HEIGHT_RATIO: f32 = 0.25;

fn draw_full_circle(pos: Vec2, radius: f32, fill: Color, stroke: Color, thickness: f32) {
    draw_poly(pos.x, pos.y, 255, radius, 0., fill);
    draw_poly_lines(pos.x, pos.y, 255, radius, 0., thickness, stroke);
}

/// Рисует эллипс туловища бойца. `angle` — угол поворота тела бойца:
/// ось ширины (поперёк плеч) идёт вдоль локальной оси X бойца.
fn draw_torso(pos: Vec2, angle: f32, color: Color) {
    let screen = world_position_to_screen(pos);
    let width = TORSO_WIDTH * PIXELS_PER_WORLD_UNIT;
    let height = TORSO_HEIGHT * PIXELS_PER_WORLD_UNIT;
    // Экранные координаты отражают ось Y, поэтому угол поворота эллипса
    // в экране противоположен углу поворота тела в мире
    let rotation = -angle.to_degrees();

    draw_ellipse(screen.x, screen.y, width, height, rotation, color);
    draw_ellipse_lines(
        screen.x,
        screen.y,
        width,
        height,
        rotation,
        OUTLINE_THICKNESS,
        OUTLINE_COLOR,
    );
}

fn draw_head(pos: Vec2, color: Color) {
    draw_full_circle(
        world_position_to_screen(pos),
        HEAD_RADIUS * PIXELS_PER_WORLD_UNIT,
        color,
        OUTLINE_COLOR,
        OUTLINE_THICKNESS,
    );
}

fn draw_fist(pos: Vec2, color: Color) {
    draw_full_circle(
        world_position_to_screen(pos),
        FIST_RADIUS * PIXELS_PER_WORLD_UNIT,
        color,
        OUTLINE_COLOR,
        OUTLINE_THICKNESS,
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
        BLADE_WIDTH * PIXELS_PER_WORLD_UNIT,
        if !is_retracts {
            SWORD_COLOR
        } else {
            RETRACT_SWORD_COLOR
        },
    );
}

/// Прямоугольники полосы состояния бойца в экранных координатах:
/// (фон, выносливость, энергия). `head_screen` — центр головы бойца в пикселях экрана.
///
/// Полоса горизонтальна и не зависит от направления взгляда бойца:
/// она центрирована по голове и лежит на её горизонтальном диаметре.
fn state_bar_rects(head_screen: Vec2, stamina: f32, energy: f32) -> (Rect, Rect, Rect) {
    let head_radius = HEAD_RADIUS * PIXELS_PER_WORLD_UNIT;
    let length = head_radius * 2. * STATE_BAR_LENGTH_RATIO;
    let height = head_radius * STATE_BAR_HEIGHT_RATIO;

    let left = head_screen.x - length / 2.;
    let top = head_screen.y - height / 2.;

    let background = Rect::new(left, top, length, height);
    // Выносливость и энергия растут от левого края полосы
    let stamina = Rect::new(left, top, length * stamina.clamp(0., 1.), height);
    let energy = Rect::new(left, top, length * energy.clamp(0., 1.), height);

    (background, stamina, energy)
}

/// Рисует полосу состояния бойца внутри его головы:
/// чёрный фон, поверх него белая полоса выносливости,
/// поверх выносливости — зелёная полоса энергии.
fn draw_state_bar(fighter: &Fighter) {
    let head = world_position_to_screen(fighter.head_position());
    let (background, stamina, energy) = state_bar_rects(head, fighter.stamina(), fighter.energy());

    draw_rectangle(
        background.x,
        background.y,
        background.w,
        background.h,
        STATE_BAR_BACKGROUND_COLOR,
    );
    draw_rectangle(
        stamina.x,
        stamina.y,
        stamina.w,
        stamina.h,
        STAMINA_BAR_COLOR,
    );
    draw_rectangle(energy.x, energy.y, energy.w, energy.h, ENERGY_BAR_COLOR);
}

pub fn draw_fighter(fighter: &Fighter, color: Color) {
    let (sword_pos, peak_pos) = fighter.sword_position();

    draw_torso(fighter.torso_position(), fighter.body_angle(), color);
    draw_head(fighter.head_position(), color);
    draw_state_bar(fighter);
    draw_fist(fighter.fist_position(), color);
    draw_sword(sword_pos, peak_pos, fighter.is_sword_retracts());
}

pub fn draw_round(round: &Round) {
    // Границы ринга
    draw_poly_lines(
        screen_width() / 2.,
        screen_height() / 2.,
        255,
        round.ring_radius() * PIXELS_PER_WORLD_UNIT,
        0.,
        OUTLINE_THICKNESS,
        WHITE,
    );
    // Красный боец
    draw_fighter(round.red_fighter(), RED);
    // Синий боец
    draw_fighter(round.blue_fighter(), BLUE);
}

/// Размер шрифта заголовка результата, пиксели
const RESULT_TITLE_FONT_SIZE: u16 = 72;
/// Размер шрифта причины окончания раунда, пиксели
const RESULT_REASON_FONT_SIZE: u16 = 32;
/// Размер шрифта подсказки, пиксели
const RESULT_HINT_FONT_SIZE: u16 = 24;
/// Прозрачность затемнения арены под результатом раунда
const RESULT_DIM_ALPHA: u8 = 180;

/// Оверлей с результатом раунда: затемнение арены, итог, причина и подсказка.
/// Если раунд ещё идёт, ничего не рисует.
pub fn draw_round_result(round: &Round) {
    let Some(result) = round.result() else {
        return;
    };

    // Затемняем арену
    draw_rectangle(
        0.,
        0.,
        screen_width(),
        screen_height(),
        Color::from_rgba(0, 0, 0, RESULT_DIM_ALPHA),
    );

    let (title, color) = match result {
        RoundResult::RedWin => ("RED WINS", RED),
        RoundResult::BlueWin => ("BLUE WINS", BLUE),
        RoundResult::Draw => ("DRAW", WHITE),
    };

    draw_centered_text(
        title,
        RESULT_TITLE_FONT_SIZE,
        screen_height() * 0.30,
        color,
    );
    draw_centered_text(
        lose_reason_text(round.red_lose_reason(), round.blue_lose_reason()),
        RESULT_REASON_FONT_SIZE,
        screen_height() * 0.44,
        WHITE,
    );
    draw_centered_text(
        "PRESS SPACE TO RESTART",
        RESULT_HINT_FONT_SIZE,
        screen_height() * 0.78,
        GRAY,
    );
}

/// Текст с причиной окончания раунда по причинам проигрыша бойцов
fn lose_reason_text(
    red_lose: Option<LoseReason>,
    blue_lose: Option<LoseReason>,
) -> &'static str {
    match (red_lose, blue_lose) {
        // Оба бойца проиграли одновременно
        (Some(_), Some(_)) => "BOTH FIGHTERS FAILED",
        // Проиграл красный
        (Some(LoseReason::OutOfRing), None) => "RED LEFT THE RING",
        (Some(LoseReason::Exhausted), None) => "RED IS EXHAUSTED",
        (Some(LoseReason::Defeated), None) => "RED IS DEFEATED",
        // Проиграл синий
        (None, Some(LoseReason::OutOfRing)) => "BLUE LEFT THE RING",
        (None, Some(LoseReason::Exhausted)) => "BLUE IS EXHAUSTED",
        (None, Some(LoseReason::Defeated)) => "BLUE IS DEFEATED",
        // Никто не проиграл: истекло время
        (None, None) => "TIME IS UP",
    }
}

/// Рисует текст по центру экрана по горизонтали.
/// `y` — базовая линия текста: сам текст рисуется выше неё.
fn draw_centered_text(text: &str, font_size: u16, y: f32, color: Color) {
    let width = measure_text(text, None, font_size, 1.0).width;

    draw_text(text, (screen_width() - width) * 0.5, y, font_size as f32, color);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::round::LoseReason;

    /// Текст причины окончания однозначно определяется причинами проигрыша
    #[test]
    fn lose_reason_text_matches_reasons() {
        assert_eq!(lose_reason_text(None, None), "TIME IS UP");
        assert_eq!(
            lose_reason_text(Some(LoseReason::OutOfRing), None),
            "RED LEFT THE RING"
        );
        assert_eq!(
            lose_reason_text(Some(LoseReason::Exhausted), None),
            "RED IS EXHAUSTED"
        );
        assert_eq!(
            lose_reason_text(None, Some(LoseReason::OutOfRing)),
            "BLUE LEFT THE RING"
        );
        assert_eq!(
            lose_reason_text(None, Some(LoseReason::Exhausted)),
            "BLUE IS EXHAUSTED"
        );
        assert_eq!(
            lose_reason_text(Some(LoseReason::OutOfRing), Some(LoseReason::Exhausted)),
            "BOTH FIGHTERS FAILED"
        );
    }

    /// Радиус головы бойца в пикселях экрана
    fn head_radius_px() -> f32 {
        HEAD_RADIUS * PIXELS_PER_WORLD_UNIT
    }

    /// Полоса состояния по длине составляет 0.8 от диаметра головы
    #[test]
    fn state_bar_length_is_eighty_percent_of_head_diameter() {
        let head = Vec2::new(300., 200.);
        let (background, _, _) = state_bar_rects(head, 1., 1.);

        assert!((background.w - head_radius_px() * 2. * 0.8).abs() < 1e-4);
    }

    /// Полоса состояния центрирована по голове и лежит на её диаметре
    #[test]
    fn state_bar_is_centered_on_head() {
        let head = Vec2::new(300., 200.);
        let (background, stamina, energy) = state_bar_rects(head, 1., 1.);

        for rect in [background, stamina, energy] {
            assert!((rect.center().x - head.x).abs() < 1e-4);
            assert!((rect.center().y - head.y).abs() < 1e-4);
        }
    }

    /// Длины полос выносливости и энергии пропорциональны их значениям
    #[test]
    fn state_bar_layers_follow_stamina_and_energy() {
        let head = Vec2::new(300., 200.);
        let (background, stamina, energy) = state_bar_rects(head, 0.5, 0.25);

        assert!((stamina.w - background.w * 0.5).abs() < 1e-4);
        assert!((energy.w - background.w * 0.25).abs() < 1e-4);
    }

    /// Полосы растут от одного левого края
    #[test]
    fn state_bar_layers_share_left_edge() {
        let head = Vec2::new(300., 200.);
        let (background, stamina, energy) = state_bar_rects(head, 0.5, 0.25);

        assert_eq!(background.x, stamina.x);
        assert_eq!(background.x, energy.x);
        assert_eq!(background.y, stamina.y);
        assert_eq!(background.y, energy.y);
    }

    /// Значения шкал обрезаются в диапазон 0..1
    #[test]
    fn state_bar_clamps_values() {
        let head = Vec2::new(300., 200.);
        let (background, stamina, energy) = state_bar_rects(head, 2., -1.);

        assert!((stamina.w - background.w).abs() < 1e-4);
        assert_eq!(energy.w, 0.);
    }

    /// При нулевом состоянии полосы пусты, а фон остаётся полным
    #[test]
    fn state_bar_is_empty_when_no_stamina_and_energy() {
        let head = Vec2::new(300., 200.);
        let (background, stamina, energy) = state_bar_rects(head, 0., 0.);

        assert!(background.w > 0.);
        assert_eq!(stamina.w, 0.);
        assert_eq!(energy.w, 0.);
    }
}
