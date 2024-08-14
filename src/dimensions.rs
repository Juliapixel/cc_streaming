pub enum Dimension {
    Width,
    Height
}

pub enum ResolutionHint {
    FixedAspect {
        dimension: Dimension,
        size: u32,
    },
    FixedResolution {
        width: u32,
        height: u32
    },
}

impl ResolutionHint {
    pub fn fixed_aspect(dimension: Dimension, size: u32) -> Self {
        Self::FixedAspect { dimension, size }
    }

    pub fn fixed_resolution(width: u32, height: u32) -> Self {
        Self::FixedResolution { width, height }
    }

    #[inline]
    pub fn get_target_res(&self, original_width: u32, original_height: u32) -> (u32, u32) {
        match *self {
            ResolutionHint::FixedAspect { dimension: Dimension::Width, size } => {
                let aspect = original_width as f64 / original_height as f64;
                (size, (size as f64 / aspect).round() as u32 )
            },
            ResolutionHint::FixedAspect { dimension: Dimension::Height, size } => {
                let aspect = original_width as f64 / original_height as f64;
                ((size as f64 * aspect).round() as u32, size)
            },
            ResolutionHint::FixedResolution { width, height } => (width, height),
        }
    }
}
