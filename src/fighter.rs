use glam::Vec2;

use crate::{
    fighter::{
        action::{Action, SwordAction},
        brain::{Brain, BrainEvent},
        sword::Sword,
    },
    math::{DT, PI, TAU, add_vector_with_min, normalize_angle, shorten_vector, step_to_zero},
    round::situation::Situation,
};

pub mod action;
pub mod brain;
pub mod getters;
pub mod sword;

// ----- Геометрия тела -----

/// Полуось туловища бойца вдоль локальной оси X (поперёк плеч), мировые единицы
pub const TORSO_WIDTH: f32 = 1.5;
/// Полуось туловища бойца вдоль локальной оси Y (вдоль взгляда), мировые единицы
pub const TORSO_HEIGHT: f32 = 0.7;
/// Число вершин, которыми аппроксимируется эллипс туловища
pub const TORSO_ELLIPSE_SEGMENTS: usize = 32;
/// Радиус головы бойца, мировые единицы
pub const HEAD_RADIUS: f32 = 0.8;
/// Радиус левой руки бойца, мировые единицы
pub const FIST_RADIUS: f32 = 0.2;

/// Смещение головы по локальной оси X (вправо), мировые единицы
pub const HEAD_OFFSET_X: f32 = 0.;
/// Смещение головы по локальной оси Y (вперёд), мировые единицы
pub const HEAD_OFFSET_Y: f32 = 0.2;
/// Смещение головы в локальной системе координат бойца, мировые единицы
pub const HEAD_LOCAL_OFFSET: Vec2 = Vec2::new(HEAD_OFFSET_X, HEAD_OFFSET_Y);

/// Смещение левой руки по локальной оси X (вправо), мировые единицы
pub const FIST_OFFSET_X: f32 = -1.5;
/// Смещение левой руки по локальной оси Y (вперёд), мировые единицы
pub const FIST_OFFSET_Y: f32 = 0.6;
/// Смещение левой руки в локальной системе координат бойца, мировые единицы
pub const FIST_LOCAL_OFFSET: Vec2 = Vec2::new(FIST_OFFSET_X, FIST_OFFSET_Y);

/// Смещение рукояти меча по локальной оси X (вправо), мировые единицы
pub const SWORD_OFFSET_X: f32 = 1.4;
/// Смещение рукояти меча по локальной оси Y (вперёд), мировые единицы
pub const SWORD_OFFSET_Y: f32 = 0.5;
/// Смещение рукояти меча в локальной системе координат бойца, мировые единицы
pub const SWORD_LOCAL_OFFSET: Vec2 = Vec2::new(SWORD_OFFSET_X, SWORD_OFFSET_Y);

/// Вершины эллипса туловища в локальной системе координат бойца.
/// Обход против часовой стрелки, вершины лежат на эллипсе.
pub fn torso_local_points() -> [Vec2; TORSO_ELLIPSE_SEGMENTS] {
    let mut points = [Vec2::ZERO; TORSO_ELLIPSE_SEGMENTS];

    for (i, point) in points.iter_mut().enumerate() {
        let angle = i as f32 / TORSO_ELLIPSE_SEGMENTS as f32 * TAU;

        *point = Vec2::new(TORSO_WIDTH * angle.cos(), TORSO_HEIGHT * angle.sin());
    }

    points
}

// ----- Перемещение бойца -----

/// Линейное ускорение тела при движении, мировых единиц в секунду в квадрате
pub const MOVE_ACCELERATION: f32 = 5.;
/// Предел линейной скорости бойца, мировых единиц в секунду
pub const MAX_MOVE_SPEED: f32 = 3.;
/// Расход энергии на единицу линейного ускорения
pub const MOVE_ENERGY_COST: f32 = 0.01;
/// Линейное трение при отсутствии команд, мировых единиц в секунду в квадрате
pub const MOVE_FRICTION: f32 = 0.4 * 9.8;

/// Угловое ускорение при повороте, радиан в секунду в квадрате
pub const TURN_ACCELERATION: f32 = PI;
/// Предел угловой скорости бойца, радиан в секунду
pub const MAX_TURN_SPEED: f32 = PI * 0.75;
/// Расход энергии на единицу углового ускорения
pub const TURN_ENERGY_COST: f32 = 0.01;
/// Угловое трение при отсутствии команд, радиан в секунду в квадрате
pub const TURN_FRICTION: f32 = TAU * 0.67;
/// Коэффициент пропорционального регулятора поворота к целевому углу.
/// Желаемая угловая скорость равна TURN_GAIN * ошибка по углу.
/// Значение подобрано так, чтобы доворот не перелетал цель: при 2.5 перелёта нет,
/// при 4 и больше боец проскакивает мимо цели на 10-18 градусов
pub const TURN_GAIN: f32 = 2.5;

// ----- Энергия -----

/// Скорость восстановления энергии в покое, единиц шкалы в секунду
pub const ENERGY_RECOVERY_RATE: f32 = 0.08;
/// Расход энергии на колющий удар
pub const PRICK_ENERGY_COST: f32 = 0.05;
/// Расход энергии на рубящий удар
pub const CHOK_ENERGY_COST: f32 = 0.07;

// ----- Соглашения об углах -----

/// Сдвиг внутреннего угла взгляда к стандартному углу в системе координат:
/// внутренний угол 0 соответствует взгляду вверх, стандартный — вправо
pub const FACING_ANGLE_OFFSET: f32 = 90.0f32.to_radians();
/// Угол направления «вперёд» в локальной системе координат бойца:
/// локальная ось Y смотрит вперёд, поэтому это 90° от локальной оси X
pub const LOCAL_FORWARD_ANGLE: f32 = 90.0f32.to_radians();

pub struct Fighter {
    brain: Box<dyn Brain>,
    sword: Sword,
    /// Позиция
    position: Vec2,
    /// Скорость (мировые единицы в секунду)
    velocity: Vec2,
    /// Направление взгляда в радианах, идет против часовой стрелки, при нуле направлен вверх
    facing: f32,
    /// Угловая скорость в радианах в секунду
    angle_speed: f32,

    energy: f32,
    stamina: f32,
    shock: f32,
    /// Боец был отправлен в нокаут: выносливость уже опустилась до нуля
    knocked_out: bool,
    /// Выносливость кончилась от полученного удара, а не от перерасхода энергии
    knocked_out_by_damage: bool,
}

impl Fighter {
    pub fn new(brain: Box<dyn Brain>) -> Self {
        Self {
            brain,
            sword: Sword::new(),
            position: Vec2::ZERO,
            facing: 0.,
            velocity: Vec2::ZERO,
            angle_speed: 0.,

            energy: 1.,
            stamina: 1.,
            shock: 0.,
            knocked_out: false,
            knocked_out_by_damage: false,
        }
    }

    pub fn tick(&mut self) {
        self.sword.tick();
        self.unshock();

        self.position += self.velocity * DT;
        self.facing += self.angle_speed * DT;
        self.facing = normalize_angle(self.facing);
    }

    pub fn get_actions(&mut self, situation: Situation) -> Option<Action> {
        if self.shock == 0. {
            Some(self.brain.get_actions(situation))
        } else {
            None
        }
    }

    pub fn do_actions(&mut self, action: Action) {
        // Энергия до действий: по разнице считается расход за тик
        let energy_before = self.energy;

        let acceleration_vector =
            self.facing_to_matrix() * action.into_vector() * MOVE_ACCELERATION * DT;
        if acceleration_vector != Vec2::ZERO {
            self.velocity = add_vector_with_min(self.velocity, acceleration_vector, MAX_MOVE_SPEED);
            self.expend_energy(acceleration_vector.length() * MOVE_ENERGY_COST);
        } else {
            // Трение
            self.velocity = shorten_vector(self.velocity, MOVE_FRICTION * DT);
        }

        let angle_acceleration = match action.turn_to {
            Some(target) => {
                // Положительная ошибка — нужно повернуть против часовой стрелки
                let delta = normalize_angle(target - self.facing());
                // Желаемая угловая скорость пропорциональна ошибке
                let desired_speed = (delta * TURN_GAIN).clamp(-MAX_TURN_SPEED, MAX_TURN_SPEED);
                // Разгон ограничен угловым ускорением бойца
                (desired_speed - self.angle_speed)
                    .clamp(-TURN_ACCELERATION * DT, TURN_ACCELERATION * DT)
            }
            None => action.rotation() * TURN_ACCELERATION * DT,
        };

        if angle_acceleration != 0. || action.turn_to.is_some() {
            self.angle_speed =
                (self.angle_speed + angle_acceleration).clamp(-MAX_TURN_SPEED, MAX_TURN_SPEED);
            self.expend_energy(angle_acceleration.abs() * TURN_ENERGY_COST);
        } else {
            // Угловое трение
            self.angle_speed = step_to_zero(self.angle_speed, TURN_FRICTION * DT);
        }

        match action.sword {
            SwordAction::None => {}
            SwordAction::Prick => self.prick(),
            SwordAction::Chok => self.chok(),
        }

        // Пока боец ничего не делает, его энергия повышается
        if acceleration_vector == Vec2::ZERO
            && angle_acceleration == 0.
            && matches!(action.sword, SwordAction::None)
        {
            self.heal();
        }

        // Расход энергии за тик сообщается мозгу: он решает, потрачена ли
        // энергия впустую
        let spent = (energy_before - self.energy).max(0.);
        if spent > 0. {
            self.emit_event(BrainEvent::EnergySpent { amount: spent });
        }
    }

    pub fn rotate(&mut self, alpha: f32) {
        self.facing += alpha;
        self.facing = normalize_angle(self.facing);
    }

    pub fn prick(&mut self) {
        if self.sword.is_ready() {
            self.sword.prick();
            self.expend_energy(PRICK_ENERGY_COST);
        }
    }

    pub fn chok(&mut self) {
        if self.sword.is_ready() {
            self.sword.chok();
            self.expend_energy(CHOK_ENERGY_COST);
        }
    }

    fn unshock(&mut self) {
        self.shock = (self.shock - DT).max(0.);
    }

    fn heal(&mut self) {
        let mut got_energy = ENERGY_RECOVERY_RATE * DT;

        let energy_need = (self.stamina - self.energy).max(0.);
        let energy_used = energy_need.min(got_energy);
        self.energy += energy_used;
        got_energy -= energy_used;

        if got_energy > 0. {
            self.energy += got_energy / 2.;
            self.stamina += got_energy / 2.;
        }

        self.stamina = self.stamina.min(1.);
        self.energy = self.energy.min(1.);
    }

    pub fn get_shock(&mut self, time: f32) {
        self.shock += time;
        // Мозг узнаёт о шоке: выполнение реакции прерывается
        self.emit_event(BrainEvent::Shocked);
    }

    /// Передать событие мозгу бойца
    pub fn emit_event(&mut self, event: BrainEvent) {
        self.brain.on_event(event);
    }

    /// Сообщить мозгу, что раунд закончился
    pub fn notify_round_end(&mut self) {
        self.brain.on_round_end();
    }

    pub fn deal_damage(&mut self, damage: f32) {
        self.stamina -= damage;
        self.energy = self.energy.min(self.stamina);
        // Выносливость дошла до нуля — нокаут, он необратим
        self.check_knockout(true);
    }

    pub fn expend_energy(&mut self, e: f32) {
        self.energy -= e;

        // Перерасход энергии сверх нуля уходит из выносливости
        self.stamina += self.energy.min(0.);
        self.energy = self.energy.max(0.);
        // Выносливость дошла до нуля — нокаут, он необратим
        self.check_knockout(false);
    }

    /// Фиксирует нокаут, если выносливость кончилась
    ///
    /// Причина запоминается только в первый раз: нокаут необратим, а значит
    /// и причина, которая его вызвала, уже не может измениться. Поэтому удар,
    /// прилетевший в том же тике после нокаута от перерасхода энергии, причину
    /// не переписывает.
    fn check_knockout(&mut self, by_damage: bool) {
        if !self.knocked_out && self.stamina <= 0. {
            self.knocked_out = true;
            self.knocked_out_by_damage = by_damage;
        }
    }

    pub fn set_position(&mut self, v: Vec2) {
        self.position = v;
    }

    /// Прибавляет скорость от импульса, итоговая скорость ограничена MAX_MOVE_SPEED
    pub fn add_velocity(&mut self, dv: Vec2) {
        // Именно clamp_length, а не add_vector_with_min: тот нормализует сумму
        // и обрезает её длину, из-за чего боец меняет направление движения,
        // а не просто замедляется
        self.set_velocity((self.velocity + dv).clamp_length(0., MAX_MOVE_SPEED));
    }

    /// Задаёт скорость бойца
    pub fn set_velocity(&mut self, v: Vec2) {
        self.velocity = v;
    }

    /// Отражает меч: удар сбит, клинок уходит в исходное положение
    pub fn reflect_sword(&mut self) {
        self.sword.retract();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        fighter::{action::Direction, brain::doll::Doll},
        math::approx_eq,
    };

    fn fighter() -> Fighter {
        Fighter::new(Box::new(Doll))
    }

    /// Импульс не выводит бойца за предел скорости
    #[test]
    fn add_velocity_respects_max_speed() {
        let mut f = fighter();

        f.set_velocity(Vec2::new(0., MAX_MOVE_SPEED));
        f.add_velocity(Vec2::new(0., 100.));

        assert!((f.velocity().length() - MAX_MOVE_SPEED).abs() < 1e-5);
    }

    /// Отражённый меч уходит в стойку
    #[test]
    fn reflect_sword_starts_retraction() {
        let mut f = fighter();

        f.prick();
        assert!(f.is_sword_attacking());

        f.reflect_sword();

        assert!(f.is_sword_retracts());
        assert!(!f.is_sword_attacking());
    }

    /// Действие «вперёд» ускоряет бойца вдоль его взгляда
    #[test]
    fn forward_moves_along_gaze() {
        let mut f = fighter();

        let mut action = Action::default();
        action.forward = Direction::Positive;
        f.do_actions(action);

        // При facing = 0 взгляд направлен вверх
        assert!((f.velocity.normalize_or_zero() - Vec2::new(0., 1.)).length() < 1e-4);
        // За тик боец набирает ускорение
        assert!((f.velocity.length() - MOVE_ACCELERATION * DT).abs() < 1e-5);
    }

    /// Действие «в сторону» ускоряет бойца в его правую сторону
    #[test]
    fn side_moves_to_the_right() {
        let mut f = fighter();

        let mut action = Action::default();
        action.side = Direction::Positive;
        f.do_actions(action);

        assert!((f.velocity.normalize_or_zero() - Vec2::new(1., 0.)).length() < 1e-4);
    }

    /// Направление движения следует за поворотом тела
    #[test]
    fn forward_follows_rotated_body() {
        let mut f = fighter();
        f.rotate(-PI / 2.);

        let mut action = Action::default();
        action.forward = Direction::Positive;
        f.do_actions(action);

        // Взгляд направлен вправо
        assert!((f.velocity.normalize_or_zero() - Vec2::new(1., 0.)).length() < 1e-4);
    }

    /// Поворот в разные стороны даёт разный знак угловой скорости,
    /// а модуль угловой скорости не превышает предел
    #[test]
    fn rotation_direction_and_limit() {
        let mut f = fighter();

        for _ in 0..100 {
            let mut action = Action::default();
            action.rotate = Direction::Negative;
            f.do_actions(action);
        }

        assert!(f.angle_speed < 0.);
        assert!((f.angle_speed + MAX_TURN_SPEED).abs() < 1e-4);

        let mut f = fighter();

        for _ in 0..100 {
            let mut action = Action::default();
            action.rotate = Direction::Positive;
            f.do_actions(action);
        }

        assert!(f.angle_speed > 0.);
        assert!((f.angle_speed - MAX_TURN_SPEED).abs() < 1e-4);
    }

    /// Боец с целевым углом доворачивает взгляд к нему
    #[test]
    fn servo_turns_to_target_angle() {
        let mut f = fighter();

        for _ in 0..300 {
            let mut action = Action::default();
            // Цель — направление вправо (стандартный угол 0)
            action.turn_to = Some(0.);

            f.do_actions(action);
            f.tick();
        }

        assert!(approx_eq(normalize_angle(f.facing()), 0.));
    }

    /// Серво не даёт угловой скорости превысить предел
    #[test]
    fn servo_respects_max_turn_speed() {
        let mut f = fighter();

        let mut max_speed: f32 = 0.;
        for _ in 0..200 {
            let mut action = Action::default();
            // Разворот на 180 градусов
            action.turn_to = Some(PI);

            f.do_actions(action);
            f.tick();

            max_speed = max_speed.max(f.angle_speed().abs());
        }

        assert!(max_speed <= MAX_TURN_SPEED + 1e-5);
    }

    /// Серво держит угловую скорость, а не гасит её угловым трением
    #[test]
    fn servo_keeps_angular_speed() {
        let mut f = fighter();

        // Цель постоянно уходит вперёд, поэтому ошибка не уменьшается
        for _ in 0..200 {
            let mut action = Action::default();
            action.turn_to = Some(f.facing() + 1.);
            f.do_actions(action);
            f.tick();
        }

        // Боец разогнался до предела и держит скорость
        assert!((f.angle_speed() - MAX_TURN_SPEED).abs() < 1e-4);
    }

    /// Перерасход энергии сверх нуля уходит из выносливости
    #[test]
    fn expend_energy_drains_stamina() {
        let mut f = fighter();
        f.energy = 0.05;
        f.stamina = 1.;

        f.expend_energy(0.1);

        // Энергия не уходит в минус
        assert_eq!(f.energy, 0.);
        // Выносливость падает ровно на величину перерасхода
        assert!((f.stamina - 0.95).abs() < 1e-5);

        // Повторный расход снова стоит столько же
        f.expend_energy(0.1);

        assert!((f.stamina - 0.85).abs() < 1e-5);
    }

    /// Пока боец ничего не делает, энергия повышается
    #[test]
    fn idle_fighter_recovers_energy() {
        let mut f = fighter();
        f.energy = 0.5;
        f.stamina = 1.;

        for _ in 0..10 {
            f.do_actions(Action::default());
        }

        assert!(f.energy > 0.5);
        // Энергия не может превышать выносливость
        assert!(f.energy <= f.stamina);
    }

    /// Действующий боец энергию не восстанавливает
    #[test]
    fn busy_fighter_does_not_recover_energy() {
        let mut f = fighter();
        f.energy = 0.5;
        f.stamina = 1.;

        for _ in 0..10 {
            let mut action = Action::default();
            action.forward = Direction::Positive;
            f.do_actions(action);
        }

        assert!(f.energy < 0.5);
    }

    /// Вершины эллипса туловища лежат на эллипсе
    #[test]
    fn torso_points_lie_on_ellipse() {
        for point in torso_local_points() {
            let x = point.x / TORSO_WIDTH;
            let y = point.y / TORSO_HEIGHT;

            assert!(
                (x * x + y * y - 1.).abs() < 1e-5,
                "вершина не на эллипсе: {point:?}"
            );
        }
    }

    /// Эллипс туловища шире, чем глубже
    #[test]
    fn torso_is_wider_than_deep() {
        assert!(TORSO_WIDTH > TORSO_HEIGHT);
    }

    /// Нокаут необратим: восстановление выносливости его не отменяет
    #[test]
    fn knockout_is_irreversible() {
        let mut f = fighter();
        f.deal_damage(1.);

        assert!(f.is_knockout(), "выносливость дошла до нуля — нокаут");

        // Боец отдыхает: выносливость растёт, но нокаут остаётся
        for _ in 0..100 {
            f.do_actions(Action::default());
        }

        assert!(f.stamina() > 0., "выносливость должна восстановиться");
        assert!(f.is_knockout(), "нокаут необратим");
    }

    /// До нокаута причина не задана
    #[test]
    fn knockout_cause_is_unknown_before_knockout() {
        let mut f = fighter();

        f.deal_damage(0.5);

        assert!(!f.is_knockout());
        assert!(!f.knocked_out_by_damage());
    }

    /// Нокаут от удара запоминается как нокаут от урона
    #[test]
    fn knockout_by_damage_is_remembered() {
        let mut f = fighter();

        f.deal_damage(1.);

        assert!(f.is_knockout());
        assert!(f.knocked_out_by_damage(), "выносливость кончилась от урона");
    }

    /// Нокаут от перерасхода энергии — не от урона
    #[test]
    fn knockout_by_energy_is_not_damage() {
        let mut f = fighter();

        // Энергии 1, а расход 2: перерасход уходит из выносливости
        f.expend_energy(2.);

        assert!(f.is_knockout());
        assert!(!f.knocked_out_by_damage(), "выносливость кончилась от энергии");
    }

    /// Причина нокаута не меняется: удар после нокаута от энергии ничего не переписывает
    #[test]
    fn knockout_cause_keeps_first_reason() {
        let mut f = fighter();

        f.expend_energy(2.);
        f.deal_damage(1.);

        assert!(f.is_knockout());
        assert!(!f.knocked_out_by_damage(), "причина осталась прежней");

        let mut g = fighter();

        g.deal_damage(1.);
        g.expend_energy(2.);

        assert!(g.is_knockout());
        assert!(g.knocked_out_by_damage(), "причина осталась прежней");
    }
}
