//! Uygulama modeli — plan §8.3. Jagua tipi içermez; host bağımsızdır.

use serde::{Deserialize, Serialize};

use crate::geometry::Polygon;

/// Poz sözleşmesi: mm ve radyan. Derece dönüşümü yalnızca SVG metninde yapılır.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Pose {
    pub x_mm: f64,
    pub y_mm: f64,
    pub rotation_rad: f64,
}

impl Pose {
    #[must_use]
    pub const fn new(x_mm: f64, y_mm: f64, rotation_rad: f64) -> Self {
        Self {
            x_mm,
            y_mm,
            rotation_rad,
        }
    }
}

/// Bir ürünün geometrisi: birden fazla footprint parçası olabilir; safety zone
/// ilk sözleşmede tek, kapalı, deliksiz basit poligondur.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProductGeometry {
    pub id: String,
    pub footprint_polygons: Vec<Polygon>,
    pub safety_zone: Polygon,
    /// Canonical (ortak origin'li) safety alanı, mm².
    pub safety_area_mm2: f64,
    /// Kaynak dosya/metadata notu; opsiyonel.
    pub source_metadata: Option<String>,
}

/// Yerleşim: ürün kimliği + poz.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Placement {
    pub product_id: String,
    pub pose: Pose,
}
