use std::io::Write;
use std::time::Duration;

use anyhow::Result;
use clap::{command, Parser, ValueEnum};
use coolled1248::colors::CoolLEDColors;
use coolled1248::coolled::{CoolLEDWriter, PayloadType};
use coolled1248::packets::get_init_packets;
use image::Pixel;
use log::*;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
enum Mode {
    Text,
    Animation,
    AnimationTest,
    Image,
    ImageTest,
}

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[arg(short, long, help = "Serial port to comunicate with the")]
    port: String,

    #[arg(value_enum)]
    mode: Mode,

    #[arg(short, long, help = "Send the initialization packets to the led")]
    init: bool,

    #[arg(long, default_value_t = 32)]
    width: usize,

    #[arg(long, default_value_t = 16)]
    height: usize,

    #[arg(long)]
    image: Option<String>,
}

fn get_channel_from_gif(
    filename: &str,
    width: usize,
    height: usize,
) -> (Vec<u8>, Vec<u8>, Vec<u8>, usize) {
    use image::buffer::ConvertBuffer;
    use image::codecs::gif::GifDecoder;
    use image::AnimationDecoder;
    use std::fs::File;

    let mut red = vec![];
    let mut green = vec![];
    let mut blue = vec![];

    let file_in = File::open(filename).unwrap();
    let decoder = GifDecoder::new(file_in).unwrap();
    let frames = decoder.into_frames();
    let frames = frames
        .map(|f| f.unwrap().buffer().convert())
        .collect::<Vec<image::RgbImage>>();

    for frame in &frames {
        let (mut frame_red, mut frame_green, mut frame_blue) = get_bytes(&frame, width, height);
        red.append(&mut frame_red);
        green.append(&mut frame_green);
        blue.append(&mut frame_blue);
    }

    let frames_number = frames.len();
    println!("frames qt {frames_number}");
    (red, green, blue, frames_number)
}

fn get_bytes(img: &image::RgbImage, width: usize, height: usize) -> (Vec<u8>, Vec<u8>, Vec<u8>) {
    let (image_witdh, image_height) = image::GenericImageView::dimensions(img);
    let (image_witdh, image_height) = (image_witdh as usize, image_height as usize);

    let default_color: image::Rgb<u8> = image::Rgb([0, 0, 0]);
    let mut red = vec![];
    let mut green = vec![];
    let mut blue = vec![];

    let mut tmp_red = 0;
    let mut tmp_green = 0;
    let mut tmp_blue = 0;
    for x in 0..width {
        for y in 0..height {
            let px = if y < image_height && x < image_witdh {
                img.get_pixel(x as u32, y as u32).to_rgb()
            } else {
                default_color
            };

            let color_channels = px.channels();

            tmp_red = (tmp_red << 1) + (color_channels[0] / 255);
            tmp_green = (tmp_green << 1) + (color_channels[1] / 255);
            tmp_blue = (tmp_blue << 1) + (color_channels[2] / 255);

            if y % 8 == 7 {
                red.push(tmp_red);
                green.push(tmp_green);
                blue.push(tmp_blue);
            }
        }
    }

    (red, green, blue)
}

fn get_channels_from_image(
    filename: &str,
    width: usize,
    height: usize,
) -> (Vec<u8>, Vec<u8>, Vec<u8>) {
    let img = image::open(&filename).unwrap();
    get_bytes(img.as_rgb8().unwrap(), width, height)
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    env_logger::init();

    let mut serial_port = serialport::new("/dev/ttyUSB0", 38400).open().expect("err");

    if cli.init {
        get_init_packets(|data: u8| {
            let _ = serial_port.write(&[data]);
        });
    }

    let pixels = cli.width * cli.height;
    let pixels_bytes = pixels / 8;
    const COLOR_CHANNELS: usize = 3;
    const PHRASE: &str = "Testing";
    let colors: [CoolLEDColors; PHRASE.len()] = [CoolLEDColors::Red; PHRASE.len()];

    let mut led_writer = match cli.mode {
        Mode::Text => CoolLEDWriter::new(PayloadType::Text(&PHRASE, &colors)),
        Mode::AnimationTest => {
            const FRAMES: usize = 3;

            let mut animation_data = vec![0; pixels_bytes * FRAMES * COLOR_CHANNELS];
            //first frame
            animation_data[0..pixels_bytes].fill(0xff);

            //second frmae
            animation_data[(pixels_bytes * 4)..pixels_bytes * 5].fill(0xff);

            //third frame
            animation_data[(pixels_bytes * 8)..].fill(0xff);

            let animation_data = animation_data.to_vec().leak();

            CoolLEDWriter::new(PayloadType::Animation(animation_data, 3))
        }
        Mode::ImageTest => {
            let mut color_data = vec![0; pixels_bytes * 3];
            color_data[64..128].fill(0xff);

            let color_data = color_data.to_vec().leak();

            CoolLEDWriter::new(PayloadType::Image(color_data))
        }
        Mode::Animation => {
            let image = cli.image.unwrap();
            let (red, green, blue, frames) = get_channel_from_gif(&image, cli.width, cli.height);
            let animation_data: Vec<u8> = red
                .into_iter()
                .chain(green.into_iter())
                .chain(blue.into_iter())
                .collect();
            let animation_data = Box::new(animation_data).leak();
            CoolLEDWriter::new(PayloadType::Animation(animation_data, frames))
        }
        Mode::Image => {
            let image = cli.image.unwrap();
            let (red, green, blue) = get_channels_from_image(&image, cli.width, cli.height);

            let color_data: Vec<u8> = red
                .into_iter()
                .chain(green.into_iter())
                .chain(blue.into_iter())
                .collect();
            let color_data = Box::new(color_data).leak();
            CoolLEDWriter::new(PayloadType::Image(color_data))
        }
    };

    let mut buffer: [u8; 255] = [0; 255];
    let quantity_packets = led_writer.get_packets_count();
    info!("packets = {}", quantity_packets);
    let mut count = 0;
    let mut total_written = 0;
    for idx in 0..quantity_packets {
        let wrote = led_writer.generate_packet(idx, &mut buffer);
        let packet_data = &buffer[..wrote];

        match serial_port.write(packet_data) {
            Ok(w) => {
                info!(
                    "packet size :{}/{} - {} bytes - data {:X?}",
                    count + 1,
                    quantity_packets,
                    packet_data.len(),
                    packet_data,
                );
                total_written += w;
            }
            Err(e) => eprintln!("{:?}", e),
        }
        std::thread::sleep(Duration::from_millis(100));

        count += 1;
        buffer.fill(0);
    }

    info!("total : {}", total_written);

    Ok(())
}
