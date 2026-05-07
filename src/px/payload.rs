use crate::px::encoder::encode_sensor;
use crate::px::fingerprint::{build_seq0_json, build_seq1_json, Seq1JsonInput};
use rand::Rng;
use std::time::SystemTime;

const BI_VALUE: &str = "Rl19VhNhNHcAAhNzXwpPdUd/YRFXUzxySkpqG30bP15FeTFvAhU+bCgNAS1HeHgPRWw5aG5eMDUvCSYfUjUkeFtLcC1TCUB7VXwwDgNIaX9KQjR+RswXEgmMGtNBHo=";

fn now_ms_str() -> String {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_millis().to_string())
        .unwrap_or_else(|_| "1777000000000".to_string())
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(1777000000000)
}

pub fn build_seq0_body(
    uuid: &str,
    sts: &str,
    tag: &str,
    page_url: &str,
) -> (Vec<(String, String)>, u64) {
    let sts_u64: u64 = sts.parse().unwrap_or_else(|_| now_ms());

    let perf_now_0: u64 = rand::thread_rng().gen_range(200u64..500);

    let payload_json = build_seq0_json(page_url, uuid, sts_u64, perf_now_0);
    let payload = encode_sensor(&payload_json, uuid, sts);

    let ft = (now_ms() % 10000) + 300;

    let fields = vec![
        ("payload".into(), payload),
        ("appId".into(), "PXu6b0qd2S".into()),
        ("tag".into(), tag.to_string()),
        ("uuid".into(), uuid.to_string()),
        ("ft".into(), ft.to_string()),
        ("seq".into(), "0".into()),
        ("en".into(), "NTA".into()),
        ("bi".into(), BI_VALUE.to_string()),
        ("pc".into(), rand_pc()),
        ("rsc".into(), "1".into()),
    ];

    (fields, perf_now_0)
}

pub struct Seq1BodyInput<'a> {
    pub uuid: &'a str,
    pub sts: &'a str,
    pub tag: &'a str,
    pub page_url: &'a str,
    pub cs: &'a str,
    pub vid: &'a str,
    pub sid: &'a str,
    pub pxhd: &'a str,
    pub cts: &'a str,
    pub server_ts: u64,
    pub token: &'a str,
    pub ob0_recv_ts: u64,
    pub seq0_perf_now: u64,
}

pub fn build_seq1_body(input: Seq1BodyInput<'_>) -> Vec<(String, String)> {
    let Seq1BodyInput {
        uuid,
        sts,
        tag,
        page_url,
        cs,
        vid,
        sid,
        pxhd,
        cts,
        server_ts,
        token,
        ob0_recv_ts,
        seq0_perf_now,
    } = input;
    let sts1 = now_ms_str();
    let sts1_u64: u64 = sts1.parse().unwrap_or_else(|_| now_ms());

    let init_ts: u64 = sts.parse().unwrap_or(sts1_u64);

    let payload_json = build_seq1_json(Seq1JsonInput {
        page_url,
        uuid,
        sts: init_ts,
        server_ts,
        token,
        ob0_recv_ts,
        seq0_perf_now,
        pxhd: if pxhd.is_empty() { None } else { Some(pxhd) },
    });
    let payload = encode_sensor(&payload_json, uuid, &sts1);

    let ft = (now_ms() % 10000) + 300;

    let mut fields = vec![
        ("payload".into(), payload),
        ("appId".into(), "PXu6b0qd2S".into()),
        ("tag".into(), tag.to_string()),
        ("uuid".into(), uuid.to_string()),
        ("ft".into(), ft.to_string()),
        ("seq".into(), "1".into()),
        ("en".into(), "NTA".into()),
        ("bi".into(), BI_VALUE.to_string()),
        ("pc".into(), rand_pc()),
        ("rsc".into(), "2".into()),
    ];

    if !cs.is_empty() {
        fields.push(("cs".into(), cs.to_string()));
    }
    if !sid.is_empty() {
        fields.push(("sid".into(), sid.to_string()));
    }
    if !vid.is_empty() {
        fields.push(("vid".into(), vid.to_string()));
    }
    if !pxhd.is_empty() {
        fields.push(("pxhd".into(), pxhd.to_string()));
    }
    if !cts.is_empty() {
        fields.push(("cts".into(), cts.to_string()));
    }

    fields
}

fn rand_pc() -> String {
    let n: u64 = rand::thread_rng().gen_range(1_000_000_000_000_000..9_999_999_999_999_999);
    n.to_string()
}
