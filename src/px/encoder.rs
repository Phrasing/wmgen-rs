use base64::{engine::general_purpose::STANDARD, Engine};

pub fn compute_ob_xor_key(tag: &str) -> u8 {
    let mut e: i64 = 0;
    for ch in tag.chars() {
        e = (31 * e + ch as i64) % 2_147_483_647;
    }
    (((e % 900) + 100) % 128) as u8
}

pub fn xor_bytes(data: &[u8], key: u8) -> Vec<u8> {
    data.iter().map(|b| b ^ key).collect()
}

pub fn encode_sensor(json: &str, uuid: &str, sts: &str) -> String {
    let xored = xor_bytes(json.as_bytes(), 50);
    let b64 = STANDARD.encode(&xored);

    let sts_b64 = STANDARD.encode(sts.as_bytes());
    let key = xor_bytes(sts_b64.as_bytes(), 10);

    let indices = compute_indices(&key, b64.len(), uuid);

    let b64_bytes = b64.as_bytes();
    let mut result: Vec<u8> = Vec::with_capacity(b64_bytes.len() + key.len());
    let mut offset = 0usize;
    for (i, k) in key.iter().enumerate() {
        let end = if indices[i] > i {
            indices[i] - i - 1
        } else {
            0
        };
        let end_clamped = end.min(b64_bytes.len());
        if end_clamped > offset {
            result.extend_from_slice(&b64_bytes[offset..end_clamped]);
        }
        result.push(*k);
        offset = end_clamped;
    }
    result.extend_from_slice(&b64_bytes[offset..]);

    String::from_utf8_lossy(&result).into_owned()
}

pub fn compute_indices(key: &[u8], payload_len: usize, uuid: &str) -> Vec<usize> {
    let uuid_b64 = STANDARD.encode(uuid.as_bytes());
    let r = xor_bytes(uuid_b64.as_bytes(), 10);
    let r_len = r.len();
    if r_len == 0 || key.is_empty() {
        return vec![];
    }

    let mut max_val: usize = 0;
    for i in 0..key.len() {
        let row = i / r_len + 1;
        let col = i % r_len;
        let row_idx = row.min(r_len - 1);
        let product = r[col] as usize * r[row_idx] as usize;
        if product > max_val {
            max_val = product;
        }
    }

    let mut positions: Vec<usize> = Vec::with_capacity(key.len());
    for i in 0..key.len() {
        let row = i / r_len + 1;
        let col = i % r_len;
        let row_idx = row.min(r_len - 1);
        let raw_pos = r[col] as usize * r[row_idx] as usize;

        let mut pos = if raw_pos >= payload_len {
            linear_map(raw_pos, 0, max_val, 0, payload_len.saturating_sub(1))
        } else {
            raw_pos
        };

        while positions.contains(&pos) {
            pos += 1;
            if pos >= payload_len {
                pos = 0;
            }
        }
        positions.push(pos);
    }

    let mut sorted = positions;
    sorted.sort_unstable();
    sorted
}

fn linear_map(value: usize, in_min: usize, in_max: usize, out_min: usize, out_max: usize) -> usize {
    if in_max == in_min {
        return out_min;
    }
    let ratio = (value - in_min) as f64 / (in_max - in_min) as f64;
    ((ratio * (out_max - out_min) as f64) + out_min as f64).floor() as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ob_xor_key_walmart_tag() {
        let key = compute_ob_xor_key("eW5CaD8AUB99Zg==");
        assert_eq!(key, 41, "OB XOR key for current Walmart tag");
    }

    #[test]
    fn sensor_encode_decode_roundtrip() {
        let json = r#"{"m":{"appId":"PXu6b0qd2S","seq":0},"p":[]}"#;
        let uuid = "485bcdc0-f437-11f0-a98f-27cdd6cede4c";
        let sts = "1700000000000";

        let encoded = encode_sensor(json, uuid, sts);
        assert!(!encoded.is_empty());

        let sts_b64 = base64::engine::general_purpose::STANDARD.encode(sts.as_bytes());
        let key = xor_bytes(sts_b64.as_bytes(), 10);
        let key_len = key.len();

        let chars: Vec<u8> = encoded.as_bytes().to_vec();
        let expected_payload_len = chars.len() - key_len;
        let indices = compute_indices(&key, expected_payload_len, uuid);

        let mut remove: Vec<usize> = indices
            .iter()
            .map(|&idx| if idx > 0 { idx - 1 } else { 0 })
            .collect();
        remove.sort_unstable_by(|a, b| b.cmp(a));
        let mut stripped = chars.clone();
        for pos in remove {
            if pos < stripped.len() {
                stripped.remove(pos);
            }
        }
        let b64 = String::from_utf8_lossy(&stripped);
        use base64::Engine;
        let xored = base64::engine::general_purpose::STANDARD
            .decode(b64.as_ref())
            .expect("valid base64");
        let decoded = xor_bytes(&xored, 50);
        assert_eq!(String::from_utf8_lossy(&decoded), json);
    }
}
