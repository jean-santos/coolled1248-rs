#[derive(PartialEq, Clone, Copy, Debug)]
pub enum CoolLEDColors {
    White,
    Red,
    Green,
    Blue,
    Yellow,
    Pink,
    Cyan,
}

impl CoolLEDColors {
    pub fn has(&self, color: CoolLEDColors) -> bool {
        match &self {
            CoolLEDColors::White => true,
            CoolLEDColors::Red => color == CoolLEDColors::Red,
            CoolLEDColors::Green => color == CoolLEDColors::Green,
            CoolLEDColors::Blue => color == CoolLEDColors::Blue,
            CoolLEDColors::Yellow => color == CoolLEDColors::Red || color == CoolLEDColors::Green,
            CoolLEDColors::Pink => color == CoolLEDColors::Red || color == CoolLEDColors::Blue,
            CoolLEDColors::Cyan => color == CoolLEDColors::Blue || color == CoolLEDColors::Green,
        }
    }
}
