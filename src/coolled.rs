use core::ops::Shr;

use crate::{
    colors::CoolLEDColors,
    ncycle::Ncycles,
    packets::PacketType,
    util::{calculate_checksum, escape_byets_in_place},
};

const TEXT_PREFIX_FIRST_PACKET_HEADER_SIZE: usize = 107;
const IMAGE_PREFIX_FIRST_PACKET_HEADER_SIZE: usize = 26;
const ANIMATION_PREFIX_FIRST_PACKET_HEADER_SIZE: usize = 27;

pub enum PayloadType<'b> {
    //Text data, slice of colors
    Text(&'b str, &'b [CoolLEDColors]),
    //Image data
    Image(&'b [u8]),
    //Animation data, animation frames
    Animation(&'b [u8],usize),
}

impl<'b> PayloadType<'b> {
    pub fn get_content_type(&self) -> u8 {
        match self {
            PayloadType::Text(_,_) => PacketType::Text as u8,
            PayloadType::Image(_) => PacketType::Draw as u8,
            PayloadType::Animation(_,_) => PacketType::Animate as u8,
        }
    }
}

pub struct CoolLEDWriter<'a> {
    payload: PayloadType<'a>,
    #[cfg(feature = "custom_charset")]
    pub(crate) custom_charset: &'a [u8],
    #[cfg(feature = "custom_charset")]
    pub(crate) custom_charset_list: &'a str,
}


impl<'a> CoolLEDWriter<'a> {
    pub fn get_packets_count(&self) -> usize {
        (self.get_total_bytes_from_phrase_data() + self.get_padding()).div_ceil(128)
    }

    fn get_total_bytes_from_phrase_data(&self) -> usize {
        match self.payload {
            PayloadType::Text(phrase,_) => phrase
                .chars()
                .map(|c| {
                    let mut buf: [u8; 16] = [0; 16];
                    self.get_font_byte_trimmed(c, 2, &mut buf) * 3
                })
                .sum(),
            PayloadType::Image(data) => data.len(),
            PayloadType::Animation(data,_) => data.len(),
        }
    }

    #[cfg(not(feature = "custom_charset"))]
    pub fn new(payload: PayloadType<'a>) -> Self {
        Self {
            payload,
        }
    }

    #[cfg(feature = "custom_charset")]
    pub fn new(payload: PayloadType<'a>, custom_charset: &'a [u8], custom_charset_list: &'a str) -> Self {
        Self {
            payload,
            custom_charset,
            custom_charset_list
        }
    }

    fn make_n_packet(&self, idx: usize, out: &mut [u8]) -> usize {
        let padding = self.get_padding();

        let whole_packet_size = self.get_total_bytes_from_phrase_data() + padding;

        let already_sended = if idx == 0 {
            0
        } else {
            (128 - padding) + 128 * (idx - 1)
        };

        let skip_bytes = already_sended;
        let missing_bytes = whole_packet_size - padding - already_sended;

        let bytes_needed = if missing_bytes > 128 {
            128
        } else {
            missing_bytes
        };

        match self.payload {
            PayloadType::Text(phrase, colors) => {
                self.write_bytes_from_phrase(out, skip_bytes, bytes_needed, phrase, colors)
            }
            PayloadType::Image(image_data) => {
                self.write_bytes_from_image(out, skip_bytes, bytes_needed, image_data)
            }
            PayloadType::Animation(ani_data,_) => {
                self.write_bytes_from_image(out, skip_bytes, bytes_needed, ani_data)
            }
        }
    }

    fn write_bytes_from_image(
        &self,
        out: &mut [u8],
        skip: usize,
        bytes_needed: usize,
        data: &[u8],
    ) -> usize {
        let mut bytes_wrote = 0;
        let it = data.iter().enumerate().skip(skip).take(bytes_needed);

        for (idx, c) in it {
            out[idx - skip] = *c;
            bytes_wrote += 1;
        }

        bytes_wrote
    }

    fn write_bytes_from_phrase(
        &self,
        out: &mut [u8],
        skip: usize,
        bytes_needed: usize,
        phrase: &str,
        colors: &[CoolLEDColors],
    ) -> usize {
        let mut bytes_wrote = 0;

        let rgb_phrase = Ncycles::new(phrase.chars(), 3);

        let len_chars = phrase.chars().count();
        let colors_len = colors.len();

        let mut buff: [u8; 16] = [0; 16];

        let it = rgb_phrase.enumerate();

        let mut skip_needed = skip;
        let mut bytes_needed_count = bytes_needed;
        let mut out_idx = 0;
        for (idx, current_char) in it {
            let char_size = self.get_font_byte_with_color(
                current_char,
                2,
                idx,
                len_chars,
                colors[idx % colors_len],
                &mut buff,
            );

            if skip_needed > char_size {
                skip_needed -= char_size;
            } else if skip_needed > 0 && skip_needed < char_size && bytes_needed_count > 0 {
                let upper_bound = core::cmp::min(bytes_needed_count, char_size);
                let range = skip_needed..upper_bound;
                let range_len = range.len();
                for out_byte_idx in range {
                    out[out_idx] = buff[out_byte_idx];
                    out_idx += 1;
                    bytes_wrote += 1;
                }
                skip_needed = 0;
                bytes_needed_count -= range_len;
            } else if bytes_needed_count > 0 {
                let upper_bound = core::cmp::min(bytes_needed_count, char_size);
                for item in buff.iter().take(upper_bound) {
                    out[out_idx] = *item;
                    out_idx += 1;
                    bytes_wrote += 1;
                }

                bytes_needed_count -= upper_bound;
            }
        }

        bytes_wrote
    }

    fn make_text_payload(&self, out: &mut [u8], phrase: &str, colors: &[CoolLEDColors]) -> usize {
        let data_size = self.get_total_bytes_from_phrase_data();

        //length of string
        out[0] = 0;

        //character string
        out[1..81].fill(0);

        out[81] = data_size.shr(8) as u8;
        out[82] = (data_size & 0xff) as u8;

        let range_remaing = 83..out.len();
        let bytes_available = range_remaing.len();

        self.write_bytes_from_phrase(&mut out[range_remaing.start..], 0, bytes_available, phrase, colors)
    }

    fn make_image_payload(&self, out: &mut [u8], image_data: &[u8]) -> usize {
        let data_size = self.get_total_bytes_from_phrase_data();
        out[0] = data_size.shr(8) as u8;
        out[1] = (data_size & 0xff) as u8;

        let range_remaing = 2..out.len();

        let bytes_available = range_remaing.len();

        log::info!(
            "Remaining space : {:?}, available : {}",
            &range_remaing,
            bytes_available
        );

        self.write_bytes_from_image(
            &mut out[range_remaing.start..],
            0,
            bytes_available,
            image_data,
        )
    }

    fn make_animation_payload(
        &self,
        out: &mut [u8],
        animation_data: &[u8],
        frames: usize,
        speed: usize,
    ) -> usize {
        //frames qt
        out[0] = frames as u8;
        out[1] = (speed >> 8) as u8;
        out[2] = (speed & 0xff) as u8;

        let range_remaing = 3..out.len();

        let bytes_available = range_remaing.len();

        log::info!(
            "Remaining space : {:?}, available : {}",
            &range_remaing,
            bytes_available
        );

        self.write_bytes_from_image(
            &mut out[range_remaing.start..],
            0,
            bytes_available,
            animation_data,
        )
    }

    fn make_first_packet(&self, out: &mut [u8]) -> usize {
        out[0..24].fill(0x0);

        match self.payload {
            PayloadType::Text(phrase,colors) => self.make_text_payload(&mut out[24..], phrase, colors),
            PayloadType::Image(image_data) => self.make_image_payload(&mut out[24..], image_data),
            PayloadType::Animation(ani_data,frames_quantity) => {
                self.make_animation_payload(&mut out[24..], ani_data, frames_quantity, 500)
            }
        }
    }

    fn get_padding(&self) -> usize {
        match self.payload {
            PayloadType::Text(_,_) => TEXT_PREFIX_FIRST_PACKET_HEADER_SIZE,
            PayloadType::Image(_) => IMAGE_PREFIX_FIRST_PACKET_HEADER_SIZE,
            PayloadType::Animation(_,_) => ANIMATION_PREFIX_FIRST_PACKET_HEADER_SIZE,
        }
    }

    fn make_subpacket(&mut self, idx: usize, out: &mut [u8]) -> usize {
        let whole_packet_size = match self.payload {
            PayloadType::Text(_,_) => self.get_padding() + self.get_total_bytes_from_phrase_data(),
            PayloadType::Image(_) => self.get_padding() + self.get_total_bytes_from_phrase_data(),
            PayloadType::Animation(_,_) => {
                self.get_padding() + self.get_total_bytes_from_phrase_data()
            }
        };

        out[0] = 0x00;
        out[1] = whole_packet_size.shr(8) as u8;
        out[2] = (whole_packet_size & 0xff) as u8;
        out[3] = idx.shr(8) as u8;
        out[4] = (idx & 0xff) as u8;
        out[5] = if idx == 0 {
            self.make_first_packet(&mut out[6..6 + 128]);
            0x80
        } else {
            self.make_n_packet(idx, &mut out[6..6 + 128]) as u8
        };
        out[out[5] as usize + 6] = calculate_checksum(&out[..out[5] as usize + 0x6]);
        (out[5] + 8).into()
    }

    pub fn generate_packet(&mut self, idx: usize, out: &mut [u8]) -> usize {
        out[0] = 0x01;

        let bytes_wrote = self.make_subpacket(idx, &mut out[4..]);
        out[1] = bytes_wrote.shr(8) as u8;
        out[2] = (bytes_wrote & 0xff) as u8;
        //Content type
        out[3] = self.payload.get_content_type();

        let current_bytes_wrote = bytes_wrote;

        let last_bytes_wrote = escape_byets_in_place(&mut out[3..], current_bytes_wrote);
        out[last_bytes_wrote + 3] = 0x03;
        last_bytes_wrote + 4
    }
}

#[cfg(all(test, not(feature = "custom_charset")))]
mod test {
    use super::*;
    extern crate alloc;
    extern crate std;
    use alloc::vec;

    #[test]
    fn testing_hardcoded_packet_result() {
        const PHRASE: &str = "Testing";
        let colors: [CoolLEDColors; PHRASE.len()] = [CoolLEDColors::Red; PHRASE.len()];

        let expected = vec![
            vec![
                1, 0, 136, 2, 6, 0, 2, 5, 175, 0, 0, 128, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 2, 5, 68, 32, 0, 32, 0, 32, 0, 63, 252, 32, 0, 32, 0, 32, 0,
                0, 0, 0, 248, 2, 5, 68, 2, 6, 23, 3,
            ],
            vec![
                1, 0, 136, 2, 6, 0, 2, 5, 175, 0, 2, 5, 128, 68, 2, 6, 68, 2, 6, 68, 2, 5, 68, 0,
                200, 0, 0, 2, 5, 136, 2, 6, 68, 2, 6, 68, 2, 6, 36, 2, 6, 36, 2, 6, 36, 2, 5, 24,
                0, 0, 2, 6, 0, 2, 6, 0, 63, 248, 2, 6, 4, 2, 6, 4, 2, 6, 4, 0, 8, 0, 0, 2, 6, 4, 2,
                6, 4, 51, 252, 0, 4, 0, 4, 0, 0, 2, 6, 0, 2, 5, 252, 2, 6, 0, 2, 6, 0, 2, 6, 0, 2,
                6, 0, 2, 5, 252, 0, 0, 2, 5, 226, 2, 6, 17, 2, 6, 17, 2, 6, 17, 2, 6, 17, 2, 5,
                254, 2, 6, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 74, 3,
            ],
            vec![
                1, 0, 136, 2, 6, 0, 2, 5, 175, 0, 2, 6, 128, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 44, 3,
            ],
            vec![
                1, 0, 55, 2, 6, 0, 2, 5, 175, 0, 2, 7, 47, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 130, 3,
            ],
        ];

        let mut c = CoolLEDWriter::new(PayloadType::Text(&PHRASE, &colors));
        let mut buffer: [u8; 255] = [0; 255];
        let quantity_packets = c.get_packets_count();
        std::println!("packets = {}", quantity_packets);
        let mut count = 0;
        let mut total_written = 0;
        for idx in 0..quantity_packets {
            let wrote = c.generate_packet(idx, &mut buffer);
            let packet_data = &buffer[..wrote];

            assert_eq!(&expected[idx], packet_data);
            std::println!(
                "packet size :{}/{} - {} bytes - data {:?}",
                count + 1,
                quantity_packets,
                packet_data.len(),
                packet_data,
            );
            total_written += packet_data.len();
            count += 1;
            buffer.fill(0);
        }
        std::println!("|total : {}", total_written);
    }
}
