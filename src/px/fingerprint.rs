use crate::USER_AGENT;
use chrono::{Datelike, FixedOffset, Timelike, Utc};
use rand::Rng;
use serde_json::json;
use std::time::SystemTime;

const CHROME_FULL_VERSION: &str = "147.0.7727.138";

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(1777000000000)
}

fn js_date_string() -> String {
    let eastern = FixedOffset::west_opt(4 * 3600).unwrap();
    let now = Utc::now().with_timezone(&eastern);
    const DAYS: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
    const MONTHS: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    let wd = now.weekday().num_days_from_sunday() as usize;
    let mo = now.month0() as usize;
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

pub fn build_seq0_json(page_url: &str, uuid: &str, sts: u64, perf_now: u64) -> String {
    let ts2 = sts + 11;
    serde_json::to_string(&json!([{
        "t": "eW5CLzwARxg=",
        "d": {
            "cyhIaTVAQF4=": page_url,
            "NAtPSnFnQ38=": 0,
            "InlZeGcTUEI=": "Win32",
            "Vi1tLBBKYRw=": 0,
            "Ahk5WERyM2o=": perf_now,
            "Fm0tbFMBJVY=": 3600,
            "ajERMCxcFQc=": sts,
            "U0goSRYkLHs=": ts2,
            "HUJmQ1soY3c=": uuid,
            "AEc7BkUsMTA=": null,
            "SlFxEA86eyc=": 0,
            "Q3g4OQUVMwI=": true
        }
    }]))
    .unwrap_or_default()
}

pub struct Seq1JsonInput<'a> {
    pub page_url: &'a str,
    pub uuid: &'a str,
    pub sts: u64,
    pub server_ts: u64,
    pub token: &'a str,
    pub ob0_recv_ts: u64,
    pub seq0_perf_now: u64,
    pub pxhd: Option<&'a str>,
}

pub fn build_seq1_json(input: Seq1JsonInput<'_>) -> String {
    let Seq1JsonInput {
        page_url,
        uuid,
        sts,
        server_ts,
        token,
        ob0_recv_ts,
        seq0_perf_now,
        pxhd,
    } = input;
    let mut rng = rand::thread_rng();

    let ob0_recv_perf: u64 = ob0_recv_ts.saturating_sub(sts) + seq0_perf_now;

    let sl_fx = ((ob0_recv_perf as f64) * 0.75).round() as u64;

    let seq1_perf_now: u64 = ob0_recv_perf + rng.gen_range(150u64..250);

    let ts_send = now_ms();

    let used_heap: u64 = 51_753_018u64
        .saturating_add(rng.gen_range(0u64..2_600_000))
        .saturating_sub(1_300_000);
    let total_heap: u64 = 73_209_630u64
        .saturating_add(rng.gen_range(0u64..2_600_000))
        .saturating_sub(1_300_000);

    let app_version = USER_AGENT.trim_start_matches("Mozilla/");
    let date_str = js_date_string();

    serde_json::to_string(&json!([{
        "t": "bRJWEyt5UyE=",
        "d": {

            "HwRkBVluazY=": server_ts,

            "UTYqdxRdL0w=": ["Go4", "Ahn5duYB"],

            "VQouCxBmJTE=": true,
            "YjkZOCRRHA4=": false,
            "LnVVdGsZUEI=": false,
            "BFs/GkEwMiw=": true,

            "EmkpaFcCJF8=": "TypeError: Cannot read properties of undefined (reading 'width')",
            "cHcLdjUcBkI=": "webkit",

            "T3Q0NQofOQA=": 33,
            "R3w8PQIXMQc=": false,
            "GmEhYF8KKVc=": false,
            "KV4SX2w1F24=": false,
            "Qll5GAcyfC8=": "AudioData.SVGAnimatedAngle.SVGMetadataElement",
            "fWJGIzgPSRE=": "109|66|66|70|80",
            "cRZKFzd/RiA=": 575,
            "T3Q0NQkTOw8=": true,
            "MDdLNnZfRwY=": true,
            "RTo+ewBUMEg=": "false",
            "cRZKFzR8RCQ=": "false",
            "MklJCHcmRz4=": 1,
            "IUYaR2cuFnw=": 1,
            "EmkpaFcFLFs=": "",
            "T3Q0NQkSMAY=": ["loadTimes", "csi", "app"],
            "BFs/GkEzOyw=": true,

            "WiFhIBxHaRE=": 2560,
            "CX5yP08Xdgw=": 1440,
            "GmEhYF8OL1M=": 2560,
            "DXJ2M0gdeAk=": 1392,
            "IxgYGWZ1HCw=": "2560X1440",
            "cyhIaTZGRFg=": 32,
            "AEc7BkYqPzQ=": 32,

            "Vi1tLBNFaRg=": 138,
            "Nk1NDHMlSTk=": 118,

            "KV4SX280F2k=": 1251,
            "VQouCxBgIzg=": 1278,
            "AEc7BkUtMzc=": 0,
            "IxgYGWZyECw=": 0,
            "GmEhYFwKLlE=": true,
            "Fm0tbFMDJlc=": false,

            "XiVlJBtOYR4=": "webkit",
            "Azh4eUZTcUo=": "https:",
            "XQImAxhpLzM=": "function share() { [native code] }",
            "OS4Cb3xFC14=": "America/New_York",
            "MDdLNnVcQQM=": "w3c",
            "ICdbJmVMUBI=": "screen",
            "Jn1dfGMWVEs=": {
                "plugext": {
                    "0": {"f": "internal-pdf-viewer", "n": "PDF Viewer"},
                    "1": {"f": "internal-pdf-viewer", "n": "Chrome PDF Viewer"},
                    "2": {"f": "internal-pdf-viewer", "n": "Chromium PDF Viewer"},
                    "3": {"f": "internal-pdf-viewer", "n": "Microsoft Edge PDF Viewer"},
                    "4": {"f": "internal-pdf-viewer", "n": "WebKit built-in PDF"}
                },
                "plugins_len": 5
            },
            "eW5CLzwFRh4=": {"smd": {"ok": true, "ex": false}},
            "MDdLNnVcQAw=": {},
            "eyBAYT5LRVc=": false,
            "KnFRcG8aW0Q=": false,

            "eyBAYT5LS1I=": "ff702ecc",
            "fEMHAjkoDDk=": {
                "support": true,
                "status": {"effectiveType": "4g", "rtt": 50, "downlink": 10, "saveData": false}
            },
            "UilpKBdCbRs=": "default",
            "OkFBAH8qRTA=": 3,
            "KxAQEW57FCQ=": false,
            "UBdrVhZ+Z2U=": "22478756611045748042",
            "EFcrFlU9JSQ=": "82755875894983951648",

            "1;><<1><10=01:0<8?=1": "0:?==0?=01<10;1=9><0",
            "cHcLdjUdAkA=": 1716,
            "EwhoCVZjbTw=": 1,
            "PANHQnpoS3g=": "49e5084e",
            "OkFBAHwrSDA=": "7c5f9724",
            "TTI2cwheO0k=": "65d826e0",
            "KD9TPm1VVw0=": "a9269e00",
            "JDtfOmJSWwg=": "50a5ec55",
            "AWZ6J0QOcBc=": "73a0fb26",
            "EmkpaFcBI14=": true,
            "YGcbZiUPEVE=": true,
            "GwBgAV5oajU=": true,
            "N2wMLXIEBhg=": false,
            "YGcbZiUPEVw=": true,
            "eE8DDj0nCTU=": true,
            "KD9TPm1XWAw=": true,
            "Nk1NDHAgQT0=": false,
            "Zj0dPCBWEAs=": false,
            "LnVVdGsaWkU=": false,
            "LnVVdGsYXUY=": false,
            "AEc7BkYuNDw=": false,
            "OkFBAHwoTzE=": false,
            "PSIGY3tPAlg=": false,
            "VGtvahINYFE=": false,
            "OkFBAHwrSzI=": false,
            "eyBAYT5PTFE=": false,
            "WQ4iDx9jKTU=": false,
            "cgkJSDdkBH0=": false,
            "ChExUE95OmM=": false,
            "Bh09XEN1MWg=": "ee",
            "Z1xcXSI3V2w=": "130db",
            "EFcrFlU/JyY=": 1.5_f64,

            "ajERMCxaHws=": used_heap,
            "EXZqN1cdYwA=": 4_294_967_296_u64,
            "WG9jbh4JbF8=": total_heap,

            "X0QkRRkiLHc=": date_str,
            "T3Q0NQkfOgE=": false,
            "IUYaR2cuHnE=": false,
            "Ahk5WER/NGg=": false,
            "V0wsTRIhI3o=": true,
            "U0goSRYmLHs=": 0,
            "LVIWU2s6HWk=": false,
            "ZHsfeiIWF0E=": "visible",
            "dg0NTDNgCHk=": false,
            "Bh09XEBwOWk=": 0,
            "Qll5GAc2cCo=": 1266,
            "dyxMbTJBQFs=": false,
            "GU5iT18ma3w=": 1372,
            "Y1hYWSU+Umw=": "missing",
            "DFM3Ekk/PiQ=": true,
            "LnVVdGgeXUY=": true,
            "KnFRcGwaWUo=": false,
            "a1BQUS06WGU=": true,
            "AWZ6J0QNcxU=": 0,
            "ChExUE97PmM=": 0,
            "fgUFRDttCHU=": "7200536400619077451328013",
            "LVIWU2s1E2A=": 1,
            "JVoeW2M8EW4=": 48,
            "ZRpeGyNyUSs=": 2,
            "aR5SHy90XiQ=": 1,
            "UBdrVhV/Z2w=": false,
            "BFs/GkEwNyw=": true,
            "N2wMLXIEAB4=": false,
            "OkFBAHwnTTY=": "73a1471ef1c2ce67f7a4b908535b5709",
            "LDNXMmlcWgg=": token,
            "GmEhYFwIKVQ=": "d9584be9865c5b907cf79681e6cb43c3",
            "MklJCHQkQjs=": "a5d95f7873a06938777d292d34088836",
            "CzBwcU5bfEI=": "7cd4d9112be6d56a093d4573861492eb",
            "Nk1NDHMlQTc=": "}lZgs>[|/PjN,Dpf",
            "XQImAxhsLDg=": [
                "PDF Viewer", "Chrome PDF Viewer", "Chromium PDF Viewer",
                "Microsoft Edge PDF Viewer", "WebKit built-in PDF"
            ],
            "Qll5GAc1fSw=": 5,
            "LVIWU2s5HWc=": true,
            "fgUFRDtoCH4=": true,
            "MVYKV3c7DmE=": true,
            "RTo+ewNcOk8=": true,
            "cHcLdjYcD0c=": "en-US",
            "InlZeGcTUEI=": "Win32",
            "PSIGY3tKAlA=": ["en-US", "en"],
            "ChExUEx4OGY=": USER_AGENT,
            "YjkZOCdXFg0=": true,
            "b1RUVSk5UWc=": 240,
            "T3Q0NQkcOgY=": 32,
            "KxAQEW14GSQ=": 2,
            "MVYKV3Q4DmQ=": "Gecko",
            "LnVVdGgSWU4=": "20030107",
            "FCsvKlFGIhw=": app_version,
            "PkVFBHgjSz8=": true,
            "DhU1VEh/P2I=": true,
            "KV4SX2wwG2k=": 2,
            "MVYKV3cwBGQ=": "Netscape",
            "fEMHAjopDDk=": "Mozilla",
            "LxQUFWl8HyM=": true,
            "Zj0dPCNRFQ8=": 50,
            "EXZqN1cbYQc=": false,
            "T3Q0NQkcMAQ=": 10,
            "GmEhYFwKK1M=": "4g",
            "U0goSRUuI3w=": true,
            "a1BQUS4/XGM=": true,
            "TBN3Ugl7fWE=": true,
            "fWJGIzgJShc=": "x86",
            "cRZKFzR9RiI=": "64",
            "UBdrVhV8Z2w=": [
                {"brand": "Google Chrome", "version": "147"},
                {"brand": "Not.A/Brand",   "version": "8"},
                {"brand": "Chromium",      "version": "147"}
            ],
            "Bh09XEN2MWc=": false,
            "IUYaR2QtF3U=": "",
            "CzBwcU5bfUI=": "Windows",
            "X0QkRRovKXU=": "19.0.0",
            "eW5CLzwFTx4=": CHROME_FULL_VERSION,
            "YjkZOCdSEQI=": true,
            "TBN3Ugl4f2k=": true,
            "cgkJSDdkA3g=": "9f4ad436c590825b763613cc0227fb6e",
            "Z1xcXSI0WGc=": true,
            "ZHsfeiITG00=": 32,
            "InlZeGcRUkg=": false,
            "NSoOa3BCAl0=": "5CsRqm",
            "NAtPSnFnQ38=": 0,
            "dEsPCjIgADA=": 2,
            "LVIWU2s1EmU=": "TypeError: Cannot read properties of null (reading '0')\n    at kv (https://www.walmart.com/px/PXu6b0qd2S/init.js:2:68293)\n    at yK (https://www.walmart.com/px/PXu6b0qd2S/init.js:3:76184)\n    at yw (https://www.walmart.com/px/PXu6b0qd2S/init.js:3:74706)\n    at https://www.walmart.com/px/PXu6b0qd2S/init.js:3:74675",
            "cyhIaTVAQF4=": page_url,
            "ZHsfeiIQGk8=": [],
            "GC8jLl1BLR8=": "",
            "AEc7BkYqNDM=": false,
            "DzR0dUpceEI=": 770897.2_f64,
            "cHcLdjUfB0c=": [-5524.291_f64],
            "AWZ6J0cNdRw=": "64556c77",
            "cRZKFzd9RC0=": "",
            "FCsvKlFHJBk=": "10207b2f",
            "DhU1VEhzPW8=": "10207b2f",
            "DXJ2M0gYfAU=": "90e65465",
            "XQImAxtvLzc=": true,
            "QAd7RgVtdXI=": true,
            "cHcLdjYeDkU=": true,
            "S3AwMQ0bPQQ=": true,
            "bHMXcikYGUA=": true,
            "SlFxEA86fyY=": "4YC14YCd4YCd4YCV4YCe4YCX4YGS5J256aus7r266YaI5oCR7r27",
            "LVIWU2g5GGY=": "d4acbe702b2ce9d7b185cbf0062c8dea",
            "OA9DTn1lR3o=": null,
            "UilpKBdFYxo=": USER_AGENT,
            "DzR0dUpYfE4=": false,
            "FUpuS1Msa34=": "90e65465",

            "NSoOa3BFBFA=": 2,
            "Vi1tLBBKYRw=": 1,
            "Ahk5WERyM2o=": seq1_perf_now,
            "TlV1FAg4eiQ=": ob0_recv_ts,
            "Fm0tbFMBJVY=": 3600,
            "ajERMCxcFQc=": sts,
            "U0goSRYkLHs=": ts_send,
            "HUJmQ1soY3c=": uuid,
            "AEc7BkUsMTA=": pxhd,
            "SlFxEA86eyc=": sl_fx,
            "Q3g4OQUVMwI=": true
        }
    }])).unwrap_or_default()
}
