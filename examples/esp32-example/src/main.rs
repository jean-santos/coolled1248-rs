use coolled1248::{colors::CoolLEDColors,packets::get_init_packets,coolled::{CoolLEDWriter,PayloadType}};
use esp_idf_hal::{gpio,prelude::Peripherals};
use log::info;
use core::time::Duration;

const PHRASE: &str = "Testing 123";
const COLORS: [CoolLEDColors; PHRASE.len()] = [CoolLEDColors::Red; PHRASE.len()];

fn main(){
    let mut led_writer = CoolLEDWriter::new(PayloadType::Text(PHRASE, &COLORS));

    esp_idf_sys::link_patches();
    let peripherals = Peripherals::take().expect("failed to take peripherals");

    let uart = {
        let tx = peripherals.pins.gpio14;
        let rx = peripherals.pins.gpio15;

        let config =
            esp_idf_hal::uart::config::Config::new().baudrate(esp_idf_hal::units::Hertz(38400));
        esp_idf_hal::uart::UartDriver::new(
            peripherals.uart2,
            tx,
            rx,
            Option::<gpio::Gpio0>::None,
            Option::<gpio::Gpio1>::None,
            &config,
        )
    }.unwrap();

    get_init_packets(|data| {
        let _ = uart.write(&[data]);
    });

    let mut buffer: [u8; 255] = [0; 255];
    let quantity_packets = led_writer.get_packets_count();
    info!("Quantity of packets = {quantity_packets}");

    (0..quantity_packets).map(|idx|{
        let wrote = led_writer.generate_packet(idx, &mut buffer);
        let packet_data = &buffer[..wrote];

        match uart.write(packet_data) {
            Ok(_) => {
                info!(
                    "packet size :{}/{} - {} bytes - data {:X?}",
                    idx + 1,
                    quantity_packets,
                    packet_data.len(),
                    packet_data,
                );
            }
            Err(e) => eprintln!("{:?}", e),
        }
        std::thread::sleep(Duration::from_millis(100));
        buffer.fill(0);
    }).for_each(drop);
}