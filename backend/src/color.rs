use ordered_float::NotNan;
use palette::{Component, IntoColor, RgbHue};
use serde::{Deserialize, Serialize};
use std::f64::consts::PI;

type PaletteHsv = palette::Hsv<palette::encoding::Srgb, f64>;
type PaletteLinSrgb = palette::LinSrgb<f64>;

/// Floating point hue/saturation color
#[derive(
    Clone,
    Copy,
    Debug,
    Serialize,
    Deserialize,
    Default,
    PartialEq,
    glib::GBoxed,
    Hash,
    Eq,
    Ord,
    PartialOrd,
)]
#[gboxed(type_name = "S76Hs")]
pub struct Hs {
    /// Hue, in radians
    pub h: NotNan<f64>,
    /// Saturation, from 0.0 to 1.0
    pub s: NotNan<f64>,
}

impl Hs {
    pub fn new(h: f64, s: f64) -> Self {
        Self {
            h: NotNan::new(h).unwrap(),
            s: NotNan::new(s).unwrap(),
        }
    }

    pub fn from_ints(h: u8, s: u8) -> Self {
        Self::new(h.convert::<f64>() * (2. * PI), s.convert())
    }

    pub fn to_ints(self) -> (u8, u8) {
        let h = (self.h / (2. * PI)).rem_euclid(1.);
        (h.convert(), self.s.convert())
    }

    pub fn to_rgb(self) -> Rgb {
        let hue = RgbHue::from_radians(*self.h);
        let hsv = PaletteHsv::new(hue, *self.s, 1.);
        let rgb: PaletteLinSrgb = hsv.into_rgb();
        let (r, g, b) = rgb.into_components();
        Rgb::from_floats(r, g, b)
    }
}

/// Integer RGB color
#[derive(Clone, Copy, Debug, Serialize, Deserialize, Default, glib::GBoxed)]
#[gboxed(type_name = "S76Rgb")]
pub struct Rgb {
    /// Red
    pub r: u8,
    /// Green
    pub g: u8,
    /// Blue
    pub b: u8,
}

impl Rgb {
    pub fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    pub fn from_floats(r: f64, g: f64, b: f64) -> Self {
        Self {
            r: r.convert(),
            g: g.convert(),
            b: b.convert(),
        }
    }

    pub fn to_floats(self) -> (f64, f64, f64) {
        (self.r.convert(), self.g.convert(), self.b.convert())
    }

    /// Convert to hexadecimal string
    pub fn to_string(self) -> String {
        format!("{:02x}{:02x}{:02x}", self.r, self.g, self.b)
    }

    /// Parse from hexadecimal string
    pub fn parse(s: &str) -> Option<Self> {
        if s.len() == 6 {
            let r = u8::from_str_radix(&s[0..2], 16).ok()?;
            let g = u8::from_str_radix(&s[2..4], 16).ok()?;
            let b = u8::from_str_radix(&s[4..6], 16).ok()?;
            Some(Self::new(r, g, b))
        } else {
            None
        }
    }

    #[allow(clippy::many_single_char_names)]
    pub fn to_hs_lossy(self) -> Hs {
        let (r, g, b) = self.to_floats();
        let rgb = PaletteLinSrgb::new(r, g, b);
        let hsv: PaletteHsv = rgb.into_hsv();
        let (h, s, _) = hsv.into_components();
        Hs::new(h.to_radians(), s)
    }
}

#[cfg(test)]
mod test {
    use crate::*;

    #[test]
    fn test_hs_rgb_hs() {
        let hs1 = Hs::new(0.3, 0.4);
        let hs2 = hs1.to_rgb().to_hs_lossy();
        let hs3 = hs2.to_rgb().to_hs_lossy();
        assert!((hs1.h - hs2.h).abs() < 0.01);
        assert!((hs1.s - hs2.s).abs() < 0.01);
        assert!((hs2.h - hs3.h).abs() < 0.0001);
        assert!((hs2.s - hs3.s).abs() < 0.0001);
    }
}
