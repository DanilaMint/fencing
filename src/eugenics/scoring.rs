//! Очки бойца и исходы боя.
//!
//! Очки — это fitness генетического алгоритма: чем больше очков набрал
//! боец, тем удачнее его геном. Расценка одна на все бои.
//!
//! Есть два варианта расценки:
//!
//! | Исход | `classic`: победителю / проигравшему | `epic`: победителю / проигравшему |
//! |---|---|---|
//! | Ничья | −1 / −1 | −3 / −3 |
//! | Выход за ринг | +1 / −1 | +1 / −1 |
//! | Выдохся от энергии | +2 / −2 | +1 / −1 |
//! | Выдохся от удара | +4 / −4 | +8 / −8 |
//!
//! Разница в цене исходов. В `classic` нокаут от энергии (бойцы просто
//! изматывают друг друга) стоит вдвое дешевле нокаута от удара, поэтому
//! отбор охотно выбирает измор: он надёжнее, чем попасть ударом.
//!
//! В `epic` нокаут от удара стоит в восемь раз дороже измора, ничья штрафуется
//! вдвое сильнее, а каждый нанесённый удар даёт ещё одно очко. Отбор тогда
//! ведёт к бою: подойти, ударить, добить.

use crate::round::{LoseReason, RoundResult};

/// Начальное число очков бойца в поколении
pub const BASE_SCORE: i32 = 1000;

/// Вариант расценки очков
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Scoring {
    /// Исходы стоят как в исходных правилах: измор дешевле, чем удар
    Classic,
    /// Добивание дороже всего, измор и ничья почти ничего не стоят
    Epic,
}

impl Scoring {
    /// Очки за ничью: столько теряет каждый
    pub fn draw(self) -> i32 {
        match self {
            Scoring::Classic => -1,
            Scoring::Epic => -3,
        }
    }

    /// Очки победителя и проигравшего по причине поражения
    pub fn by_reason(self, reason: LoseReason) -> (i32, i32) {
        match (self, reason) {
            (Scoring::Classic, LoseReason::OutOfRing) => (1, -1),
            (Scoring::Classic, LoseReason::Exhausted) => (2, -2),
            (Scoring::Classic, LoseReason::Defeated) => (4, -4),
            (Scoring::Epic, LoseReason::OutOfRing) => (1, -1),
            // Измор почти не ценится: боец, доводящий противника до усталости
            // сам себе яму вырыл
            (Scoring::Epic, LoseReason::Exhausted) => (1, -1),
            // Добивание — самый ценный исход
            (Scoring::Epic, LoseReason::Defeated) => (8, -8),
        }
    }

    /// Очки за каждый нанесённый удар
    ///
    /// Получатель удара теряет столько же: сумма очков в бою не меняется.
    pub fn hit_bonus(self) -> i32 {
        match self {
            Scoring::Classic => 0,
            Scoring::Epic => 1,
        }
    }
}

impl std::fmt::Display for Scoring {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Scoring::Classic => write!(f, "classic"),
            Scoring::Epic => write!(f, "epic"),
        }
    }
}

impl std::str::FromStr for Scoring {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "classic" => Ok(Scoring::Classic),
            "epic" => Ok(Scoring::Epic),
            other => Err(format!("неизвестная расценка: \"{other}\"")),
        }
    }
}

/// Очки обоих бойцов по результату боя
///
/// Возвращает пару `(очки красного, очки синего)`.
pub fn points_for(
    scoring: Scoring,
    result: RoundResult,
    red_lose: Option<LoseReason>,
    blue_lose: Option<LoseReason>,
) -> (i32, i32) {
    match result {
        // Ничья: оба теряют очки
        RoundResult::Draw => (scoring.draw(), scoring.draw()),
        // Победа красного: проигравший — синий
        RoundResult::RedWin => {
            let (winner, loser) = points_by_reason(scoring, blue_lose);
            (winner, loser)
        }
        // Победа синего: проигравший — красный
        RoundResult::BlueWin => {
            let (winner, loser) = points_by_reason(scoring, red_lose);
            (loser, winner)
        }
    }
}

/// Очки победителя и проигравшего по причине поражения
fn points_by_reason(scoring: Scoring, reason: Option<LoseReason>) -> (i32, i32) {
    match reason {
        Some(reason) => scoring.by_reason(reason),
        // Причины поражения у победителя быть не может: бой без поражения
        // проигравшего — это ничья, но она разбирается раньше
        None => (scoring.draw(), scoring.draw()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Ничья стоит очко обоим
    #[test]
    fn draw_costs_one_point_to_both() {
        let (red, blue) = points_for(Scoring::Classic, RoundResult::Draw, None, None);

        assert_eq!(red, -1);
        assert_eq!(blue, -1);
    }

    /// В классической расценке ничья штрафуется сильнее
    #[test]
    fn epic_draw_costs_more() {
        let (red, blue) = points_for(Scoring::Epic, RoundResult::Draw, None, None);

        assert_eq!(red, -3);
        assert_eq!(blue, -3);
    }

    /// Выход за ринг: победителю одно очко, проигравшему минус одно
    #[test]
    fn out_of_ring_gives_one_point_to_winner() {
        let (red, blue) = points_for(
            Scoring::Classic,
            RoundResult::RedWin,
            None,
            Some(LoseReason::OutOfRing),
        );

        assert_eq!(red, 1);
        assert_eq!(blue, -1);
    }

    /// Нокаут от энергии: победителю два очка, проигравшему минус два
    #[test]
    fn exhaustion_gives_two_points_to_winner() {
        let (red, blue) = points_for(
            Scoring::Classic,
            RoundResult::RedWin,
            None,
            Some(LoseReason::Exhausted),
        );

        assert_eq!(red, 2);
        assert_eq!(blue, -2);
    }

    /// Нокаут от удара: победителю четыре очка, проигравшему минус четыре
    #[test]
    fn defeat_gives_four_points_to_winner() {
        let (red, blue) = points_for(
            Scoring::Classic,
            RoundResult::RedWin,
            None,
            Some(LoseReason::Defeated),
        );

        assert_eq!(red, 4);
        assert_eq!(blue, -4);
    }

    /// В эпической расценке добивание стоит в восемь раз дороже измора
    #[test]
    fn epic_prices_finishing_much_higher() {
        let (winner, _) = points_for(
            Scoring::Epic,
            RoundResult::RedWin,
            None,
            Some(LoseReason::Defeated),
        );
        let (grind, _) = points_for(
            Scoring::Epic,
            RoundResult::RedWin,
            None,
            Some(LoseReason::Exhausted),
        );

        assert_eq!(winner, 8);
        assert_eq!(grind, 1);
    }

    /// Победа синего меняет местами очки
    #[test]
    fn blue_win_swaps_points() {
        let (red, blue) = points_for(
            Scoring::Classic,
            RoundResult::BlueWin,
            Some(LoseReason::Defeated),
            None,
        );

        assert_eq!(red, -4);
        assert_eq!(blue, 4);
    }

    /// В решительном бою очки победителя и проигравшего в сумме дают ноль
    #[test]
    fn decisive_fights_sum_to_zero() {
        for scoring in [Scoring::Classic, Scoring::Epic] {
            for reason in [
                LoseReason::OutOfRing,
                LoseReason::Exhausted,
                LoseReason::Defeated,
            ] {
                let (red, blue) = points_for(scoring, RoundResult::RedWin, None, Some(reason));

                assert_eq!(red + blue, 0, "причина {reason:?}");
            }
        }
    }

    /// Начальный счёт бойца — тысяча
    #[test]
    fn base_score_is_one_thousand() {
        assert_eq!(BASE_SCORE, 1000);
    }
}
