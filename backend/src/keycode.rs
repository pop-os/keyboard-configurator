// has_keycode logic; convert to/from int
// serialize (Display?)/format
// serde: serialize/deserialize as string

use bitflags::bitflags;

bitflags! {
    pub struct Mods: u16 {
        const CTRL = 0x1;
        const SHIFT = 0x2;
        const ALT = 0x4;
        const SUPER = 0x8;

        const RIGHT = 0x10;

        const RIGHT_CTRL = Self::RIGHT.bits | Self::CTRL.bits;
        const RIGHT_SHIFT = Self::RIGHT.bits | Self::SHIFT.bits;
        const RIGHT_ALT = Self::RIGHT.bits | Self::ALT.bits;
        const RIGHT_SUPER = Self::RIGHT.bits | Self::SUPER.bits;
    }
}

impl Default for Mods {
    fn default() -> Self {
        Self::empty()
    }
}

impl Mods {
    // Convert single modifier from name
    pub fn from_mod_str(s: &str) -> Option<Self> {
        match s {
            "LEFT_CTRL" => Some(Self::CTRL),
            "LEFT_SHIFT" => Some(Self::SHIFT),
            "LEFT_ALT" => Some(Self::ALT),
            "LEFT_SUPER" => Some(Self::SUPER),
            "RIGHT_CTRL" => Some(Self::RIGHT_CTRL),
            "RIGHT_SHIFT" => Some(Self::RIGHT_SHIFT),
            "RIGHT_ALT" => Some(Self::RIGHT_ALT),
            "RIGHT_SUPER" => Some(Self::RIGHT_SUPER),
            _ => None,
        }
    }

    // Convert to single modifier
    pub(crate) fn as_mod_str(self) -> Option<&'static str> {
        match self {
            Self::CTRL => Some("LEFT_CTRL"),
            Self::SHIFT => Some("LEFT_SHIFT"),
            Self::ALT => Some("LEFT_ALT"),
            Self::SUPER => Some("LEFT_SUPER"),
            Self::RIGHT_CTRL => Some("RIGHT_CTRL"),
            Self::RIGHT_SHIFT => Some("RIGHT_SHIFT"),
            Self::RIGHT_ALT => Some("RIGHT_ALT"),
            Self::RIGHT_SUPER => Some("RIGHT_SUPER"),
            _ => None,
        }
    }

    pub fn mod_names(self) -> impl Iterator<Item = &'static str> {
        [Self::CTRL, Self::SHIFT, Self::ALT, Self::SUPER]
            .iter()
            .filter_map(move |i| (self & (*i | Self::RIGHT)).as_mod_str())
    }

    pub fn toggle_mod(self, other: Self) -> Self {
        let other_key = other & !Self::RIGHT;
        if !self.contains(other_key) {
            self | other
        } else {
            let key = self & !other_key;
            if key == Self::RIGHT {
                Self::empty()
            } else {
                key
            }
        }
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, glib::Boxed)]
#[boxed_type(name = "S76Keycode")]
pub enum Keycode {
    Basic(Mods, String),
    MT(Mods, String),
    LT(u8, String),
}

impl Keycode {
    pub fn parse(s: &str) -> Option<Self> {
        let mut tokens = tokenize(s);
        match tokens.next()? {
            "MT" => parse_mt(tokens),
            "LT" => parse_lt(tokens),
            keycode => parse_basic(tokenize(s)),
        }
    }

    pub fn none() -> Self {
        Self::Basic(Mods::empty(), "NONE".to_string())
    }

    pub fn is_none(&self) -> bool {
        if let Keycode::Basic(mode, keycode) = self {
            mode.is_empty() && keycode.as_str() == "NONE"
        } else {
            false
        }
    }

    pub fn is_roll_over(&self) -> bool {
        if let Keycode::Basic(mode, keycode) = self {
            mode.is_empty() && keycode.as_str() == "ROLL_OVER"
        } else {
            false
        }
    }
}

impl std::fmt::Display for Keycode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Basic(mods, scancode_name) => {
                let mut has_mod = false;
                for mod_name in mods.mod_names() {
                    if has_mod {
                        write!(f, " | ")?;
                    }
                    write!(f, "{}", mod_name)?;
                }
                if !(scancode_name == "NONE" && has_mod) {
                    write!(f, "{}", scancode_name)?;
                }
            }
            Self::MT(mods, scancode_name) => {
                write!(f, "MT(")?;
                let mut has_mod = false;
                for mod_name in mods.mod_names() {
                    if has_mod {
                        write!(f, " | ")?;
                    }
                    write!(f, "{}", mod_name)?;
                }
                write!(f, ", {})", scancode_name)?;
            }
            Self::LT(layer, scancode_name) => {
                write!(f, "LT({}, {})", layer, scancode_name)?;
            }
        }
        Ok(())
    }
}

const SEPARATORS: &[char] = &[',', '|', '(', ')'];

// Tokenize into iterator of &str, splitting on whitespace and putting
// separators in their own tokens.
fn tokenize(mut s: &str) -> impl Iterator<Item = &str> {
    std::iter::from_fn(move || {
        s = s.trim_start_matches(' ');
        let idx = if SEPARATORS.contains(&s.chars().next()?) {
            1
        } else {
            s.find(|c| c == ' ' || SEPARATORS.contains(&c))
                .unwrap_or(s.len())
        };
        let tok = &s[..idx];
        s = &s[idx..];
        Some(tok)
    })
}

fn parse_mt<'a>(mut tokens: impl Iterator<Item = &'a str>) -> Option<Keycode> {
    if tokens.next() != Some("(") {
        return None;
    }

    let mut mods = Mods::empty();
    loop {
        mods |= Mods::from_mod_str(tokens.next()?)?;
        match tokens.next()? {
            "|" => {}
            "," => {
                break;
            }
            _ => {
                return None;
            }
        }
    }

    let keycode = tokens.next()?.to_string();

    if (tokens.next(), tokens.next()) != (Some(")"), None) {
        return None;
    }

    Some(Keycode::MT(mods, keycode))
}

fn parse_lt<'a>(mut tokens: impl Iterator<Item = &'a str>) -> Option<Keycode> {
    if tokens.next() != Some("(") {
        return None;
    }

    let layer = tokens.next()?.parse().ok()?;

    if tokens.next() != Some(",") {
        return None;
    }

    let keycode = tokens.next()?.to_string();

    if (tokens.next(), tokens.next()) != (Some(")"), None) {
        return None;
    }

    Some(Keycode::LT(layer, keycode))
}

// XXX limit to basic if there are mods?
fn parse_basic<'a>(mut tokens: impl Iterator<Item = &'a str>) -> Option<Keycode> {
    let mut mods = Mods::empty();
    let mut keycode = None;

    loop {
        let token = tokens.next()?;
        if let Some(mod_) = Mods::from_mod_str(token) {
            mods |= mod_;
        } else if keycode.is_none() && token.chars().next()?.is_alphanumeric() {
            keycode = Some(token.to_string());
        } else {
            return None;
        }
        match tokens.next() {
            Some("|") => {}
            Some(_) => {
                return None;
            }
            None => {
                break;
            }
        }
    }

    Some(Keycode::Basic(
        mods,
        keycode.unwrap_or_else(|| "NONE".to_string()),
    ))
}
