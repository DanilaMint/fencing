//! Таблица «стимул -> реакция».
//!
//! В таблице N позиций, изначально все пустые. Боец ищет запись, стимул
//! которой похож на текущую обстановку; если похожей записи нет, самая
//! слабая позиция занимается новой случайной реакцией.

use std::{
    io,
    path::Path,
};

use fastrand::Rng;
use serde::{Deserialize, Serialize};

use crate::fighter::brain::strategist::{
    reaction::{Reaction, random_reaction},
    stimulus::Stimulus,
};

/// Число записей в таблице реакций
pub const TABLE_SIZE: usize = 16;

/// Порог похожести: запись подходит, если похожесть меньше порога
pub const MATCH_THRESHOLD: f32 = 0.12;
/// Разброс порога: к нему добавляется случайный шум, иначе боец
/// зацикливается на одной и той же записи
pub const THRESHOLD_NOISE: f32 = 0.06;
/// Авансовый рейтинг новой записи: чтобы её не стёрли по первому же исходу
pub const INITIAL_RATING: f32 = 1.0;
/// Потолок рейтинга записи
pub const RATING_MAX: f32 = 10.0;

/// Запись таблицы: стимул, реакция и рейтинг
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Entry {
    /// Стимул, при котором запись была создана
    pub stimulus: Stimulus,
    /// Реакция, которую боец выполняет
    pub reaction: Reaction,
    /// Рейтинг: средняя выгода одного выполнения реакции.
    /// Чем больше, тем удачнее запись
    pub rating: f32,
    /// Сколько выполнений учтено в рейтинге
    pub uses: u32,
}

/// Таблица реакций: N позиций, изначально все пустые
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Table {
    entries: Vec<Option<Entry>>,
}

impl Table {
    /// Новая пустая таблица.
    ///
    /// Размер приводится к минимуму в одну позицию: иначе искать нечего.
    pub fn new(size: usize) -> Self {
        Self {
            entries: vec![None; size.max(1)],
        }
    }

    /// Число позиций в таблице
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Число заполненных позиций
    pub fn filled(&self) -> usize {
        self.entries.iter().filter(|entry| entry.is_some()).count()
    }

    /// Запись по индексу
    pub fn entry(&self, index: usize) -> Option<&Entry> {
        self.entries.get(index).and_then(|entry| entry.as_ref())
    }

    /// Кладёт запись в указанную позицию.
    /// Если позиция занята, она заменяется.
    pub fn set_entry(&mut self, index: usize, entry: Entry) {
        if index < self.entries.len() {
            self.entries[index] = Some(entry);
        }
    }

    /// Рейтинг записи. У пустой позиции рейтинга нет вовсе
    pub fn rating(&self, index: usize) -> Option<f32> {
        self.entry(index).map(|entry| entry.rating)
    }

    /// Выбирает запись, подходящую под стимул.
    ///
    /// Если подходящих записей нет, самая слабая позиция (пустые позиции
    /// считаются самыми слабыми, поэтому заполняются первыми) занимается
    /// новой случайной реакцией. Возвращает индекс выбранной записи.
    pub fn select(&mut self, stimulus: &Stimulus, rng: &mut Rng) -> usize {
        // Порог с небольшим случайным шумом
        let threshold = MATCH_THRESHOLD + (rng.f32() - 0.5) * THRESHOLD_NOISE;

        let mut best: Option<(usize, f32)> = None;

        for (index, entry) in self.entries.iter().enumerate() {
            let Some(entry) = entry else { continue };

            let similarity = stimulus.similarity(&entry.stimulus);
            if similarity >= threshold {
                continue;
            }

            let better = match best {
                None => true,
                // Из одинаково похожих берём ту, что удачнее
                Some((best_index, best_similarity)) => {
                    similarity < best_similarity
                        || (similarity == best_similarity
                            && entry.rating > self.rating(best_index).unwrap_or(f32::NEG_INFINITY))
                }
            };

            if better {
                best = Some((index, similarity));
            }
        }

        if let Some((index, _)) = best {
            return index;
        }

        // Похожих реакций нет: занимаем самую слабую позицию
        let index = self.lowest_rated_index();

        self.entries[index] = Some(Entry {
            stimulus: *stimulus,
            reaction: random_reaction(rng),
            // Авансовый рейтинг: новая запись не должна умереть
            // от первого же неудачного исхода
            rating: INITIAL_RATING,
            // Авансовый рейтинг считается первым наблюдением
            uses: 1,
        });

        index
    }

    /// Учитывает результат выполнения реакции в рейтинге записи.
    ///
    /// Рейтинг — среднее по выполнениям: так записи сравнимы между собой.
    /// Если просто копить очки, то запись, которую применяют чаще, набирает
    /// их только за счёт числа повторов.
    ///
    /// Если рейтинг опустился ниже нуля, запись стирается.
    /// Возвращает true, если запись была стёрта.
    pub fn apply_rating(&mut self, index: usize, delta: f32) -> bool {
        let Some(entry) = self.entries.get_mut(index).and_then(|entry| entry.as_mut()) else {
            return false;
        };

        let uses = entry.uses as f32;
        entry.rating = ((entry.rating * uses + delta) / (uses + 1.)).min(RATING_MAX);
        entry.uses += 1;

        if entry.rating < 0. {
            self.entries[index] = None;
            return true;
        }

        false
    }

    /// Сохраняет таблицу в JSON-файл
    pub fn save_to_file(&self, path: impl AsRef<Path>) -> io::Result<()> {
        let text = serde_json::to_string_pretty(self).map_err(data_error)?;

        std::fs::write(path, text)
    }

    /// Загружает таблицу из JSON-файла
    pub fn load_from_file(path: impl AsRef<Path>) -> io::Result<Self> {
        let text = std::fs::read_to_string(path)?;

        serde_json::from_str(&text).map_err(data_error)
    }

    /// Индекс позиции с самым низким рейтингом.
    /// Пустые позиции считаются самыми слабыми.
    fn lowest_rated_index(&self) -> usize {
        let mut index = 0;
        let mut best = f32::INFINITY;

        for (i, entry) in self.entries.iter().enumerate() {
            let rating = entry.as_ref().map_or(f32::NEG_INFINITY, |entry| entry.rating);

            if rating < best {
                best = rating;
                index = i;
            }
        }

        index
    }
}

/// Ошибка разбора или записи JSON переводится в ошибку ввода-вывода
fn data_error(err: serde_json::Error) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, err)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Стимул для тестов
    fn stimulus(enemy_distance: f32) -> Stimulus {
        Stimulus {
            center_distance: 0.2,
            enemy_distance,
            ..Default::default()
        }
    }

    /// Совсем далёкая обстановка: похожей записи на неё нет
    fn far_stimulus() -> Stimulus {
        Stimulus {
            center_distance: 1.,
            enemy_distance: 1.,
            to_enemy_angle: 1.,
            from_enemy_angle: 1.,
            facing_delta: 1.,
        }
    }

    /// Пустая таблица: выбор создаёт запись с авансовым рейтингом
    #[test]
    fn empty_table_selects_and_fills_a_slot() {
        let mut table = Table::new(4);
        let mut rng = Rng::with_seed(1);

        let index = table.select(&stimulus(0.5), &mut rng);

        assert_eq!(table.filled(), 1);
        assert_eq!(table.rating(index), Some(INITIAL_RATING));
    }

    /// Сначала заполняются пустые позиции, перезапись начинается только
    /// после того, как заняты все
    #[test]
    fn empty_slots_are_filled_before_overwriting() {
        let mut table = Table::new(3);
        let mut rng = Rng::with_seed(2);

        // Разные обстановки: каждая даёт свою запись
        for distance in [0., 0.5, 1.] {
            table.select(&stimulus(distance), &mut rng);
        }

        assert_eq!(table.filled(), 3);

        // Теперь позиции заняты: следующая выборка перезапишет одну из них,
        // а не займёт новую
        let before = table.clone();
        table.select(&far_stimulus(), &mut rng);

        assert_eq!(table.filled(), 3);
        assert_ne!(table, before);
    }

    /// Одинаковая ситуация ведёт в ту же запись
    #[test]
    fn same_stimulus_reuses_the_same_entry() {
        let mut table = Table::new(4);
        let mut rng = Rng::with_seed(3);

        let first = table.select(&stimulus(0.5), &mut rng);
        let second = table.select(&stimulus(0.5), &mut rng);

        assert_eq!(first, second);
        assert_eq!(table.filled(), 1);
    }

    /// Сильно отличающаяся ситуация создаёт новую запись
    #[test]
    fn far_situation_creates_new_entry() {
        let mut table = Table::new(4);
        let mut rng = Rng::with_seed(4);

        table.select(&Stimulus::default(), &mut rng);
        table.select(&stimulus(1.), &mut rng);

        assert_eq!(table.filled(), 2);
    }

    /// Перезаписывается запись с минимальным рейтингом
    #[test]
    fn lowest_rated_entry_is_overwritten_first() {
        let mut table = Table::new(2);
        let mut rng = Rng::with_seed(5);

        let first = table.select(&Stimulus::default(), &mut rng);
        let second = table.select(&stimulus(1.), &mut rng);
        assert_ne!(first, second);

        // Первая запись становится хуже второй, но не стирается
        table.apply_rating(first, -0.9);
        table.apply_rating(second, 0.5);
        // Рейтинг — среднее: авансовый рейтинг и результат делятся пополам
        assert_eq!(table.rating(first), Some((INITIAL_RATING - 0.9) / 2.));

        // Совсем далёкий стимул не находит похожей записи
        table.select(&far_stimulus(), &mut rng);

        // Перезаписана самая слабая запись, вторая не тронута
        assert_eq!(table.rating(first), Some(INITIAL_RATING));
        assert_eq!(table.rating(second), Some((INITIAL_RATING + 0.5) / 2.));
    }

    /// Рейтинг ниже нуля стирает запись
    #[test]
    fn rating_below_zero_erases_entry() {
        let mut table = Table::new(2);
        let mut rng = Rng::with_seed(6);

        let index = table.select(&stimulus(0.5), &mut rng);

        let erased = table.apply_rating(index, -INITIAL_RATING - 1.);

        assert!(erased);
        assert!(table.entry(index).is_none());
        assert_eq!(table.filled(), 0);
    }

    /// Рейтинг не растёт выше потолка
    #[test]
    fn rating_is_capped_at_max() {
        let mut table = Table::new(2);
        let mut rng = Rng::with_seed(7);

        let index = table.select(&stimulus(0.5), &mut rng);
        table.apply_rating(index, 100.);

        assert_eq!(table.rating(index), Some(RATING_MAX));
    }

    /// Таблица нулевого размера остаётся рабочей
    #[test]
    fn table_of_zero_size_stays_usable() {
        let mut table = Table::new(0);
        let mut rng = Rng::with_seed(8);

        let index = table.select(&stimulus(0.5), &mut rng);

        assert_eq!(table.len(), 1);
        assert_eq!(table.filled(), 1);
        assert_eq!(index, 0);
    }

    /// Таблица сохраняется в файл и читается обратно без потерь
    #[test]
    fn table_survives_save_and_load() {
        let mut table = Table::new(3);
        let mut rng = Rng::with_seed(9);

        let index = table.select(&stimulus(0.5), &mut rng);
        table.apply_rating(index, 1.5);

        let path = std::env::temp_dir().join("fencing_table_test.json");
        table.save_to_file(&path).unwrap();

        let loaded = Table::load_from_file(&path).unwrap();

        assert_eq!(loaded, table);
        assert_eq!(loaded.rating(index), Some((INITIAL_RATING + 1.5) / 2.));

        let _ = std::fs::remove_file(&path);
    }

    /// Пустая таблица тоже сохраняется и читается
    #[test]
    fn empty_table_survives_save_and_load() {
        let table = Table::new(4);

        let path = std::env::temp_dir().join("fencing_empty_table_test.json");
        table.save_to_file(&path).unwrap();

        let loaded = Table::load_from_file(&path).unwrap();

        assert_eq!(loaded, table);
        assert_eq!(loaded.filled(), 0);

        let _ = std::fs::remove_file(&path);
    }
}
