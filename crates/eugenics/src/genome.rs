//! Геном бойца: сеть перцептрона и операции над ней.
//!
//! Геном — это веса сети: восемь входов, скрытый слой и три группы выходов.
//! Скрещивание и мутация работают с генами по отдельности: каждый вес сети
//! независим от остальных, поэтому и наследник получается из кусков двух
//! родителей.

use fastrand::Rng;

use fencing_perceptron::{
    features::FEATURE_COUNT,
    net::{Net, OUTPUT_COUNT},
};

/// Пределы весов слоёв сети
///
/// Пределы — по Ксавьеру, те же, что и при случайной инициализации:
/// мутированная сеть живёт в том же диапазоне, что и случайная.
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    /// Предел весов и смещений скрытого слоя
    pub hidden: f32,
    /// Предел весов и смещений выходного слоя
    pub output: f32,
}

/// Пределы весов для сети заданной ширины
pub fn limits(hidden: usize) -> Limits {
    Limits {
        hidden: (6. / (FEATURE_COUNT + hidden) as f32).sqrt(),
        output: (6. / (hidden + OUTPUT_COUNT) as f32).sqrt(),
    }
}

/// Наследник двух родителей
///
/// Каждый ген берётся у матери с вероятностью `rate` и у отца в остальных
/// случаях. Однородное скрещивание перебирает огромное число комбинаций
/// и не связывает несвязанные гены, как это делает одноточечное.
pub fn crossover(mother: &Net, father: &Net, rate: f32, rng: &mut Rng) -> Result<Net, String> {
    if mother.hidden != father.hidden {
        return Err(format!(
            "ширины скрытого слоя родителей не совпадают: {} и {}",
            mother.hidden, father.hidden
        ));
    }

    let mut child = Net::new(mother.hidden);

    for (gene, (mom, dad)) in child
        .w1
        .iter_mut()
        .zip(mother.w1.iter().zip(father.w1.iter()))
    {
        *gene = pick(*mom, *dad, rate, rng);
    }

    for (gene, (mom, dad)) in child
        .b1
        .iter_mut()
        .zip(mother.b1.iter().zip(father.b1.iter()))
    {
        *gene = pick(*mom, *dad, rate, rng);
    }

    for (gene, (mom, dad)) in child
        .w2
        .iter_mut()
        .zip(mother.w2.iter().zip(father.w2.iter()))
    {
        *gene = pick(*mom, *dad, rate, rng);
    }

    for (gene, (mom, dad)) in child
        .b2
        .iter_mut()
        .zip(mother.b2.iter().zip(father.b2.iter()))
    {
        *gene = pick(*mom, *dad, rate, rng);
    }

    Ok(child)
}

/// Мутация генов
///
/// Каждый ген с вероятностью `rate` получает гауссов шум. Величина шума —
/// `scale` от предела слоя, а после мутации ген зажимается в пределы: иначе
/// активации сети «залипают» на краях и боец перестаёт реагировать.
pub fn mutate(net: &mut Net, rate: f32, scale: f32, rng: &mut Rng) {
    let limits = limits(net.hidden);

    mutate_genes(&mut net.w1, rate, scale * limits.hidden, limits.hidden, rng);
    mutate_genes(&mut net.b1, rate, scale * limits.hidden, limits.hidden, rng);
    mutate_genes(&mut net.w2, rate, scale * limits.output, limits.output, rng);
    mutate_genes(&mut net.b2, rate, scale * limits.output, limits.output, rng);
}

/// Доля генов, различающихся у двух сетей
///
/// Мера разнообразия: у выродившейся элиты сети почти одинаковые, и доля
/// различающихся генов падает.
pub fn difference(first: &Net, second: &Net) -> f32 {
    if first.hidden != second.hidden {
        return 1.;
    }

    let mut different = 0usize;
    let mut total = 0usize;

    for (a, b) in first.w1.iter().zip(second.w1.iter()) {
        different += (a != b) as usize;
        total += 1;
    }

    for (a, b) in first.b1.iter().zip(second.b1.iter()) {
        different += (a != b) as usize;
        total += 1;
    }

    for (a, b) in first.w2.iter().zip(second.w2.iter()) {
        different += (a != b) as usize;
        total += 1;
    }

    for (a, b) in first.b2.iter().zip(second.b2.iter()) {
        different += (a != b) as usize;
        total += 1;
    }

    if total == 0 {
        return 0.;
    }

    different as f32 / total as f32
}

/// Выбор одного из двух генов
fn pick(mother: f32, father: f32, rate: f32, rng: &mut Rng) -> f32 {
    if rng.f32() < rate {
        mother
    } else {
        father
    }
}

/// Мутация набора генов слоя
fn mutate_genes(genes: &mut [f32], rate: f32, sigma: f32, limit: f32, rng: &mut Rng) {
    for gene in genes.iter_mut() {
        if rng.f32() < rate {
            *gene = (*gene + gaussian(rng) * sigma).clamp(-limit, limit);
        }
    }
}

/// Гауссово число со средним нулём и единичным разбросом
///
/// Бокс–Мюллер: два равномерных числа дают одно нормальное.
fn gaussian(rng: &mut Rng) -> f32 {
    // Ноль под корнем недопустим: чуть сдвигаем, чтобы не было NaN
    let uniform = rng.f32().max(1e-7);
    let angle = rng.f32() * std::f32::consts::TAU;

    (-2. * uniform.ln()).sqrt() * angle.cos()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn net(hidden: usize, seed: u64) -> Net {
        Net::random(hidden, seed)
    }

    /// Кроссовер всегда берёт гены у родителей
    #[test]
    fn crossover_genes_come_from_parents() {
        let mother = net(4, 1);
        let father = net(4, 2);
        let mut rng = Rng::with_seed(7);

        let child = crossover(&mother, &father, 0.5, &mut rng).unwrap();

        for (gene, (mom, dad)) in child
            .w1
            .iter()
            .zip(mother.w1.iter().zip(father.w1.iter()))
        {
            assert!(gene == mom || gene == dad, "ген взят не у родителей");
        }
    }

    /// Кроссовер со стопроцентной долей матери даёт копию матери
    #[test]
    fn crossover_with_full_rate_gives_mother() {
        let mother = net(4, 1);
        let father = net(4, 2);
        let mut rng = Rng::with_seed(7);

        let child = crossover(&mother, &father, 1., &mut rng).unwrap();

        assert_eq!(child.w1, mother.w1);
        assert_eq!(child.b1, mother.b1);
        assert_eq!(child.w2, mother.w2);
        assert_eq!(child.b2, mother.b2);
    }

    /// Кроссовер с нулевой долей матери даёт копию отца
    #[test]
    fn crossover_with_zero_rate_gives_father() {
        let mother = net(4, 1);
        let father = net(4, 2);
        let mut rng = Rng::with_seed(7);

        let child = crossover(&mother, &father, 0., &mut rng).unwrap();

        assert_eq!(child.w1, father.w1);
        assert_eq!(child.w1.len(), father.w1.len());
    }

    /// Одинаковое зерно даёт одинакового наследника
    #[test]
    fn crossover_is_repeatable() {
        let mother = net(4, 1);
        let father = net(4, 2);

        let mut first = Rng::with_seed(99);
        let mut second = Rng::with_seed(99);

        let a = crossover(&mother, &father, 0.5, &mut first).unwrap();
        let b = crossover(&mother, &father, 0.5, &mut second).unwrap();

        assert_eq!(a, b);
    }

    /// Кроссовер родителей разной ширины — ошибка
    #[test]
    fn crossover_rejects_different_widths() {
        let mother = net(4, 1);
        let father = net(8, 2);
        let mut rng = Rng::with_seed(7);

        let err = crossover(&mother, &father, 0.5, &mut rng).unwrap_err();

        assert!(err.contains("не совпадают"), "{err}");
    }

    /// Мутация меняет веса
    #[test]
    fn mutation_changes_genes() {
        let mut target = net(4, 1);
        let before = target.clone();
        let mut rng = Rng::with_seed(5);

        mutate(&mut target, 1., 0.5, &mut rng);

        assert_ne!(target.w1, before.w1);
    }

    /// Мутация без вероятности ничего не меняет
    #[test]
    fn mutation_without_rate_changes_nothing() {
        let mut target = net(4, 1);
        let before = target.clone();
        let mut rng = Rng::with_seed(5);

        mutate(&mut target, 0., 0.5, &mut rng);

        assert_eq!(target, before);
    }

    /// После мутации веса остаются в пределах слоя
    #[test]
    fn mutated_weights_stay_within_limits() {
        let mut target = net(4, 1);
        let limits = limits(4);
        let mut rng = Rng::with_seed(5);

        for _ in 0..100 {
            mutate(&mut target, 1., 1., &mut rng);
        }

        for weight in target.w1.iter().chain(target.b1.iter()) {
            assert!(weight.abs() <= limits.hidden, "вес вне пределов: {weight}");
        }

        for weight in target.w2.iter().chain(target.b2.iter()) {
            assert!(weight.abs() <= limits.output, "вес вне пределов: {weight}");
        }
    }

    /// Одинаковые сети не различаются ни одним геном
    #[test]
    fn equal_nets_have_no_difference() {
        let first = net(4, 1);
        let second = net(4, 1);

        assert_eq!(difference(&first, &second), 0.);
    }

    /// Разные сети различаются генами
    #[test]
    fn different_nets_have_difference() {
        let first = net(4, 1);
        let second = net(4, 2);

        assert!(difference(&first, &second) > 0.9);
    }
}
