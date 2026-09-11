//! Alan bilgisi — plan §8.1: iş mantığında birim daima mm, alan [0, 5000]².
//! Arayüzdeki 5000 hesabı buradan türetilir; ikinci bağımsız 5000 yazılmaz.

/// Sabit alan kenar uzunluğu (mm).
pub const AREA_SIZE_MM: f64 = 5000.0;

/// Alanın toplam yüzölçümü (mm²).
pub const AREA_MM2: f64 = AREA_SIZE_MM * AREA_SIZE_MM;

/// Nokta kapalı alanın içinde mi (sınır teması dahil; `linear_epsilon_mm` toleranslı).
#[must_use]
pub fn point_in_area(x: f64, y: f64, linear_epsilon_mm: f64) -> bool {
    x >= -linear_epsilon_mm
        && y >= -linear_epsilon_mm
        && x <= AREA_SIZE_MM + linear_epsilon_mm
        && y <= AREA_SIZE_MM + linear_epsilon_mm
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn area_is_twenty_five_square_meters() {
        assert_eq!(AREA_MM2, 25_000_000.0);
    }

    #[test]
    fn boundary_points_are_inside() {
        assert!(point_in_area(0.0, 0.0, 1e-9));
        assert!(point_in_area(5000.0, 5000.0, 1e-9));
        assert!(!point_in_area(5000.0 + 1e-9, 0.0, 0.0));
        assert!(point_in_area(5000.0 + 0.005, 0.0, 0.01));
    }
}
