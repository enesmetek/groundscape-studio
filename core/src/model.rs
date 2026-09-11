//! Uygulama modeli — plan §8.3. Jagua tipi içermez; host bağımsızdır.

use serde::{Deserialize, Serialize};

use crate::geometry::Polygon;

/// Poz sözleşmesi: mm ve radyan. Derece dönüşümü yalnızca SVG metninde yapılır.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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

/// Yerleşim rolü: güvenlik sınıfı değil, tasarım tercihi (MVP-2 plan §2.2).
/// `auto` rolünde ürün türü çıkarımı yapılmaz; yalnızca geometriye bakılır.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PlacementRole {
    #[default]
    Auto,
    Anchor,
    Distributed,
    Peripheral,
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
    /// Canonical footprint alanı, mm² — tüm parçaların toplamı; import'ta
    /// bir kez hesaplanır (MVP-2 plan P1).
    pub footprint_area_mm2: f64,
    /// Canonical koordinatta footprint parçalarının alan ağırlıklı merkez
    /// noktası; import'ta bir kez hesaplanır (MVP-2 plan P1).
    pub footprint_centroid_local: [f64; 2],
    /// Yerleşim rolü; yoksa `auto` (MVP-2 plan P1).
    #[serde(default)]
    pub placement_role: PlacementRole,
    /// Fonksiyon etiketleri; Faz 1'de hiçbir kod okumaz (MVP-2 plan P1).
    #[serde(default)]
    pub tags: Vec<String>,
    /// Yaş grubu; Faz 1'de hiçbir kod okumaz (MVP-2 plan P1).
    #[serde(default)]
    pub age_group: Option<String>,
    /// Kaynak dosya/metadata notu; opsiyonel.
    pub source_metadata: Option<String>,
}

/// Yerleşim: ürün kimliği + poz.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Placement {
    pub product_id: String,
    pub pose: Pose,
}
