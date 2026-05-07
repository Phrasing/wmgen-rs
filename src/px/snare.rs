use crate::USER_AGENT;
use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use base64::{engine::general_purpose::STANDARD, Engine};
use chrono::{Datelike, FixedOffset, Timelike, Utc};
use rand::Rng;
use serde_json::{json, Value};
use std::time::SystemTime;

const OBS_KEY: &[u8; 32] = b"abC3UuT0Yte5FBGN2F6cQu0pegMgCMpr";
const OBS_PREFIX: &str = "KAUHEVKF";
const OBS_TOKEN: &str = "elh9ie";

const CHROME_FULL_VERSION: &str = "147.0.7727.138";

const CANVAS_HASH_B: &str = "c09900f7285ee8cb5a8bad134c65a644000dbda64d3b83583786a907ca4e0030";
const CANVAS_HASH_C: &str = "72de186a658767c00798634a57cf7ed098749c18f05e9862dd669ee4d2798656";

const WEBGL_VENDOR: &str = "Google Inc. (NVIDIA)";
const WEBGL_RENDERER: &str =
    "ANGLE (NVIDIA, NVIDIA GeForce RTX 5080 (0x00002C02) Direct3D11 vs_5_0 ps_5_0, D3D11)";

pub fn encrypt_obs(json: &str) -> anyhow::Result<String> {
    let mut rng = rand::thread_rng();
    let nonce_bytes: [u8; 12] = rng.gen();
    let nonce = Nonce::from_slice(&nonce_bytes);

    let cipher =
        Aes256Gcm::new_from_slice(OBS_KEY).map_err(|e| anyhow::anyhow!("AES key error: {e}"))?;
    let ciphertext = cipher
        .encrypt(nonce, json.as_bytes())
        .map_err(|e| anyhow::anyhow!("AES-GCM encrypt: {e}"))?;

    let mut payload = nonce_bytes.to_vec();
    payload.extend_from_slice(&ciphertext);

    Ok(format!("{}{}", OBS_PREFIX, STANDARD.encode(&payload)))
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(1777000000000)
}

fn format_tv() -> String {
    let eastern = FixedOffset::west_opt(4 * 3600).unwrap();
    let now = Utc::now().with_timezone(&eastern);
    let hour12 = now.hour() % 12;
    let hour12 = if hour12 == 0 { 12 } else { hour12 };
    let ampm = if now.hour() < 12 { "AM" } else { "PM" };
    format!(
        "v9_c8-{}/{}/{}-{}:{:02}:{:02} {}",
        now.month(),
        now.day(),
        now.year(),
        hour12,
        now.minute(),
        now.second(),
        ampm
    )
}

#[allow(dead_code)]
fn format_date_string() -> String {
    let eastern = FixedOffset::west_opt(4 * 3600).unwrap();
    let now = Utc::now().with_timezone(&eastern);
    const DAYS: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
    const MONTHS: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    let wd = now.weekday().num_days_from_sunday() as usize;
    let mo = (now.month0()) as usize;
    format!(
        "{} {} {} {} {:02}:{:02}:{:02} GMT-0400 (Eastern Daylight Time)",
        DAYS[wd],
        MONTHS[mo],
        now.day(),
        now.year(),
        now.hour(),
        now.minute(),
        now.second()
    )
}

fn uuid4() -> String {
    let mut rng = rand::thread_rng();
    let mut b = [0u8; 16];
    rng.fill(&mut b);
    b[6] = (b[6] & 0x0f) | 0x40;
    b[8] = (b[8] & 0x3f) | 0x80;
    format!(
        "{:08x}-{:04x}-{:04x}-{:04x}-{:012x}",
        u32::from_be_bytes([b[0], b[1], b[2], b[3]]),
        u16::from_be_bytes([b[4], b[5]]),
        u16::from_be_bytes([b[6], b[7]]),
        u16::from_be_bytes([b[8], b[9]]),
        (b[10] as u64) << 40
            | (b[11] as u64) << 32
            | (b[12] as u64) << 24
            | (b[13] as u64) << 16
            | (b[14] as u64) << 8
            | b[15] as u64,
    )
}

fn snare_session_id() -> String {
    let mut rng = rand::thread_rng();
    let a: u16 = rng.gen();
    let b: u64 = rng.gen::<u64>() & 0x0000_00ff_ffff_ffff;
    format!("{:04x}-{:011x}", a, b)
}

fn ua_brands() -> Value {
    json!([
        {"brand": "Google Chrome",  "version": CHROME_FULL_VERSION},
        {"brand": "Not.A/Brand",    "version": "8.0.0.0"},
        {"brand": "Chromium",       "version": CHROME_FULL_VERSION}
    ])
}

fn webgl_extensions() -> Value {
    json!([
        "ANGLE_instanced_arrays",
        "EXT_blend_minmax",
        "EXT_clip_control",
        "EXT_color_buffer_half_float",
        "EXT_depth_clamp",
        "EXT_disjoint_timer_query",
        "EXT_float_blend",
        "EXT_frag_depth",
        "EXT_polygon_offset_clamp",
        "EXT_shader_texture_lod",
        "EXT_texture_compression_bptc",
        "EXT_texture_compression_rgtc",
        "EXT_texture_filter_anisotropic",
        "EXT_texture_mirror_clamp_to_edge",
        "EXT_sRGB",
        "KHR_parallel_shader_compile",
        "OES_element_index_uint",
        "OES_fbo_render_mipmap",
        "OES_standard_derivatives",
        "OES_texture_float",
        "OES_texture_float_linear",
        "OES_texture_half_float",
        "OES_texture_half_float_linear",
        "OES_vertex_array_object",
        "WEBGL_blend_func_extended",
        "WEBGL_color_buffer_float",
        "WEBGL_compressed_texture_s3tc",
        "WEBGL_compressed_texture_s3tc_srgb",
        "WEBGL_debug_renderer_info",
        "WEBGL_debug_shaders",
        "WEBGL_depth_texture",
        "WEBGL_draw_buffers",
        "WEBGL_lose_context",
        "WEBGL_multi_draw",
        "WEBGL_polygon_mode"
    ])
}

fn build_snare_json(page_url: &str, lh: &str, include_uiprc: bool) -> String {
    let mut rng = rand::thread_rng();
    let dl = now_ms();
    let xst = dl + rng.gen_range(400u64..900);
    let rui = uuid4();
    let lsid = uuid4();
    let sstid = snare_session_id();

    let tv = format_tv();

    let mut payload = json!({
        "i_t": OBS_TOKEN,
        "tv": tv,
        "wr": "cosmos",
        "a": {},
        "b": {
            "prs": {
                "acc":   "granted",
                "bgsy":  "granted",
                "cmr":   "prompt",
                "cpr":   "prompt",
                "cpw":   "granted",
                "dpc":   "prompt",
                "gysc":  "granted",
                "gloc":  "prompt",
                "lfnt":  "prompt",
                "mgnt":  "granted",
                "mphn":  "prompt",
                "mdi":   "prompt",
                "ntfn":  "prompt",
                "phnd":  "granted",
                "pstg":  "prompt",
                "stgac": "prompt",
                "wmg":   "prompt"
            },
            "qa":   USER_AGENT,
            "qr":   "en-US",
            "ol":   "Win32",
            "od":   "true",
            "pdd":  "true",
            "udid": "unknown",
            "rt":   "Gecko",
            "rb":   "20030107",
            "er":   "Google Inc.",
            "eb":   "unknown",
            "ot":   "enabled",
            "et":   "enabled",
            "bt":   "unknown",
            "nt":   "enabled",
            "adj":  "false",
            "ie":   "America/New_York",
            "st":   "supported",
            "ls":   ["PDF Viewer", "Chrome PDF Viewer", "Chromium PDF Viewer", "Microsoft Edge PDF Viewer", "WebKit built-in PDF"],
            "fls":  ["internal-pdf-viewer", "internal-pdf-viewer", "internal-pdf-viewer", "internal-pdf-viewer", "internal-pdf-viewer"],
            "is":   ["application/pdf", "text/pdf"],
            "li":   "enabled",
            "ok":   "disabled",
            "sw":   "false",
            "brck": "notbrave",
            "apv":  0
        },
        "d": {
            "esd": {"iaa": "1", "oia": "1", "ivi": "0"},
            "fiopn": {"mx": 100000, "my": 100000},
            "lb":   "100",
            "ci":   "true",
            "cvb":  CANVAS_HASH_B,
            "cvc":  CANVAS_HASH_C,
            "ole":  "Windows",
            "vp":   "19.0.0",
            "mol":  "unknown",
            "teca": "x86",
            "besn": "64",
            "fofr": "unknown",
            "fvl":  ua_brands(),
            "ufv":  CHROME_FULL_VERSION,
            "ww":   "false",
            "mlb":  "false",
            "pv1":  "2560x1305",
            "pv2":  "2560x1305",
            "pmp":  "present",
            "rpd":  "1.5",
            "dti":  "false",
            "ptm":  "0",
            "md":   "32",
            "ch":   "32",
            "cn":   "2560x1440",
            "ca":   "2560x1392",
            "oh":   "32",
            "dp":   "32",
            "sotp": "landscape-primary",
            "sxtn": "isextended",
            "tc": {
                "cef":  "4g",
                "cdl":  "10",
                "cdlm": "unknown",
                "crtt": "50"
            },
            "cva": {
                "er":  WEBGL_VENDOR,
                "rrr": WEBGL_RENDERER,
                "ess": webgl_extensions()
            },
            "eaa": "unknown",
            "bwfl": {
                "Arial":               {"fow": "647.75390625",  "fod": "true"},
                "Helvetica":           {"fow": "647.75390625",  "fod": "true"},
                "American Typewriter": {"fow": "620.05078125",  "fod": "false"},
                "Andale Mono":         {"fow": "620.05078125",  "fod": "false"},
                "Arial Black":         {"fow": "791.9296875",   "fod": "true"},
                "Hoefler Text":        {"fow": "620.05078125",  "fod": "false"},
                "SimSun":              {"fow": "468",           "fod": "true"},
                "Noto Sans Batak":     {"fow": "620.05078125",  "fod": "false"},
                "Calibri":             {"fow": "624.7265625",   "fod": "true"},
                "Cambria":             {"fow": "658.16015625",  "fod": "true"},
                "Sitka":               {"fow": "620.05078125",  "fod": "false"}
            },
            "mter": {
                "acca": "1.4473588658278522",
                "abba": "0.12343746096704435",
                "atta": "0.4636476090008061",
                "caac": "-0.8390715290095377",
                "caah": "709.889355822726",
                "aeem": "2.718281828459045",
                "caal": "0.7639704044417283",
                "sall": "-0.6452512852657808",
                "tala": "-0.8446024630198843",
                "lgtm": "6.907755278982137",
                "pka":  "3.141592653589793",
                "saa":  "0.8414709848078965",
                "saah": "1.1752011936438014",
                "soso": "1.4142135623730951",
                "ntta": "1.5574077246549023",
                "htta": "0.7615941559557649",
                "ln":   "0.6931471805599453"
            }
        },
        "s": {
            "lsid":  lsid,
            "olsid": lsid,
            "sstid": sstid
        },
        "att": "domready_complete",
        "dl":   dl,
        "prerr": "none",
        "lh":   lh,
        "pg":   page_url,
        "wplh": page_url,
        "wlao": "unknown",
        "lii":  "false",
        "liif": "false",
        "xst":  xst,
        "rui":  rui
    });

    if include_uiprc {
        payload["uiprc"] = json!(61);
    }

    serde_json::to_string(&payload).unwrap_or_default()
}

pub async fn post_snare(client: &wreq::Client, domain: &str, page_url: &str, lh: &str) {
    let include_uiprc = domain.contains("identity");
    let json_str = build_snare_json(page_url, lh, include_uiprc);
    let wire = match encrypt_obs(&json_str) {
        Ok(w) => w,
        Err(e) => {
            tracing::debug!("snare encrypt failed for {domain}: {e}");
            return;
        }
    };

    let url = format!("https://{domain}/si/{OBS_TOKEN}/obs");
    let result = client
        .post(&url)
        .header("content-type", "text/plain;charset=UTF-8")
        .header("origin", format!("https://{domain}"))
        .header("user-agent", USER_AGENT)
        .body(wire)
        .send()
        .await;

    if let Err(e) = result {
        tracing::debug!("snare POST to {domain} failed: {e}");
    }
}
