use macroquad::color::Color;

/// Цвет клинка меча во время удара
pub const SWORD_COLOR: Color = Color::from_rgba(200, 200, 200, 255);
/// Цвет клинка меча во время возврата в стойку
pub const RETRACT_SWORD_COLOR: Color = Color::from_rgba(100, 100, 100, 255);

/// Цвет обводки частей тела
pub const OUTLINE_COLOR: Color = Color::from_rgba(5, 5, 5, 255);

/// Цвет фона полосы состояния бойца
pub const STATE_BAR_BACKGROUND_COLOR: Color = Color::from_rgba(0, 0, 0, 255);
/// Цвет полосы выносливости
pub const STAMINA_BAR_COLOR: Color = Color::from_rgba(255, 255, 255, 255);
/// Цвет полосы энергии
pub const ENERGY_BAR_COLOR: Color = Color::from_rgba(0, 200, 0, 255);

/// Цвет красного бойца
pub const RED_COLOR: Color = Color::from_rgba(255, 10, 10, 255);
/// Цвет синего бойца
pub const BLUE_COLOR: Color = Color::from_rgba(10, 10, 255, 255);
