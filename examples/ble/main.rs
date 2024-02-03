use anyhow::Result;
use btleplug::api::{Central, Manager as _, Peripheral as _, ScanFilter, WriteType};
use btleplug::platform::{Adapter, Manager, Peripheral};
use clap::{command, Parser, ValueEnum};
use coolled1248::colors::CoolLEDColors;
use coolled1248::coolled::CoolLEDWriter;
use coolled1248::coolled::PayloadType;
use coolled1248::packets::write_mode_led;
use coolled1248::packets::EffectsMode;
use image::{GenericImageView, Pixel};
use log::info;
use std::error::Error;
use std::time::Duration;
use tokio::time;
use uuid::{uuid, Uuid};

const COOLLEDX_CHARACTERISTIC_UUID: Uuid = uuid!("0000fff1-0000-1000-8000-00805f9b34fb");

async fn find_coolledx(central: &Adapter) -> Option<Peripheral> {
    for p in central.peripherals().await.unwrap() {
        if p.properties()
            .await
            .unwrap()
            .unwrap()
            .local_name
            .iter()
            .any(|name| name.contains("CoolLEDX"))
        {
            return Some(p);
        }
    }
    None
}

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
    #[arg(value_enum)]
    mode: Option<Mode>,

    #[arg(short, long, help = "Send the initialization packets to the led")]
    init: bool,

    #[arg(long, default_value_t = 64)]
    width: usize,

    #[arg(long, default_value_t = 16)]
    height: usize,

    #[arg(long)]
    image: Option<String>,

    #[arg(long)]
    adjust_bright: Option<u8>,
}

fn get_diff_color(cola: i32, colb: i32) -> i32 {
    let (ar, ag, ab) = ((cola >> 16) & 0xff, (cola >> 8) & 0xff, cola & 0xff);
    let (br, bg, bb) = ((colb >> 16) & 0xff, (colb >> 8) & 0xff, colb & 0xff);

    let sum = (ar - br).pow(2) + (ag - bg).pow(2) + (ab - bb).pow(2);

    (sum as f32).sqrt() as i32
}

const VALID_COLORS: [i32; 7] = [
    0xffffff, //white
    0xff0000, //red
    0x00ff00, //green
    0x0000ff, //blue
    0xffff00, //yellow
    0x00ffff, //cyan
    0xff00ff, //pink
];

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
            let mut is_default_color = false;
            let px = if y < image_height && x < image_witdh {
                img.get_pixel(x as u32, y as u32).to_rgb()
            } else {
                is_default_color = true;
                default_color
            };

            let color_channels = px.channels();

            let (tmp_red, tmp_green, tmp_blue) = if is_default_color {
                tmp_red = (tmp_red << 1) + (color_channels[0] / 255);
                tmp_green = (tmp_green << 1) + (color_channels[1] / 255);
                tmp_blue = (tmp_blue << 1) + (color_channels[2] / 255);
                (tmp_red, tmp_green, tmp_blue)
            } else {
                let mut diffs = vec![];
                for vc in VALID_COLORS {
                    let red = color_channels[0] as i32;
                    let green = color_channels[1] as i32;
                    let blue = color_channels[2] as i32;
                    let cc = (red.wrapping_shl(16) + green.wrapping_shl(8) + blue).into();
                    diffs.push(get_diff_color(vc, cc));
                }

                if let Some(min) = diffs.iter().min() {
                    if let Some(pos_min) = diffs.iter().position(|x| x == min) {
                        let closest_clor = VALID_COLORS[pos_min];
                        let ccr = (closest_clor >> 16) & 0xff;
                        let ccg = (closest_clor >> 8) & 0xff;
                        let ccb = closest_clor & 0xff;
                        tmp_red = (tmp_red << 1) + (ccr / 255) as u8;
                        tmp_green = (tmp_green << 1) + (ccg / 255) as u8;
                        tmp_blue = (tmp_blue << 1) + (ccb / 255) as u8;
                    }
                }

                (tmp_red, tmp_green, tmp_blue)
            };

            if y % 8 == 7 {
                red.push(tmp_red as u8);
                green.push(tmp_green as u8);
                blue.push(tmp_blue as u8);
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

    let (image_witdh, image_height) = image::GenericImageView::dimensions(&img);

    println!("witdh : {} height : {}", &image_witdh, &image_height);

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
            let mut is_default_color = false;
            let px = if y < image_height && x < image_witdh {
                img.get_pixel(x as u32, y as u32).to_rgb()
            } else {
                is_default_color = true;
                default_color
            };

            let color_channels = px.channels();

            let (tmp_red, tmp_green, tmp_blue) = if is_default_color {
                tmp_red = (tmp_red << 1) + (color_channels[0] / 255);
                tmp_green = (tmp_green << 1) + (color_channels[1] / 255);
                tmp_blue = (tmp_blue << 1) + (color_channels[2] / 255);
                (tmp_red, tmp_green, tmp_blue)
            } else {
                let mut diffs = vec![];

                for vc in VALID_COLORS {
                    let red = color_channels[0] as i32;
                    let green = color_channels[1] as i32;
                    let blue = color_channels[2] as i32;
                    let rgb_i32 = (red.wrapping_shl(16) + green.wrapping_shl(8) + blue).into();
                    diffs.push(get_diff_color(vc, rgb_i32));
                }

                if let Some(min) = diffs.iter().min() {
                    if let Some(pos_min) = diffs.iter().position(|x| x == min) {
                        let closest_color = VALID_COLORS[pos_min];

                        let closest_color_red = (closest_color >> 16) & 0xff;
                        let closest_color_green = (closest_color >> 8) & 0xff;
                        let closest_color_blue = closest_color & 0xff;

                        tmp_red = (tmp_red << 1) + (closest_color_red / 255) as u8;
                        tmp_green = (tmp_green << 1) + (closest_color_green / 255) as u8;
                        tmp_blue = (tmp_blue << 1) + (closest_color_blue / 255) as u8;
                    }
                }

                (tmp_red, tmp_green, tmp_blue)
            };

            if y % 8 == 7 {
                red.push(tmp_red as u8);
                green.push(tmp_green as u8);
                blue.push(tmp_blue as u8);
            }
        }
    }

    (red, green, blue)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();
    let cli = Cli::parse();

    let manager = Manager::new().await?;

    let central = manager
        .adapters()
        .await
        .expect("Unable to fetch adapter list.")
        .into_iter()
        .nth(0)
        .expect("Unable to find adapters.");

    central.start_scan(ScanFilter::default()).await?;
    time::sleep(Duration::from_secs(3)).await;

    let coolledx = find_coolledx(&central)
        .await
        .expect("Couldn't find CoolLEDX");

    coolledx.connect().await?;

    // discover services and characteristics
    coolledx.discover_services().await?;

    // find the characteristic we want
    let chars = coolledx.characteristics();
    let cmd_char = chars
        .iter()
        .find(|c| c.uuid == COOLLEDX_CHARACTERISTIC_UUID)
        .expect("Unable to find characterics");

    let pixels = cli.width * cli.height;
    let pixels_bytes = pixels / 8;
    const COLOR_CHANNELS: usize = 3;
    const PHRASE: &str = "Testing";
    let colors: [CoolLEDColors; PHRASE.len()] = [CoolLEDColors::Red; PHRASE.len()];

    if let Some(mode) = cli.mode {
        let mut led_writer = match mode {
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
                let (red, green, blue, frames) =
                    get_channel_from_gif(&image, cli.width, cli.height);
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
                let led_writer = CoolLEDWriter::new(PayloadType::Image(color_data));
                led_writer
            }
        };

        let mut buffer: [u8; 255] = [0; 255];
        let quantity_packets = led_writer.get_packets_count();
        info!("packets = {}", quantity_packets);

        for idx in 0..quantity_packets {
            let wrote = led_writer.generate_packet(idx, &mut buffer);
            let packet_data = &buffer[..wrote];
            let iter = packet_data.chunks(20);

            for chk in iter {
                coolledx
                    .write(&cmd_char, &chk, WriteType::WithoutResponse)
                    .await?;
                time::sleep(Duration::from_millis(200)).await;
            }
            buffer.fill(0);
        }
    }

    if let Some(_b) = cli.adjust_bright {
        let mut buff = vec![];
        let write_buff = |data| {
            buff.push(data);
        };

        write_mode_led(write_buff, EffectsMode::Left);

        println!("writing : {:X?}", &buff);

        coolledx
            .write(&cmd_char, buff.as_slice(), WriteType::WithoutResponse)
            .await?;

        return Ok(());
    }

    Ok(())
}
