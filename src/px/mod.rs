mod collector;
mod encoder;
mod fingerprint;
mod ob_decoder;
mod payload;
pub mod snare;
mod uuid_v1;

use anyhow::Result;
use std::time::SystemTime;

pub use uuid_v1::uuid_v1;

pub struct PxCookies {
    pub px3: String,
    pub pxvid: String,
    pub pxcts: String,
    pub pxde: String,
    pub pxhd: Option<String>,
}

pub struct PxSession {
    client: wreq::Client,
    tag: String,
    xor_key: u8,
}

impl PxSession {
    pub async fn new(client: wreq::Client) -> Result<Self> {
        tracing::info!("PX: fetching sensor tag from init.js");
        let tag = collector::fetch_tag(&client).await?;
        let xor_key = encoder::compute_ob_xor_key(&tag);
        tracing::debug!(tag = %tag, xor_key, "PxSession ready");
        tracing::info!("PX: sensor tag acquired, XOR key derived");
        Ok(Self {
            client,
            tag,
            xor_key,
        })
    }

    pub async fn solve(&self, page_url: &str) -> Result<PxCookies> {
        let domain = if page_url.contains("identity.walmart.com") {
            "identity"
        } else {
            "www"
        };
        tracing::info!("PX [{domain}]: posting seq=0 collector payload");
        let uuid = uuid_v1();

        let sts = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_millis().to_string())
            .unwrap_or_else(|_| "1777000000000".to_string());

        let origin = if page_url.contains("identity.walmart.com") {
            "https://identity.walmart.com"
        } else {
            "https://www.walmart.com"
        };

        let (body0, seq0_perf_now) = payload::build_seq0_body(&uuid, &sts, &self.tag, page_url);
        let resp0 = collector::post_collector(&self.client, body0, origin).await?;
        let ob0_recv_ts = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        tracing::debug!(ob_len = resp0.ob.len(), "collector seq=0 response");

        let ob0_text = ob_decoder::decode_ob(&resp0.ob, self.xor_key)?;
        tracing::debug!(ob0 = %ob0_text, "decoded OB seq=0");

        let roles0 = ob_decoder::auto_map(&ob0_text);
        let cookies0 = ob_decoder::extract_cookies(&ob0_text);

        let cs = roles0.get(ob_decoder::ROLE_CS).cloned().unwrap_or_default();
        let vid = roles0
            .get(ob_decoder::ROLE_VID)
            .cloned()
            .unwrap_or_default();
        let sid = roles0
            .get(ob_decoder::ROLE_SID)
            .cloned()
            .unwrap_or_default();
        let cts = roles0
            .get(ob_decoder::ROLE_CTS)
            .cloned()
            .unwrap_or_default();

        let server_ts: u64 = roles0
            .get(ob_decoder::ROLE_TIMESTAMP)
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        let token = roles0
            .get(ob_decoder::ROLE_TOKEN)
            .cloned()
            .unwrap_or_default();

        tracing::debug!(
            cs_len = cs.len(),
            vid_len = vid.len(),
            sid_len = sid.len(),
            server_ts,
            token_len = token.len(),
            "OB seq=0 roles extracted"
        );
        let pxhd_from_ob0 = cookies0.get("_pxhd").cloned();

        tracing::info!("PX [{domain}]: seq=0 OK — vid/cs/token extracted, posting seq=1");

        let body1 = payload::build_seq1_body(payload::Seq1BodyInput {
            uuid: &uuid,
            sts: &sts,
            tag: &self.tag,
            page_url,
            cs: &cs,
            vid: &vid,
            sid: &sid,
            pxhd: pxhd_from_ob0.as_deref().unwrap_or(""),
            cts: &cts,
            server_ts,
            token: &token,
            ob0_recv_ts,
            seq0_perf_now,
        });
        let resp1 = collector::post_collector(&self.client, body1, origin).await?;
        tracing::debug!(ob_len = resp1.ob.len(), "collector seq=1 response");

        let ob1_text = ob_decoder::decode_ob(&resp1.ob, self.xor_key)?;
        tracing::debug!(ob1 = %ob1_text, "decoded OB seq=1");

        let cookies1 = ob_decoder::extract_cookies(&ob1_text);
        let roles1 = ob_decoder::auto_map(&ob1_text);

        let px3 = cookies1
            .get("_px3")
            .or_else(|| cookies0.get("_px3"))
            .cloned()
            .unwrap_or_default();
        let pxvid = cookies1
            .get("_pxvid")
            .or_else(|| cookies0.get("_pxvid"))
            .or_else(|| roles1.get(ob_decoder::ROLE_VID))
            .or(if !vid.is_empty() { Some(&vid) } else { None })
            .cloned()
            .unwrap_or_default();

        let pxcts = cookies1
            .get("pxcts")
            .or_else(|| cookies0.get("pxcts"))
            .or(if !cts.is_empty() { Some(&cts) } else { None })
            .or_else(|| roles1.get(ob_decoder::ROLE_SID))
            .cloned()
            .unwrap_or_default();

        let pxde = cookies1
            .get("_pxde")
            .or_else(|| cookies0.get("_pxde"))
            .cloned()
            .unwrap_or_default();

        let pxhd = pxhd_from_ob0.or_else(|| cookies1.get("_pxhd").cloned());

        if px3.is_empty() {
            tracing::warn!("PX [{domain}]: seq=1 returned empty _px3 — session may be flagged");
        } else {
            tracing::info!("PX [{domain}]: seq=1 OK — _px3 acquired");
        }

        Ok(PxCookies {
            px3,
            pxvid,
            pxcts,
            pxde,
            pxhd,
        })
    }
}
