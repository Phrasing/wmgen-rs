use rand::Rng;
use std::time::SystemTime;

pub fn uuid_v1() -> String {
    let mut rng = rand::thread_rng();

    let now_ms = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);

    let uuid_epoch_ms: i64 = 12_219_292_800_000;
    let adjusted = now_ms + uuid_epoch_ms;

    let time_low = ((10000i64 * (adjusted & 0x0FFF_FFFF) + 1) % 4_294_967_296) as u32;

    let time_mid_hi = ((adjusted as f64) / 4_294_967_296.0 * 10000.0) as i64 & 0x0FFF_FFFF;

    let mut bytes = [0u8; 16];

    bytes[0] = (time_low >> 24) as u8;
    bytes[1] = (time_low >> 16) as u8;
    bytes[2] = (time_low >> 8) as u8;
    bytes[3] = time_low as u8;

    bytes[4] = (time_mid_hi >> 8) as u8;
    bytes[5] = time_mid_hi as u8;

    bytes[6] = ((time_mid_hi >> 24) as u8 & 0x0F) | 0x10;
    bytes[7] = (time_mid_hi >> 16) as u8;

    let clock_seq: u16 = rng.gen::<u16>() & 0x3FFF;
    bytes[8] = ((clock_seq >> 8) as u8) | 0x80;
    bytes[9] = clock_seq as u8;

    let node: [u8; 6] = rng.gen();
    bytes[10] = node[0] | 0x01;
    bytes[11] = node[1];
    bytes[12] = node[2];
    bytes[13] = node[3];
    bytes[14] = node[4];
    bytes[15] = node[5];

    format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        bytes[0], bytes[1], bytes[2], bytes[3],
        bytes[4], bytes[5],
        bytes[6], bytes[7],
        bytes[8], bytes[9],
        bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15],
    )
}
