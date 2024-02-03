use crate::util::escpae_bytes;

const PREFIX_PACKET: [u8; 2] = [0x01, 0x00];
const PACKET_INIT1: [u8; 3] = [0x32, 0x04, 0x00];
const PACKET_INIT2: [u8; 2] = [0x23, 0x01];
const PACKET_INIT3: [u8; 3] = [0x32, 0x02, 0x00];
const PACKET_INIT4: [u8; 2] = [0x34, 0x00];
const PACKET_INIT5: [u8; 2] = [0x30, 0x00];
const PACKET_INIT6: [u8; 2] = [0x31, 0x00];
const PACKET_INIT7: [u8; 3] = [0x32, 0x01, 0x00];

/// Options of the command to turn on and off the display
pub enum AppStatus {
    On,
    Off,
}

/// Types of packet supported on the device. We only use Text,Draw, Animate, Mode, Speed, Bright and Switch
pub enum PacketType {
    Music = 1,
    Text = 2,
    Draw = 3,
    Animate = 4,
    Icon = 5,
    Mode = 6,
    Speed = 7,
    Bright = 8,
    Switch = 9,
    Xfer = 10,
}

/// Features of effects to be sent to the display
pub enum EffectsMode {
    Static = 0x1,
    Left = 0x2,
    Right = 0x3,
    Up = 0x4,
    Down = 0x5,
    Snowflake = 0x6,
    Picture = 0x7,
    Lase = 0x8,
}

/// Initialization packets to be sent when the device is connected
/// via UART. When connected via bluetooth
/// it already does the initialization process
pub fn get_init_packets<F: FnMut(u8)>(mut func: F) {
    write_packet(&mut func, &PACKET_INIT1);
    write_packet(&mut func, &PACKET_INIT2);
    write_packet(&mut func, &PACKET_INIT3);
    write_packet(&mut func, &PACKET_INIT4);
    write_packet(&mut func, &PACKET_INIT5);
    write_packet(&mut func, &PACKET_INIT6);
    write_packet(&mut func, &PACKET_INIT7);
}

fn write_packet<F: FnMut(u8)>(mut func: F, data: &[u8]) {
    let _ = &PREFIX_PACKET.map(&mut func);

    //STX
    func(0x02);

    escpae_bytes(&mut func, data.len() as u8);

    for byte in data.iter() {
        escpae_bytes(&mut func, *byte);
    }

    //ETX
    func(0x03);
}

/// Write a packet to change the brightness
pub fn write_bright<F: FnMut(u8)>(mut func: F, bright: u8) {
    let _ = &PREFIX_PACKET.map(&mut func);
    //STX
    func(0x02);
    func(0x06);

    func(PacketType::Bright as u8);

    let bright_level = if bright < 0x10 { 0x10 } else { bright };
    escpae_bytes(&mut func, bright_level);

    //ETX
    func(0x03);
}

/// Write a packet to chance the speed of the text
pub fn write_speed<F: FnMut(u8)>(mut func: F, speed: u8) {
    let _ = &PREFIX_PACKET.map(&mut func);
    //STX
    func(0x02);
    func(0x06);

    func(PacketType::Speed as u8);

    let speed_level = if speed < 0x10 { 0x10 } else { speed };

    escpae_bytes(&mut func, speed_level);

    //ETX
    func(0x03);
}

/// Write a packet to change the effect on the text
pub fn write_mode_led<F: FnMut(u8)>(mut func: F, mode: EffectsMode) {
    let _ = &PREFIX_PACKET.map(&mut func);
    //STX
    func(0x02);
    func(0x06);

    func(PacketType::Mode as u8);

    escpae_bytes(&mut func, mode as u8);

    //ETX
    func(0x03);
}

/// Write a packet to turn on and off the device
pub fn write_app_status<F: FnMut(u8)>(mut func: F, status: AppStatus) {
    let _ = &PREFIX_PACKET.map(&mut func);
    //STX
    func(0x02);
    func(0x06);

    func(PacketType::Switch as u8);

    let data = match status {
        AppStatus::On => 0x1u8,
        AppStatus::Off => 0x0,
    };

    escpae_bytes(&mut func, data);

    //ETX
    func(0x03);
}

#[cfg(test)]
mod test {
    use super::*;
    extern crate alloc;
    extern crate std;
    use alloc::vec;

    #[test]
    fn write_static() {
        let mut buff = vec![];

        let mut write_buff = |data| {
            buff.push(data);
        };

        write_mode_led(&mut write_buff, EffectsMode::Static);
        assert_eq!(buff, vec![0x01, 0x00, 0x02, 0x06, 0x06, 0x02, 0x05, 0x03]);
    }
    #[test]
    fn write_up() {
        let mut buff = vec![];

        let mut write_buff = |data| {
            buff.push(data);
        };

        write_mode_led(&mut write_buff, EffectsMode::Up);
        assert_eq!(buff, vec![0x01, 0x00, 0x02, 0x06, 0x06, 0x04, 0x03]);
    }

    #[test]
    fn write_bright_min() {
        let mut buff = vec![];

        let write_buff = |data| {
            buff.push(data);
        };
        write_bright(write_buff, 0x10);
        assert_eq!(buff, vec![0x01, 0x00, 0x02, 0x06, 0x08, 0x10, 0x03]);
    }

    #[test]
    fn write_speed_min() {
        let mut buff = vec![];

        let write_buff = |data| {
            buff.push(data);
        };
        write_speed(write_buff, 0x10);
        assert_eq!(buff, vec![0x01, 0x00, 0x02, 0x06, 0x07, 0x10, 0x03]);
    }

    #[test]
    fn write_app_off() {
        let mut buff = vec![];

        let write_buff = |data| {
            buff.push(data);
        };
        write_app_status(write_buff, AppStatus::Off);
        assert_eq!(buff, vec![0x01, 0x00, 0x02, 0x06, 0x09, 0x00, 0x03]);
    }

    #[test]
    fn write_app_on() {
        let mut buff = vec![];

        let write_buff = |data| {
            buff.push(data);
        };
        write_app_status(write_buff, AppStatus::On);
        assert_eq!(buff, vec![0x01, 0x00, 0x02, 0x06, 0x09, 0x02, 0x05, 0x03]);
    }
}
