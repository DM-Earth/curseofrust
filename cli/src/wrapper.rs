use curseofrust::{grid::Stencil, Difficulty, Speed};

use crate::Error;

pub struct StencilWrapper(pub Stencil);

impl std::str::FromStr for StencilWrapper {
    type Err = Error;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(match s {
            "rhombus" => Stencil::Rhombus,
            "rect" => Stencil::Rect,
            "hex" => Stencil::Hex,
            _ => {
                return Err(Error::UnknowVariant {
                    ty: "shape",
                    variants: &["rhombus", "rect", "hex"],
                    value: s.to_owned(),
                })
            }
        }))
    }
}

pub struct DifficultyWrapper(pub Difficulty);

impl std::str::FromStr for DifficultyWrapper {
    type Err = Error;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(match s {
            "ee" => Difficulty::Easiest,
            "e" => Difficulty::Easy,
            "n" => Difficulty::Normal,
            "h" => Difficulty::Hard,
            "hh" => Difficulty::Hardest,
            _ => {
                return Err(Error::UnknowVariant {
                    ty: "difficulty",
                    variants: &["ee", "e", "n", "h", "hh"],
                    value: s.to_owned(),
                })
            }
        }))
    }
}

pub struct SpeedWrapper(pub Speed);

impl std::str::FromStr for SpeedWrapper {
    type Err = Error;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(match s {
            "p" => Speed::Pause,
            "sss" => Speed::Slowest,
            "ss" => Speed::Slower,
            "s" => Speed::Slow,
            "n" => Speed::Normal,
            "f" => Speed::Fast,
            "ff" => Speed::Faster,
            "fff" => Speed::Fastest,
            _ => {
                return Err(Error::UnknowVariant {
                    ty: "speed",
                    variants: &["p", "sss", "ss", "s", "n", "f", "ff", "fff"],
                    value: s.to_owned(),
                })
            }
        }))
    }
}
