use crate::colors::CoolLEDColors;
use crate::coolled::CoolLEDWriter;

#[cfg(not(feature = "custom_charset"))]
const FONT_DATA: &[u8; 2097152] = include_bytes!("../assets/font_data.bin");


impl<'a> CoolLEDWriter<'a> {
    #[cfg(not(feature = "custom_charset"))]
    //Read 32 bytes from the font data
    fn read_font_bytes(&self, c: char) -> &'static [u8] {
        &FONT_DATA[(c as u16 * ' ' as u16) as usize..(c as u16 * ' ' as u16 + 32) as usize]
    }

    #[cfg(feature = "custom_charset")]
    //Read 32 bytes from the font data
    fn read_font_bytes(&self, character: char) -> &'a [u8] {
        if let Some(pos) = self.custom_charset_list.chars().position(|needle_char| needle_char == character) {
            let addr = pos * 0x20 as usize;
            &self.custom_charset[addr..addr+0x20]
        }else{
            &self.custom_charset[0..0x20]
        }
    }

    fn delete_empty_column(data: &[u8], out: &mut [u8]) -> usize {
        let first_non_zero_idx_inc = {
            let mut idxf = None;
            for current_idx in (0..31).step_by(2) {
                if data[current_idx] != 0 || data[current_idx + 1] != 0 {
                    idxf = Some(current_idx);
                    break;
                }
            }
            idxf
        };

        if let Some(first_non_zero_idx_inc) = first_non_zero_idx_inc {
            let first_non_zero_idx_dec = {
                let mut idxf = None;
                for current_rev in (0..31).step_by(2).rev() {
                    if data[current_rev] != 0 || data[current_rev + 1] != 0 {
                        idxf = Some(current_rev);
                        break;
                    }
                }
                idxf
            };

            if let Some(first_non_zero_idx_dec) = first_non_zero_idx_dec {
                match first_non_zero_idx_inc.cmp(&first_non_zero_idx_dec) {
                    core::cmp::Ordering::Less => {
                        let src = &data[first_non_zero_idx_inc..first_non_zero_idx_dec + 2];
                        out[0..src.len()].clone_from_slice(src);
                        return src.len();
                    }
                    core::cmp::Ordering::Equal => {
                        let src = &data[first_non_zero_idx_inc..first_non_zero_idx_inc + 2];
                        out[0..src.len()].clone_from_slice(src);
                        return src.len();
                    }
                    core::cmp::Ordering::Greater => {
                        out[0..16].fill(0);
                        return out[0..16].len();
                    }
                }
            }
        }

        out[0..16].fill(0);
        out[0..16].len()
    }

    pub fn get_font_byte_trimmed(&self, character: char, i: i32, out: &mut [u8]) -> usize {
        let unicode_data = self.read_font_bytes(character);

        if i == 2 || i == 3 {
            let data_size = Self::delete_empty_column(unicode_data, out);
            out[data_size] = 0;
            out[data_size + 1] = 0;
            out[0..=data_size + 1].len()
        } else {
            Self::delete_empty_column(unicode_data, out)
        }
    }

    //We write the bytes of the character if the character color
    //needs the current cycle color.
    //We cycle between RGB.
    //i.e if the character color is yellow, we need red and green, and
    //we skip blue writing 16x 0x00
    pub fn get_font_byte_with_color(
        &self,
        c: char,
        i: i32,
        idx: usize,
        len_chars: usize,
        character_color: CoolLEDColors,
        out: &mut [u8],
    ) -> usize {
        let current_cycle = idx / len_chars;

        let current_char_size = self.get_font_byte_trimmed(c, i, out);

        let cycle_color = match current_cycle {
            1 => CoolLEDColors::Green,
            2 => CoolLEDColors::Blue,
            _ => CoolLEDColors::Red,
        };
        if character_color.has(cycle_color) {
            current_char_size
        } else {
            let dst = &mut out[0..16];
            dst.fill(0);
            dst.len()
        }
    }
}
