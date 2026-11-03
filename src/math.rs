pub use core::f32::consts::*;
use glam::Vec2;

/// Квадратный корень числа, подходит для констант
pub const fn sqrt_const(x: f32) -> f32 {
    if x < 0.0 {
        return f32::NAN;
    }
    if x == 0.0 || x.is_nan() {
        return x;
    }
    if x.is_infinite() {
        return x;
    }

    // Начальное приближение через нормализацию: приводим x к [0.25, 1)
    // путём умножения/деления на степени 4, чтобы избежать проблем с диапазоном.
    let mut m = x;
    let mut scale = 0i32;
    while m >= 1.0 {
        m *= 0.25;
        scale += 1;
    }
    while m < 0.0625 {
        m *= 4.0;
        scale -= 1;
    }

    // Для m ∈ [0.0625, 1) хорошее стартовое приближение: y ≈ 0.5 + m
    let mut y = 0.5 + 0.5 * m;
    let mut i = 0;
    while i < 6 {
        y = 0.5 * (y + m / y);
        i += 1;
    }

    // Возвращаем масштаб: sqrt(m * 4^scale) = sqrt(m) * 2^scale
    // 2^scale считаем через повторное умножение (const fn не имеет powi).
    let mut factor = 1.0f32;
    let mut s = scale;
    if s >= 0 {
        while s > 0 {
            factor *= 2.0;
            s -= 1;
        }
        y * factor
    } else {
        s = -s;
        while s > 0 {
            factor *= 0.5;
            s -= 1;
        }
        y * factor
    }
}

/// Квадрат числа, подходит для констант
pub const fn square(x: f32) -> f32 {
    x * x
}

/// Нормализует угол, заключая его в диапазоне -PI..PI
pub const fn normalize_angle(mut x: f32) -> f32 {
    x %= TAU;

    if x > PI {
        x - TAU
    } else if x < -PI {
        x + TAU
    } else {
        x
    }
}

/// Складывает векторы и сокращает длину
pub fn add_vector_with_min(v1: Vec2, v2: Vec2, len: f32) -> Vec2 {
    let new_vec = v1 + v2;
    new_vec.normalize_or_zero() * new_vec.length().min(len)
}

pub fn shorten_vector(v: Vec2, dlen: f32) -> Vec2 {
    v.normalize_or_zero() * (v.length() - dlen).max(0.)
}

/// Вычитает `d` из модуля `x`
pub fn step_to_zero(x: f32, d: f32) -> f32 {
    if x.abs() <= d { 0. } else { x - d * x.signum() }
}

/// Углы сравниваются с небольшой погрешностью
#[cfg(test)]
pub fn approx_eq(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-4
}

/// Количество тиков (шагов симуляции) в секунду
pub const TPS: u32 = 50;
/// Длительность одного тика симуляции (delta time), секунды
pub const DT: f32 = 1. / TPS as f32;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn sqrt_const_test() {
        assert_eq!(sqrt_const(4.), 2.);
        assert_eq!(sqrt_const(10000.), 100.);
        assert_eq!(sqrt_const(2.), SQRT_2);

        assert!(sqrt_const(-2.).is_nan());

        assert_eq!(sqrt_const(0.), 0.);

        assert!(sqrt_const(f32::NAN).is_nan());

        assert_eq!(sqrt_const(f32::INFINITY), f32::INFINITY);
    }

    #[test]
    pub fn normalize_angle_test() {
        assert_eq!(normalize_angle(0.), 0.);
        assert_eq!(normalize_angle(PI), PI);

        // Положительные углы, большие PI, сворачиваются вниз
        assert!(approx_eq(normalize_angle(4.), 4. - TAU));
        assert!(approx_eq(normalize_angle(PI + 0.5), PI + 0.5 - TAU));

        // Отрицательные углы, меньшие -PI, сворачиваются вверх
        assert!(approx_eq(normalize_angle(-4.), -4. + TAU));
        assert!(approx_eq(normalize_angle(-PI - 0.5), -PI - 0.5 + TAU));

        // Углы внутри диапазона не меняются
        assert!(approx_eq(normalize_angle(1.2), 1.2));
        assert!(approx_eq(normalize_angle(-1.2), -1.2));

        // После нормализации угол всегда лежит в (-PI..PI]
        for i in -20..20 {
            let x = i as f32 * 0.73;
            let n = normalize_angle(x);

            assert!(n > -PI && n <= PI, "{} -> {}", x, n);
        }
    }

    #[test]
    pub fn step_to_zero_test() {
        assert_eq!(step_to_zero(0.5, 0.2), 0.3);
        assert_eq!(step_to_zero(-0.5, 0.2), -0.3);

        // Шаг больше модуля обнуляет значение
        assert_eq!(step_to_zero(0.1, 0.2), 0.);
        assert_eq!(step_to_zero(-0.1, 0.2), 0.);
        assert_eq!(step_to_zero(0., 0.2), 0.);
    }
}
