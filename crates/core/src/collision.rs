//! Столкновения бойцов: клинок против тела, клинок против клинка, тело против тела.
//!
//! Правила описаны в `readme.md`, раздел «Столкновение с мечом».
//! Сила удара и толчок считаются через импульс: сближение клинка и цели
//! умножается на массу клинка.

use glam::{Mat2, Vec2};

use crate::{
    brain::BrainEvent,
    fighter::{
        FIST_RADIUS, Fighter, HEAD_RADIUS, TORSO_ELLIPSE_SEGMENTS, TORSO_HEIGHT, TORSO_WIDTH,
        sword::BLADE_WIDTH, torso_local_points,
    },
};

/// Условная масса клинка: он легче тела бойца в несколько раз
pub const BLADE_MASS: f32 = 0.2;
/// Урон по выносливости на единицу импульса удара
pub const DAMAGE_PER_IMPULSE: f32 = 0.15;
/// Урон по левой руке в несколько раз меньше, чем по туловищу
pub const FIST_DAMAGE_FACTOR: f32 = 0.25;
/// Длительность эффекта шока при попадании в голову, секунды
pub const SHOCK_DURATION: f32 = 0.5;
/// Доля импульса, передаваемая телу противника
pub const IMPULSE_TO_VICTIM_SHARE: f32 = 0.6;
/// Доля импульса, возвращаемая атакующему (отдача)
pub const IMPULSE_TO_ATTACKER_SHARE: f32 = 0.25;
/// Доля импульса, разводящая бойцов при отражении ударов
pub const IMPULSE_ON_PARRY_SHARE: f32 = 0.25;
/// Коэффициент восстановления при столкновении тел
pub const BODY_RESTITUTION: f32 = 0.2;

/// Порог, ниже которого расстояние считается нулевым
const DISTANCE_EPSILON: f32 = 1e-4;

/// Часть тела бойца
#[derive(Clone, Copy, PartialEq)]
enum Part {
    Torso,
    Head,
    Fist,
}

/// Форма части тела для расчёта столкновений
enum Shape {
    /// Круг: голова и левая рука
    Circle {
        /// Центр круга в мировых координатах
        center: Vec2,
        /// Радиус круга
        radius: f32,
    },
    /// Эллипс туловища, вписанный в многоугольник
    Polygon {
        /// Центр эллипса в мировых координатах
        center: Vec2,
        /// Матрица перевода локальных координат бойца в мировые
        matrix: Mat2,
    },
}

/// Контакт клинка с поверхностью части тела
struct SurfaceContact {
    /// Внешняя нормаль поверхности в точке контакта
    normal: Vec2,
    /// Знаковое расстояние от поверхности до клинка:
    /// отрицательное значение означает, что клинок внутри фигуры
    distance: f32,
    /// Параметр точки контакта на клинке: 0 — рукоять, 1 — остриё
    blade_t: f32,
    /// Точка контакта в локальной системе координат бойца
    local: Vec2,
}

impl Part {
    /// Все части тела бойца в порядке проверки при ударе
    const ALL: [Part; 3] = [Part::Torso, Part::Head, Part::Fist];

    /// Форма части тела
    fn shape(self, f: &Fighter) -> Shape {
        match self {
            Part::Torso => Shape::Polygon {
                center: f.torso_position(),
                matrix: f.facing_to_matrix(),
            },
            Part::Head => Shape::Circle {
                center: f.head_position(),
                radius: HEAD_RADIUS,
            },
            Part::Fist => Shape::Circle {
                center: f.fist_position(),
                radius: FIST_RADIUS,
            },
        }
    }

    /// Скорость части тела в точке контакта.
    /// Точка задана в локальной системе координат бойца
    fn velocity_at(self, f: &Fighter, local: Vec2) -> Vec2 {
        match self {
            Part::Torso => f.point_velocity(local),
            Part::Head => f.head_velocity(),
            Part::Fist => f.fist_velocity(),
        }
    }

    /// Множитель урона по части тела
    fn damage_factor(self) -> f32 {
        match self {
            Part::Fist => FIST_DAMAGE_FACTOR,
            Part::Torso | Part::Head => 1.,
        }
    }
}

/// Контакт отрезка клинка с поверхностью части тела
fn surface_contact(shape: &Shape, hilt: Vec2, peak: Vec2) -> SurfaceContact {
    match shape {
        Shape::Circle { center, radius } => {
            let (point, t) = closest_point_on_segment(hilt, peak, *center);
            let offset = point - *center;

            SurfaceContact {
                normal: offset.normalize_or_zero(),
                distance: offset.length() - *radius,
                blade_t: t,
                local: Vec2::ZERO,
            }
        }
        Shape::Polygon { center, matrix } => {
            let local_hilt = matrix.transpose() * (hilt - *center);
            let local_peak = matrix.transpose() * (peak - *center);
            let points = torso_local_points();

            // Ищем ребро, до которого клинок ближе всего
            let mut best: Option<(f32, usize, Vec2, Vec2, f32)> = None;

            for i in 0..TORSO_ELLIPSE_SEGMENTS {
                let a = points[i];
                let b = points[(i + 1) % TORSO_ELLIPSE_SEGMENTS];

                let (blade_point, blade_t, edge_point, _, distance) =
                    closest_points_on_segments(local_hilt, local_peak, a, b);

                if best.is_none_or(|best| distance < best.0) {
                    best = Some((distance, i, blade_point, edge_point, blade_t));
                }
            }

            let (distance, index, blade_point, edge_point, blade_t) =
                best.expect("у эллипса есть рёбра");

            // Внешняя нормаль ребра: многоугольник обходится против часовой стрелки
            let edge = points[(index + 1) % TORSO_ELLIPSE_SEGMENTS] - points[index];
            let normal = Vec2::new(edge.y, -edge.x).normalize_or_zero();

            // Клинок внутри эллипса? Тогда расстояние до поверхности отрицательное
            let inside = (blade_point.x / TORSO_WIDTH).powi(2)
                + (blade_point.y / TORSO_HEIGHT).powi(2)
                < 1.;
            let distance = if inside { -distance } else { distance };

            SurfaceContact {
                normal: *matrix * normal,
                distance,
                blade_t,
                local: edge_point,
            }
        }
    }
}

/// Попадание клинка в часть тела
#[derive(Clone, Copy)]
struct Hit {
    part: Part,
    /// Нормаль от поверхности части тела к клинку
    normal: Vec2,
    /// Сила удара: скорость сближения, умноженная на массу клинка
    power: f32,
    /// Глубина проникновения клинка в часть тела
    depth: f32,
}

/// Разрешает все столкновения между двумя бойцами за один тик.
/// Вызывать после `Fighter::tick()` обоих бойцов.
pub fn resolve_collisions(a: &mut Fighter, b: &mut Fighter) {
    // Сначала клинки: отражённые удары не должны успеть нанести урон
    deflect_swords(a, b);
    strike(a, b);
    strike(b, a);
    separate_bodies(a, b);
}

/// Столкновение двух атакующих клинков: оба удара отражаются
fn deflect_swords(a: &mut Fighter, b: &mut Fighter) {
    // Если меч оппонента не наносит удар, столкновения нет
    if !a.is_sword_attacking() || !b.is_sword_attacking() {
        return;
    }

    let (a_hilt, a_peak) = a.sword_position();
    let (b_hilt, b_peak) = b.sword_position();

    let (a_point, a_t, b_point, b_t, distance) =
        closest_points_on_segments(a_hilt, a_peak, b_hilt, b_peak);

    if distance >= BLADE_WIDTH {
        return;
    }

    // Нормаль от клинка первого бойца к клинку второго
    let normal = if distance > DISTANCE_EPSILON {
        (b_point - a_point) / distance
    } else {
        // Клинки пересеклись: направление берём от рукояти к рукояти
        (b_hilt - a_hilt).normalize_or_zero()
    };

    // Расстояние от рукояти до точки контакта вдоль клинка
    let a_s = a_t * (a_peak - a_hilt).length();
    let b_s = b_t * (b_peak - b_hilt).length();

    // Скорость сближения точек контакта
    let closing = (a.sword_point_velocity(a_s) - b.sword_point_velocity(b_s)).dot(normal);

    // Оба удара отражены: клинки уходят в исходное положение
    a.reflect_sword();
    b.reflect_sword();

    // Удар отбит: оба бойца знают, что их удар не прошёл
    a.emit_event(BrainEvent::Parried);
    b.emit_event(BrainEvent::Parried);

    if closing <= 0. {
        return;
    }

    // Импульс разводит бойцов
    let impulse = closing * BLADE_MASS;

    a.add_velocity(-normal * impulse * IMPULSE_ON_PARRY_SHARE);
    b.add_velocity(normal * impulse * IMPULSE_ON_PARRY_SHARE);
}

/// Удар клинка бойца по телу противника
fn strike(attacker: &mut Fighter, victim: &mut Fighter) {
    if !attacker.is_sword_attacking() {
        return;
    }

    let mut target: Option<Hit> = None;

    for part in Part::ALL {
        if let Some(hit) = sword_contact(attacker, victim, part) {
            // Урон наносит одна часть тела — та, чья поверхность ближе всего к клинку.
            // При равной глубине побеждает первая: туловище прикрывает голову
            if target.is_none_or(|t| hit.depth > t.depth) {
                target = Some(hit);
            }
        }
    }

    let Some(hit) = target else {
        return;
    };

    victim.deal_damage(hit.power * DAMAGE_PER_IMPULSE * hit.part.damage_factor());

    // Бойцы знают о результате удара: один нанёс, другой получил
    attacker.emit_event(BrainEvent::HitDealt { power: hit.power });
    victim.emit_event(BrainEvent::HitTaken { power: hit.power });

    // Попадание в голову лишает бойца способности управлять телом
    if hit.part == Part::Head {
        victim.get_shock(SHOCK_DURATION);
    }

    // Импульс: жертву сбивает по ходу клинка, атакующий получает отдачу
    victim.add_velocity(-hit.normal * hit.power * IMPULSE_TO_VICTIM_SHARE);
    attacker.add_velocity(hit.normal * hit.power * IMPULSE_TO_ATTACKER_SHARE);

    // Удар сбит: клинок уходит в стойку, урон больше не начисляется
    attacker.reflect_sword();
}

/// Контакт клинка бойца с частью тела противника
fn sword_contact(attacker: &Fighter, victim: &Fighter, part: Part) -> Option<Hit> {
    let (hilt, peak) = attacker.sword_position();
    let blade = peak - hilt;

    let shape = part.shape(victim);
    let contact = surface_contact(&shape, hilt, peak);

    let depth = BLADE_WIDTH * 0.5 - contact.distance;

    if depth <= 0. {
        return None;
    }

    // Расстояние от рукояти до точки контакта вдоль клинка
    let s = contact.blade_t * blade.length();

    let v_blade = attacker.sword_point_velocity(s);
    let v_part = part.velocity_at(victim, contact.local);

    // Нормаль направлена от поверхности части тела к клинку
    let normal = if contact.normal.length_squared() > DISTANCE_EPSILON * DISTANCE_EPSILON {
        contact.normal
    } else {
        // Клинок лежит на оси части тела: направление берём против хода клинка
        let speed = v_blade.length();
        if speed > DISTANCE_EPSILON {
            -v_blade / speed
        } else {
            -blade.normalize_or_zero()
        }
    };

    // Скорость сближения вдоль нормали
    let closing = (v_part - v_blade).dot(normal);

    if closing <= 0. {
        return None;
    }

    Some(Hit {
        part,
        normal,
        power: closing * BLADE_MASS,
        depth,
    })
}

/// Столкновение тел бойцов: раздвижение и обмен частью скорости
fn separate_bodies(a: &mut Fighter, b: &mut Fighter) {
    let delta = b.position() - a.position();
    let distance = delta.length();

    // Нормаль от первого бойца ко второму
    let normal = if distance > DISTANCE_EPSILON {
        delta / distance
    } else {
        Vec2::new(1., 0.)
    };

    // Туловища не должны перекрываться: минимальное расстояние между
    // центрами равно сумме проекций эллипсов туловищ на направление раздвига
    let min_distance = a.torso_support_along(normal) + b.torso_support_along(-normal);

    if distance >= min_distance {
        return;
    }

    // Раздвигаем бойцов на половину перекрытия
    let correction = normal * ((min_distance - distance) * 0.5);
    a.set_position(a.position() - correction);
    b.set_position(b.position() + correction);

    // Скорость сближения вдоль нормали
    let closing = (a.velocity() - b.velocity()).dot(normal);

    if closing <= 0. {
        return;
    }

    // Бойцы обмениваются частью скорости
    let impulse = (1. + BODY_RESTITUTION) * closing * 0.5;

    a.add_velocity(-normal * impulse);
    b.add_velocity(normal * impulse);
}

/// Ближайшая к `p` точка отрезка [a, b] и параметр `t` на отрезке (0..1)
fn closest_point_on_segment(a: Vec2, b: Vec2, p: Vec2) -> (Vec2, f32) {
    let ab = b - a;
    let length_sq = ab.length_squared();

    if length_sq < DISTANCE_EPSILON {
        return (a, 0.);
    }

    let t = ((p - a).dot(ab) / length_sq).clamp(0., 1.);

    (a + ab * t, t)
}

/// Ближайшие точки двух отрезков, их параметры на отрезках и расстояние между ними
fn closest_points_on_segments(
    a1: Vec2,
    a2: Vec2,
    b1: Vec2,
    b2: Vec2,
) -> (Vec2, f32, Vec2, f32, f32) {
    let d1 = a2 - a1;
    let d2 = b2 - b1;
    let r = a1 - b1;

    let a = d1.dot(d1);
    let e = d2.dot(d2);
    let f = d2.dot(r);

    let s;
    let t;

    if a < DISTANCE_EPSILON {
        // Первый отрезок вырожден в точку
        s = 0.;
        t = if e < DISTANCE_EPSILON {
            0.
        } else {
            (f / e).clamp(0., 1.)
        };
    } else {
        let c = d1.dot(r);

        if e < DISTANCE_EPSILON {
            // Второй отрезок вырожден в точку
            t = 0.;
            s = (-c / a).clamp(0., 1.);
        } else {
            let b = d1.dot(d2);
            let denominator = a * e - b * b;

            let mut s_value = if denominator > DISTANCE_EPSILON {
                ((b * f - c * e) / denominator).clamp(0., 1.)
            } else {
                // Отрезки параллельны
                0.
            };

            let mut t_value = (b * s_value + f) / e;

            if t_value < 0. {
                t_value = 0.;
                s_value = (-c / a).clamp(0., 1.);
            } else if t_value > 1. {
                t_value = 1.;
                s_value = ((b - c) / a).clamp(0., 1.);
            }

            s = s_value;
            t = t_value;
        }
    }

    let p1 = a1 + d1 * s;
    let p2 = b1 + d2 * t;

    (p1, s, p2, t, (p1 - p2).length())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        brain::doll::Doll,
        fighter::MAX_MOVE_SPEED,
        math::PI,
    };

    /// Боец в заданной точке, смотрящий в заданном направлении
    fn fighter_at(position: Vec2, facing: f32) -> Fighter {
        let mut f = Fighter::new(Box::new(Doll));
        f.set_position(position);
        f.rotate(facing);

        f
    }

    /// Прогоняет тики физики, разрешая столкновения на каждом
    fn fight(a: &mut Fighter, b: &mut Fighter, ticks: usize) {
        for _ in 0..ticks {
            a.tick();
            b.tick();
            resolve_collisions(a, b);
        }
    }

    /// Прогоняет тики, пока не выполнится условие.
    /// Возвращает, выполнилось ли оно.
    fn fight_until(
        a: &mut Fighter,
        b: &mut Fighter,
        ticks: usize,
        condition: impl Fn(&Fighter, &Fighter) -> bool,
    ) -> bool {
        for _ in 0..ticks {
            a.tick();
            b.tick();
            resolve_collisions(a, b);

            if condition(a, b) {
                return true;
            }
        }

        false
    }

    /// Атакующий: стоит в начале координат и смотрит вверх
    fn attacker() -> Fighter {
        fighter_at(Vec2::ZERO, 0.)
    }

    /// Противник спиной к атакующему: удар приходится в туловище
    fn torso_target() -> Fighter {
        fighter_at(Vec2::new(0.95, 4.0), 0.)
    }

    /// Противник лицом к атакующему: удар приходится в голову
    fn head_target() -> Fighter {
        fighter_at(Vec2::new(0.95, 4.0), PI)
    }

    /// Противник так, что удар приходится в левую руку:
    /// кулак стоит на линии клинка и ниже ближней границы торса
    fn fist_target() -> Fighter {
        fighter_at(Vec2::new(2.0, 3.6), PI / 2.)
    }

    /// Колющий удар в туловище снижает выносливость противника
    #[test]
    fn prick_hits_torso() {
        let mut a = attacker();
        let mut b = torso_target();

        a.prick();

        let hit = fight_until(&mut a, &mut b, 60, |_, b| b.stamina() < 1.);
        assert!(hit, "удар не достиг противника");

        // Туловище не накладывает эффект шока
        assert_eq!(b.shock(), 0.);
        // Удар сбит: клинок ушёл в стойку
        assert!(a.is_sword_retracts());
        assert!(!a.is_sword_attacking());
    }

    /// Рубящий удар тоже наносит урон
    #[test]
    fn chok_hits_torso() {
        let mut a = attacker();
        let mut b = torso_target();

        a.chok();

        let hit = fight_until(&mut a, &mut b, 60, |_, b| b.stamina() < 1.);
        assert!(hit, "удар не достиг противника");
    }

    /// Попадание в голову накладывает эффект шока
    #[test]
    fn hit_in_head_applies_shock() {
        let mut a = attacker();
        let mut b = head_target();

        a.prick();

        // Шок живёт недолго, поэтому проверяем его сразу после попадания
        let hit = fight_until(&mut a, &mut b, 60, |_, b| b.shock() > 0.);
        assert!(hit, "удар не попал в голову");

        assert!(b.stamina() < 1.);
    }

    /// Урон по левой руке в несколько раз меньше, чем по туловищу
    #[test]
    fn fist_damage_is_smaller_than_torso() {
        let mut a = attacker();
        let mut b = fist_target();

        a.prick();

        let hit = fight_until(&mut a, &mut b, 60, |_, b| b.stamina() < 1.);
        assert!(hit, "удар не достиг руки");

        let fist_damage = 1. - b.stamina();

        let mut a = attacker();
        let mut b = torso_target();

        a.prick();
        fight_until(&mut a, &mut b, 60, |_, b| b.stamina() < 1.);

        let torso_damage = 1. - b.stamina();

        assert!(fist_damage < torso_damage * 0.5);
    }

    /// Чем быстрее летит клинок, тем больше урона
    #[test]
    fn faster_strike_does_more_damage() {
        let mut slow = attacker();
        let mut target = torso_target();

        slow.prick();
        fight_until(&mut slow, &mut target, 60, |_, b| b.stamina() < 1.);

        let mut fast = attacker();
        let mut target = torso_target();

        // Атакующий разгоняется навстречу противнику
        fast.set_velocity(Vec2::new(0., 2.));
        fast.prick();
        fight_until(&mut fast, &mut target, 60, |_, b| b.stamina() < 1.);

        assert!(1. - target.stamina() > 1. - slow.stamina());
    }

    /// Сила удара даёт разумный урон: ничтожный, но и не нокаут с одного раза
    #[test]
    fn damage_is_in_reasonable_range() {
        let mut a = attacker();
        let mut b = torso_target();

        a.prick();
        fight_until(&mut a, &mut b, 60, |_, b| b.stamina() < 1.);

        let damage = 1. - b.stamina();

        assert!(damage > 0.02, "удар слишком слабый: {}", damage);
        assert!(damage < 0.3, "удар слишком сильный: {}", damage);
    }

    /// Длительный бой не ломает физику: значения остаются конечными
    #[test]
    fn long_fight_is_stable() {
        let mut a = attacker();
        let mut b = fighter_at(Vec2::new(0.95, 3.0), 0.);

        a.prick();
        b.chok();

        for _ in 0..600 {
            a.tick();
            b.tick();
            resolve_collisions(&mut a, &mut b);

            assert!(a.position().is_finite());
            assert!(b.position().is_finite());
            assert!(a.velocity().is_finite());
            assert!(b.velocity().is_finite());
            assert!(a.stamina().is_finite());
            assert!(b.stamina().is_finite());
        }
    }

    /// Урон наносится один раз за удар
    #[test]
    fn strike_damages_only_once() {
        let mut a = attacker();
        let mut b = torso_target();

        a.prick();
        fight(&mut a, &mut b, 60);

        let damage = 1. - b.stamina();
        assert!(damage > 0.);

        // Клинок больше не входит в тело, но урон и не должен расти
        fight(&mut a, &mut b, 60);

        assert!((1. - b.stamina() - damage).abs() < 1e-5);
    }

    /// Два атакующих клинка отражают друг друга
    #[test]
    fn crossing_swords_are_reflected() {
        let mut a = attacker();
        let mut b = fighter_at(Vec2::new(1.9, 0.5), PI / 2.);

        a.prick();
        b.prick();

        let mut reflected = false;

        for _ in 0..60 {
            a.tick();
            b.tick();

            let both_attacking = a.is_sword_attacking() && b.is_sword_attacking();

            resolve_collisions(&mut a, &mut b);

            // Оба удара сбились одновременно, хотя до столкновения оба были в ударе
            if both_attacking && !a.is_sword_attacking() && !b.is_sword_attacking() {
                reflected = true;
                break;
            }
        }

        assert!(reflected, "удары не были отражены");
        // Отражённые удары не наносят урона
        assert_eq!(a.stamina(), 1.);
        assert_eq!(b.stamina(), 1.);
    }

    /// Меч оппонента в стойке столкновений не даёт
    #[test]
    fn idle_opponent_sword_is_not_reflected() {
        let mut a = attacker();
        let mut b = fighter_at(Vec2::new(1.9, 0.5), PI / 2.);

        a.prick();

        // Клинки пересекаются в первые же тики
        fight(&mut a, &mut b, 3);

        // Меч оппонента остался в стойке: отражения не было
        assert!(!b.is_sword_retracts());
        assert!(!b.is_sword_attacking());
        // Атакующий продолжает удар
        assert!(a.is_sword_attacking());
    }

    /// Боец без удара не наносит урона
    #[test]
    fn idle_sword_does_no_damage() {
        let mut a = attacker();
        let mut b = torso_target();

        fight(&mut a, &mut b, 60);

        assert_eq!(b.stamina(), 1.);
    }

    /// Столкнувшиеся тела раздвигаются
    #[test]
    fn bodies_are_pushed_apart() {
        let mut a = fighter_at(Vec2::new(-0.9, 0.), 0.);
        let mut b = fighter_at(Vec2::new(0.9, 0.), 0.);

        resolve_collisions(&mut a, &mut b);

        let distance = (b.position() - a.position()).length();
        assert!(distance >= TORSO_WIDTH * 2. - 1e-4);
    }

    /// При столкновении тел бойцы обмениваются частью скорости
    #[test]
    fn body_collision_exchanges_velocity() {
        let mut a = fighter_at(Vec2::new(-0.9, 0.), 0.);
        let mut b = fighter_at(Vec2::new(0.9, 0.), 0.);

        a.set_velocity(Vec2::new(2., 0.));
        b.set_velocity(Vec2::new(-2., 0.));

        resolve_collisions(&mut a, &mut b);

        // После удара бойцы расходятся
        assert!(a.velocity().x < 0.);
        assert!(b.velocity().x > 0.);
        // Относительная скорость уменьшилась, но не исчезла: тела слегка отскакивают
        let relative = (b.velocity() - a.velocity()).x;
        assert!(relative > 0.);
        assert!(relative < 4.);
    }

    /// Столкновение тел не меняет касательную составляющую скорости
    #[test]
    fn body_collision_keeps_tangential_velocity() {
        let mut a = fighter_at(Vec2::new(-0.9, 0.), 0.);
        let mut b = fighter_at(Vec2::new(0.9, 0.), 0.);

        a.set_velocity(Vec2::new(2., 1.));
        b.set_velocity(Vec2::new(0., -1.));

        resolve_collisions(&mut a, &mut b);

        assert!((a.velocity().y - 1.).abs() < 1e-5);
        assert!((b.velocity().y + 1.).abs() < 1e-5);
    }

    /// Далёкие бойцы друг друга не замечают
    #[test]
    fn body_collision_does_nothing_when_far() {
        let mut a = fighter_at(Vec2::new(-2., 0.), 0.);
        let mut b = fighter_at(Vec2::new(2., 0.), 0.);

        a.set_velocity(Vec2::new(2., 0.));
        b.set_velocity(Vec2::new(-2., 0.));

        resolve_collisions(&mut a, &mut b);

        assert!((a.velocity() - Vec2::new(2., 0.)).length() < 1e-5);
        assert!((b.velocity() - Vec2::new(-2., 0.)).length() < 1e-5);
        assert!((a.position() - Vec2::new(-2., 0.)).length() < 1e-5);
    }

    /// Импульс сбивает поражённого по ходу клинка
    #[test]
    fn victim_is_pushed_along_the_blade() {
        let mut a = attacker();
        let mut b = torso_target();

        a.prick();
        fight(&mut a, &mut b, 60);

        // Клинок шёл вверх, значит и бойца сбило вверх
        assert!(b.velocity().y > 0.);
    }

    /// Атакующий получает отдачу, но меньшую, чем толчок жертвы
    #[test]
    fn attacker_gets_recoil() {
        let mut a = attacker();
        let mut b = torso_target();

        a.prick();
        fight(&mut a, &mut b, 60);

        // Атакующего отбросило назад
        assert!(a.velocity().y < 0.);
        assert!(a.velocity().y.abs() < b.velocity().y.abs());
    }

    /// Скорость бойца не превышает предел после импульса
    #[test]
    fn impulse_respects_max_speed() {
        let mut a = attacker();
        let mut b = torso_target();

        b.set_velocity(Vec2::new(0., MAX_MOVE_SPEED));
        a.prick();
        fight(&mut a, &mut b, 60);

        assert!(a.velocity().length() <= MAX_MOVE_SPEED + 1e-5);
        assert!(b.velocity().length() <= MAX_MOVE_SPEED + 1e-5);
    }

    /// Урон соответствует импульсу удара
    #[test]
    fn damage_matches_analytic_value() {
        let mut a = attacker();
        let mut b = torso_target();

        a.prick();

        let mut checked = false;

        for _ in 0..60 {
            a.tick();
            b.tick();

            if !checked {
                if let Some(hit) = sword_contact(&a, &b, Part::Torso) {
                    let expected = hit.power * DAMAGE_PER_IMPULSE * Part::Torso.damage_factor();

                    resolve_collisions(&mut a, &mut b);

                    assert!((1. - b.stamina() - expected).abs() < 1e-5);
                    checked = true;

                    continue;
                }
            }

            resolve_collisions(&mut a, &mut b);
        }

        assert!(checked, "удар не достиг противника");
    }

    /// Удар в бок туловища сносит противника в сторону от клинка
    #[test]
    fn side_hit_pushes_victim_sideways() {
        let mut a = attacker();
        // Противник стоит сбоку: клинок накрывает левый край его торса
        let mut b = fighter_at(Vec2::new(2.75, 3.5), 0.);

        a.prick();
        fight(&mut a, &mut b, 60);

        // Удар пришёлся в левый бок: противника сносит вправо
        assert!(b.velocity().x > 0.);
    }

    /// Нормаль контакта — внешняя нормаль поверхности, а не направление от центра
    #[test]
    fn contact_normal_follows_surface() {
        let victim = fighter_at(Vec2::new(2.75, 3.5), 0.);

        // Точка поверхности эллипса в левой нижней части
        let surface = Vec2::new(1.4, 3.195);
        // Направление от центра тела к этой точке заметно отличается от нормали
        let from_center = (surface - victim.position()).normalize_or_zero();
        let normal = Vec2::new(-0.694, -0.720).normalize_or_zero();

        // Клинок подходит к поверхности по нормали
        let contact = surface_contact(
            &Part::Torso.shape(&victim),
            surface - normal * 0.3,
            surface,
        );

        assert!((contact.normal - normal).length() < 0.15);
        assert!((contact.normal - from_center).length() > 0.3);
    }
}
