#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum OutputResolution {
    #[default]
    Auto,
    Hd720p,
    FullHd1080p,
    Qhd1440p,
}

impl OutputResolution {
    const ALL: [Self; 4] = [Self::Auto, Self::Hd720p, Self::FullHd1080p, Self::Qhd1440p];

    pub fn from_config(value: Option<&str>) -> Self {
        match value
            .unwrap_or_default()
            .trim()
            .to_ascii_lowercase()
            .as_str()
        {
            "720p" | "hd" | "1280x720" => Self::Hd720p,
            "1080p" | "fullhd" | "full-hd" | "1920x1080" => Self::FullHd1080p,
            "1440p" | "qhd" | "2k" | "2560x1440" => Self::Qhd1440p,
            _ => Self::Auto,
        }
    }

    pub fn as_config(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Hd720p => "720p",
            Self::FullHd1080p => "1080p",
            Self::Qhd1440p => "1440p",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Auto => "AUTO",
            Self::Hd720p => "HD 720P",
            Self::FullHd1080p => "FULL HD 1080P",
            Self::Qhd1440p => "QHD 1440P",
        }
    }

    pub fn dimensions(self) -> Option<(u32, u32)> {
        match self {
            Self::Auto => None,
            Self::Hd720p => Some((1280, 720)),
            Self::FullHd1080p => Some((1920, 1080)),
            Self::Qhd1440p => Some((2560, 1440)),
        }
    }

    pub fn next(self) -> Self {
        self.cycle(1)
    }

    pub fn cycle(self, delta: isize) -> Self {
        cycle_value(Self::ALL, self, delta)
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum ScalingFilter {
    #[default]
    Nearest,
    PixelPerfect,
    SharpBilinear,
}

impl ScalingFilter {
    const ALL: [Self; 3] = [Self::Nearest, Self::PixelPerfect, Self::SharpBilinear];

    pub fn from_config(value: Option<&str>) -> Self {
        match value
            .unwrap_or_default()
            .trim()
            .to_ascii_lowercase()
            .as_str()
        {
            "pixel-perfect" | "pixel_perfect" | "pixelperfect" | "integer" => Self::PixelPerfect,
            "sharp-bilinear" | "sharp_bilinear" | "sharpbilinear" | "sharp" => Self::SharpBilinear,
            _ => Self::Nearest,
        }
    }

    pub fn as_config(self) -> &'static str {
        match self {
            Self::Nearest => "nearest",
            Self::PixelPerfect => "pixel-perfect",
            Self::SharpBilinear => "sharp-bilinear",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Nearest => "NEAREST",
            Self::PixelPerfect => "PIXEL PERFECT",
            Self::SharpBilinear => "SHARP BILINEAR",
        }
    }

    pub fn next(self) -> Self {
        self.cycle(1)
    }

    pub fn cycle(self, delta: isize) -> Self {
        cycle_value(Self::ALL, self, delta)
    }
}

fn cycle_value<T, const N: usize>(values: [T; N], current: T, delta: isize) -> T
where
    T: Copy + PartialEq,
{
    let current = values
        .iter()
        .position(|candidate| *candidate == current)
        .unwrap_or(0);
    let next = (current as isize + delta).rem_euclid(N as isize) as usize;
    values[next]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn output_resolution_config_roundtrip_and_dimensions() {
        for resolution in OutputResolution::ALL {
            assert_eq!(
                OutputResolution::from_config(Some(resolution.as_config())),
                resolution
            );
        }
        assert_eq!(
            OutputResolution::from_config(Some("2560x1440")),
            OutputResolution::Qhd1440p
        );
        assert_eq!(
            OutputResolution::FullHd1080p.dimensions(),
            Some((1920, 1080))
        );
        assert_eq!(OutputResolution::Auto.dimensions(), None);
    }

    #[test]
    fn scaling_filter_config_roundtrip() {
        for filter in ScalingFilter::ALL {
            assert_eq!(ScalingFilter::from_config(Some(filter.as_config())), filter);
        }
        assert_eq!(
            ScalingFilter::from_config(Some("integer")),
            ScalingFilter::PixelPerfect
        );
    }

    #[test]
    fn video_options_cycle_in_both_directions() {
        assert_eq!(OutputResolution::Auto.cycle(-1), OutputResolution::Qhd1440p);
        assert_eq!(
            ScalingFilter::Nearest.cycle(-1),
            ScalingFilter::SharpBilinear
        );
        assert_eq!(ScalingFilter::SharpBilinear.next(), ScalingFilter::Nearest);
    }
}
