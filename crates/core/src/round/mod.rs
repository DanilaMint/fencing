pub mod situation;

use fastrand::Rng;
use glam::Vec2;

use crate::{
    collision::resolve_collisions,
    brain::Brain,
    fighter::Fighter,
    math::{DT, FRAC_PI_2, TPS},
    round::situation::analyze_situation,
};

/// Радиус ринга, мировые единицы
pub const RING_RADIUS: f32 = 6.0;
/// Начальное смещение бойца от центра ринга по оси X, мировые единицы
pub const INITIAL_POSITION_X: f32 = 2.0;
/// Максимальный разброс начальной позиции по оси Y, мировые единицы
pub const INITIAL_POSITION_SPREAD_Y: f32 = 0.5;
/// Максимальный разброс начального направления взгляда, градусы
pub const INITIAL_FACING_JITTER: f32 = 20.0;

/// Длительность раунда, секунды
pub const ROUND_DURATION: f32 = 120.;
/// Длительность раунда в тиках: целое число, поэтому конец раунда
/// определяется точно, без накопления ошибки в f32
pub const ROUND_DURATION_TICKS: u32 = (ROUND_DURATION * TPS as f32) as u32;

/// Причина, по которой боец проиграл
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoseReason {
    /// Боец вышел за пределы ринга
    OutOfRing,
    /// Боец отправлен в нокаут: выносливость опустилась до нуля от перерасхода энергии
    Exhausted,
    /// Боец отправлен в нокаут: выносливость опустилась до нуля от полученного удара
    Defeated,
}

/// Результат раунда
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoundResult {
    /// Победа красного бойца
    RedWin,
    /// Победа синего бойца
    BlueWin,
    /// Ничья
    Draw,
}

pub struct Round {
    red: Fighter,
    blue: Fighter,

    /// Число прошедших тиков раунда
    ticks: u32,
    ring_radius: f32,

    /// Результат раунда: None, пока раунд не закончился
    result: Option<RoundResult>,
    /// Причина поражения красного бойца
    red_lose: Option<LoseReason>,
    /// Причина поражения синего бойца
    blue_lose: Option<LoseReason>,
}

impl Round {
    pub fn new(red: Box<dyn Brain>, blue: Box<dyn Brain>) -> Self {
        let mut rng = Rng::new();
        rng.seed(fastrand::u64(0..=u64::MAX));

        let mut red = Fighter::new(red);
        red.set_position(Vec2::new(
            -INITIAL_POSITION_X,
            INITIAL_POSITION_SPREAD_Y * (rng.f32_inclusive() * 2. - 1.),
        ));
        red.rotate(
            -FRAC_PI_2 + INITIAL_FACING_JITTER.to_radians() * (rng.f32_inclusive() * 2. - 1.),
        );

        let mut blue = Fighter::new(blue);
        blue.set_position(Vec2::new(
            INITIAL_POSITION_X,
            INITIAL_POSITION_SPREAD_Y * (rng.f32_inclusive() * 2. - 1.),
        ));
        blue.rotate(
            FRAC_PI_2 + INITIAL_FACING_JITTER.to_radians() * (rng.f32_inclusive() * 2. - 1.),
        );

        Self {
            red,
            blue,

            ticks: 0,
            ring_radius: RING_RADIUS,

            result: None,
            red_lose: None,
            blue_lose: None,
        }
    }

    /// Полный цикл тика: команды мозгов, их выполнение, движение бойцов и столкновения.
    /// После окончания раунда тик ничего не делает: бойцы замирают,
    /// а результат остаётся неизменным.
    pub fn tick(&mut self) {
        if self.is_finished() {
            return;
        }

        self.ticks += 1;

        // Ситуации для мозгов: состояние на начало тика
        let red_situation = analyze_situation(&self.red, &self.blue, self.ring_radius);
        let blue_situation = analyze_situation(&self.blue, &self.red, self.ring_radius);

        let red_command = self.red.get_actions(red_situation);
        let blue_command = self.blue.get_actions(blue_situation);

        // Выполнение команд: ускорение, поворот, замах, расход энергии.
        // Если боец в шоке, он не отдаёт команд и просто тормозит трением.
        self.red.do_actions(red_command.unwrap_or_default());
        self.blue.do_actions(blue_command.unwrap_or_default());

        // Движение бойцов и продвижение клинка
        self.red.tick();
        self.blue.tick();

        // Столкновения: клинки, удары и расталкивание тел
        resolve_collisions(&mut self.red, &mut self.blue);

        // Условия окончания раунда проверяются после физики:
        // к этому моменту позиции и выносливость бойцов уже изменились
        self.check_end();
    }

    /// Проверяет условия окончания раунда и фиксирует результат.
    /// Вызывается в конце тика, когда состояние бойцов уже актуально.
    fn check_end(&mut self) {
        let red_lose = self.lose_reason(&self.red);
        let blue_lose = self.lose_reason(&self.blue);

        self.red_lose = red_lose;
        self.blue_lose = blue_lose;

        // Сначала проверяются бойцы, потом время: нарушение правил
        // на площадке важнее, чем истечение времени раунда
        self.result = match (red_lose, blue_lose) {
            // Оба бойца проиграли одновременно — ничья
            (Some(_), Some(_)) => Some(RoundResult::Draw),
            // Проиграл красный — побеждает синий
            (Some(_), None) => Some(RoundResult::BlueWin),
            // Проиграл синий — побеждает красный
            (None, Some(_)) => Some(RoundResult::RedWin),
            // Никто не проиграл: осталось проверить время
            (None, None) => {
                if self.ticks >= ROUND_DURATION_TICKS {
                    Some(RoundResult::Draw)
                } else {
                    None
                }
            }
        };

        // Раунд закончился: мозги закрывают текущие реакции, чтобы их
        // результаты не смешались с результатами следующего боя
        if self.result.is_some() {
            self.red.notify_round_end();
            self.blue.notify_round_end();
        }
    }

    /// Причина поражения бойца. None, если боец пока не проиграл.
    fn lose_reason(&self, fighter: &Fighter) -> Option<LoseReason> {
        // Выход за кольцевую границу ринга — по центру туловища:
        // это та же точка, что используется в столкновениях и в
        // Situation::center_distance, поэтому ИИ и правило не расходятся
        if fighter.position().length() > self.ring_radius {
            return Some(LoseReason::OutOfRing);
        }

        // Нокаут: выносливость опустилась до нуля.
        // Если выносливость кончилась от удара — это победа за счёт удара,
        // а не от собственной нерасторопности
        if fighter.is_knockout() {
            return Some(if fighter.knocked_out_by_damage() {
                LoseReason::Defeated
            } else {
                LoseReason::Exhausted
            });
        }

        None
    }

    pub fn red_fighter(&self) -> &Fighter {
        &self.red
    }

    pub fn blue_fighter(&self) -> &Fighter {
        &self.blue
    }

    pub fn ring_radius(&self) -> f32 {
        self.ring_radius
    }

    /// Число прошедших тиков раунда
    pub fn ticks(&self) -> u32 {
        self.ticks
    }

    /// Время раунда, секунды
    pub fn time(&self) -> f32 {
        self.ticks as f32 * DT
    }

    /// Результат раунда. None, пока раунд не закончился
    pub fn result(&self) -> Option<RoundResult> {
        self.result
    }

    /// Закончился ли раунд
    pub fn is_finished(&self) -> bool {
        self.result.is_some()
    }

    /// Причина поражения красного бойца
    pub fn red_lose_reason(&self) -> Option<LoseReason> {
        self.red_lose
    }

    /// Причина поражения синего бойца
    pub fn blue_lose_reason(&self) -> Option<LoseReason> {
        self.blue_lose
    }

    /// Извлекает мозги бойцов, чтобы переиспользовать их в следующем раунде.
    /// Раунд при этом потребляется: вернуть его уже нельзя.
    pub fn extract_brains(self) -> (Box<dyn Brain>, Box<dyn Brain>) {
        let Round { red, blue, .. } = self;

        (red.extract_brain(), blue.extract_brain())
    }
}

#[cfg(test)]
mod tests {
    use glam::Vec2;

    use crate::{
        brain::{Brain, doll::Doll},
        fighter::{
            Fighter,
            action::{Action, Direction, SwordAction},
        },
        math::PI,
        round::situation::Situation,
        round::{LoseReason, ROUND_DURATION_TICKS, Round, RoundResult},
    };

    /// Мозг, который всё время идёт вперёд
    struct Forward;

    impl Brain for Forward {
        fn get_actions(&mut self, _situation: Situation) -> Action {
            Action {
                forward: Direction::Positive,
                ..Default::default()
            }
        }
    }

    /// Мозг, который всё время колет
    struct Prick;

    impl Brain for Prick {
        fn get_actions(&mut self, _situation: Situation) -> Action {
            Action {
                sword: SwordAction::Prick,
                ..Default::default()
            }
        }
    }

    fn round_with(red: Box<dyn Brain>, blue: Box<dyn Brain>) -> Round {
        Round::new(red, blue)
    }

    #[test]
    fn tick_executes_commands() {
        let mut round = round_with(Box::new(Forward), Box::new(Doll));

        let start = round.red.position();
        // Направление движения задаётся направлением взгляда бойца
        let forward = round.red.facing_to_matrix() * Vec2::new(0., 1.);

        for _ in 0..20 {
            round.tick();
        }

        let displacement = round.red.position() - start;
        assert!(
            displacement.dot(forward) > 0.1,
            "боец должен двигаться вперёд, смещение: {displacement:?}"
        );
    }

    #[test]
    fn tick_resolves_collisions() {
        let mut round = round_with(Box::new(Doll), Box::new(Doll));

        // Ставим бойцов друг на друга
        round.red.set_position(Vec2::ZERO);
        round.blue.set_position(Vec2::ZERO);

        // Бойцы с разным направлением взгляда, поэтому минимальное расстояние
        // считается по проекции туловищ на ось раздвига
        let axis = Vec2::new(1., 0.);
        let expected = round.red.torso_support_along(axis) + round.blue.torso_support_along(-axis);

        round.tick();

        let distance = round.red.position().distance(round.blue.position());
        assert!(
            distance >= expected - 1e-3,
            "бойцы должны быть разведены, расстояние: {distance}, ожидалось: {expected}"
        );
    }

    #[test]
    fn tick_does_not_panic() {
        let mut round = round_with(Box::new(Doll), Box::new(Doll));

        for _ in 0..100 {
            round.tick();
        }
    }

    /// Полный цикл: команда мозга доходит до удара и снижает выносливость противника
    #[test]
    fn tick_applies_damage() {
        let mut round = round_with(Box::new(Prick), Box::new(Doll));

        // Атакующий стоит в начале координат и смотрит вверх
        round.red.set_position(Vec2::ZERO);
        round.red.rotate(PI / 2. - round.red.facing());
        // Противник спиной к атакующему: удар приходится в туловище
        round.blue.set_position(Vec2::new(0.95, 4.0));

        for _ in 0..60 {
            round.tick();
        }

        assert!(
            round.blue.stamina() < 1.,
            "удар должен снизить выносливость противника"
        );
    }

    /// Раунд с заданным радиусом ринга: бойцы стоят внутри и ничего не делают
    fn idle_round(ring_radius: f32) -> Round {
        let mut round = round_with(Box::new(Doll), Box::new(Doll));
        round.ring_radius = ring_radius;
        // Бойцы далеко друг от друга, чтобы их не расталкивало
        round.red.set_position(Vec2::new(-2., 0.));
        round.blue.set_position(Vec2::new(2., 0.));

        round
    }

    /// Отправляет бойца в нокаут
    fn knock_out(fighter: &mut Fighter) {
        fighter.deal_damage(10.);
    }

    /// Боец, вышедший за кольцевую границу ринга, проигрывает
    #[test]
    fn fighter_out_of_ring_loses() {
        let mut round = idle_round(5.);
        round.red.set_position(Vec2::new(-6., 0.));

        round.tick();

        assert_eq!(round.result(), Some(RoundResult::BlueWin));
        assert_eq!(round.red_lose_reason(), Some(LoseReason::OutOfRing));
        assert_eq!(round.blue_lose_reason(), None);
        assert!(round.is_finished());
    }

    /// Боец, отправленный в нокаут, проигрывает
    #[test]
    fn knocked_out_fighter_loses() {
        let mut round = idle_round(5.);
        knock_out(&mut round.red);

        round.tick();

        assert_eq!(round.result(), Some(RoundResult::BlueWin));
        assert_eq!(round.red_lose_reason(), Some(LoseReason::Defeated));
        assert_eq!(round.blue_lose_reason(), None);
    }

    /// Боец, вымотавшийся без урона, проигрывает по истощению
    #[test]
    fn exhausted_fighter_loses() {
        let mut round = idle_round(5.);
        // Перерасход энергии уходит из выносливости
        round.red.expend_energy(2.);

        round.tick();

        assert_eq!(round.result(), Some(RoundResult::BlueWin));
        assert_eq!(round.red_lose_reason(), Some(LoseReason::Exhausted));
        assert_eq!(round.blue_lose_reason(), None);
    }

    /// Оба бойца одновременно вышли из ринга — ничья
    #[test]
    fn both_out_of_ring_is_draw() {
        let mut round = idle_round(5.);
        round.red.set_position(Vec2::new(-6., 0.));
        round.blue.set_position(Vec2::new(6., 0.));

        round.tick();

        assert_eq!(round.result(), Some(RoundResult::Draw));
        assert_eq!(round.red_lose_reason(), Some(LoseReason::OutOfRing));
        assert_eq!(round.blue_lose_reason(), Some(LoseReason::OutOfRing));
    }

    /// Оба бойца одновременно получили нокаут — ничья
    #[test]
    fn both_knocked_out_is_draw() {
        let mut round = idle_round(5.);
        knock_out(&mut round.red);
        knock_out(&mut round.blue);

        round.tick();

        assert_eq!(round.result(), Some(RoundResult::Draw));
        assert_eq!(round.red_lose_reason(), Some(LoseReason::Defeated));
        assert_eq!(round.blue_lose_reason(), Some(LoseReason::Defeated));
    }

    /// Один боец вышел из ринга, другой получил нокаут — ничья
    #[test]
    fn out_of_ring_and_knockout_is_draw() {
        let mut round = idle_round(5.);
        round.red.set_position(Vec2::new(-6., 0.));
        knock_out(&mut round.blue);

        round.tick();

        assert_eq!(round.result(), Some(RoundResult::Draw));
        assert_eq!(round.red_lose_reason(), Some(LoseReason::OutOfRing));
        assert_eq!(round.blue_lose_reason(), Some(LoseReason::Defeated));
    }

    /// По истечении времени раунда объявляется ничья
    #[test]
    fn time_limit_is_draw() {
        let mut round = idle_round(10.);

        for _ in 0..ROUND_DURATION_TICKS {
            round.tick();
        }

        assert_eq!(round.result(), Some(RoundResult::Draw));
        // Никто не проиграл: ничья именно по времени
        assert_eq!(round.red_lose_reason(), None);
        assert_eq!(round.blue_lose_reason(), None);
        assert!(round.is_finished());
    }

    /// До истечения времени раунд не заканчивается
    #[test]
    fn round_is_not_finished_before_time_limit() {
        let mut round = idle_round(10.);

        for _ in 0..ROUND_DURATION_TICKS - 1 {
            round.tick();
        }

        assert!(!round.is_finished());
        assert_eq!(round.result(), None);
    }

    /// После окончания раунда тик ничего не меняет
    #[test]
    fn finished_round_does_not_change() {
        let mut round = idle_round(5.);
        round.red.set_position(Vec2::new(-6., 0.));
        round.tick();
        assert!(round.is_finished());

        let time = round.time();
        let ticks = round.ticks();
        let red_position = round.red.position();
        let blue_position = round.blue.position();
        let result = round.result();

        for _ in 0..10 {
            round.tick();
        }

        assert_eq!(round.time(), time);
        assert_eq!(round.ticks(), ticks);
        assert_eq!(round.red.position(), red_position);
        assert_eq!(round.blue.position(), blue_position);
        assert_eq!(round.result(), result);
    }

    /// Новый раунд ещё не закончен
    #[test]
    fn new_round_is_not_finished() {
        let round = round_with(Box::new(Doll), Box::new(Doll));

        assert!(!round.is_finished());
        assert_eq!(round.result(), None);
        assert_eq!(round.time(), 0.);
        assert_eq!(round.ticks(), 0);
    }

    /// Боец ровно на границе ринга ещё считается внутри
    #[test]
    fn fighter_on_ring_border_is_inside() {
        let mut round = idle_round(5.);
        round.red.set_position(Vec2::new(-5., 0.));

        round.tick();

        assert!(!round.is_finished());
        assert_eq!(round.result(), None);
    }

    /// Мозги бойцов можно переиспользовать в новом раунде
    #[test]
    fn brains_can_be_reused() {
        let round = round_with(Box::new(Doll), Box::new(Doll));

        let (red_brain, blue_brain) = round.extract_brains();
        let next = round_with(red_brain, blue_brain);

        assert!(!next.is_finished());
        assert_eq!(next.ticks(), 0);
    }
}
