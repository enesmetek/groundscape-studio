//! Hata sözlüğü — plan §19. Kodlar Worker protokolünde (Aşama G) string olarak
//! serileştirilir; arayüz açıklamayı gösterir, stack trace değil.

use dxf::enums::Units;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ImportError {
    #[error("DXF parse failed")]
    DxfParse(#[from] dxf::DxfError),
    #[error("drawing unit unspecified")]
    UnitUnspecified,
    #[error("drawing unit not supported: {0:?}")]
    UnitNotSupported(Units),
    #[error("missing layer: {0}")]
    MissingLayer(&'static str),
    #[error("unsupported entity in selected layer: {0}")]
    UnsupportedEntity(&'static str),
    #[error("contour is not closed")]
    OpenContour,
    #[error("invalid polygon: {0}")]
    InvalidPolygon(&'static str),
    #[error("footprint extends outside safety zone")]
    FootprintOutsideSafety,
}

impl ImportError {
    /// Plan §19 hata kodu; TypeScript hata sözlüğüyle aynı adlar.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::DxfParse(_) => "DXF_PARSE_FAILED",
            Self::UnitUnspecified => "UNIT_UNSPECIFIED",
            Self::UnitNotSupported(_) => "UNIT_NOT_SUPPORTED",
            Self::MissingLayer(_) => "MISSING_LAYER",
            Self::UnsupportedEntity(_) => "UNSUPPORTED_ENTITY",
            Self::OpenContour => "OPEN_CONTOUR",
            Self::InvalidPolygon(_) => "INVALID_POLYGON",
            Self::FootprintOutsideSafety => "FOOTPRINT_OUTSIDE_SAFETY",
        }
    }
}
