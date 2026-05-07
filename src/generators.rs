use base64::Engine;
use rand::seq::SliceRandom;
use rand::Rng;

const FIRST_NAMES: &[&str] = &[
    "James",
    "Mary",
    "John",
    "Patricia",
    "Robert",
    "Jennifer",
    "Michael",
    "Linda",
    "David",
    "Elizabeth",
    "William",
    "Barbara",
    "Richard",
    "Susan",
    "Joseph",
    "Jessica",
    "Thomas",
    "Sarah",
    "Charles",
    "Karen",
    "Christopher",
    "Nancy",
    "Daniel",
    "Lisa",
    "Matthew",
    "Margaret",
    "Anthony",
    "Betty",
    "Mark",
    "Sandra",
    "Donald",
    "Ashley",
    "Steven",
    "Kimberly",
    "Paul",
    "Emily",
    "Andrew",
    "Donna",
    "Joshua",
    "Michelle",
    "Kenneth",
    "Carol",
    "Kevin",
    "Amanda",
    "Brian",
    "Melissa",
    "George",
    "Deborah",
    "Edward",
    "Stephanie",
    "Ronald",
    "Rebecca",
];

const LAST_NAMES: &[&str] = &[
    "Smith",
    "Johnson",
    "Williams",
    "Brown",
    "Jones",
    "Garcia",
    "Miller",
    "Davis",
    "Rodriguez",
    "Martinez",
    "Hernandez",
    "Lopez",
    "Gonzalez",
    "Wilson",
    "Anderson",
    "Thomas",
    "Taylor",
    "Moore",
    "Jackson",
    "Martin",
    "Lee",
    "Perez",
    "Thompson",
    "White",
    "Harris",
    "Sanchez",
    "Clark",
    "Ramirez",
    "Lewis",
    "Robinson",
    "Walker",
    "Young",
    "Allen",
    "King",
    "Wright",
    "Scott",
    "Torres",
    "Nguyen",
    "Hill",
    "Flores",
    "Green",
    "Adams",
    "Nelson",
    "Baker",
    "Hall",
    "Rivera",
    "Campbell",
    "Mitchell",
    "Carter",
    "Roberts",
];

pub fn random_first_name() -> String {
    FIRST_NAMES
        .choose(&mut rand::thread_rng())
        .unwrap()
        .to_string()
}

pub fn random_last_name() -> String {
    LAST_NAMES
        .choose(&mut rand::thread_rng())
        .unwrap()
        .to_string()
}

pub fn random_password() -> String {
    let lower = "abcdefghijkmnpqrstuvwxyz";
    let upper = "ABCDEFGHJKLMNPQRSTUVWXYZ";
    let digits = "23456789";
    let mut rng = rand::thread_rng();
    let mut pw: Vec<char> = Vec::with_capacity(14);
    pw.push(upper.chars().nth(rng.gen_range(0..upper.len())).unwrap());
    pw.push(lower.chars().nth(rng.gen_range(0..lower.len())).unwrap());
    pw.push(digits.chars().nth(rng.gen_range(0..digits.len())).unwrap());
    let pool: Vec<char> = format!("{lower}{upper}{digits}").chars().collect();
    for _ in 0..11 {
        pw.push(*pool.choose(&mut rng).unwrap());
    }
    pw.shuffle(&mut rng);
    pw.into_iter().collect()
}

pub fn trace_id() -> String {
    let mut buf = [0u8; 16];
    rand::thread_rng().fill(&mut buf);
    hex::encode_lower(buf)
}

pub fn span_id() -> String {
    let mut buf = [0u8; 8];
    rand::thread_rng().fill(&mut buf);
    hex::encode_lower(buf)
}

pub fn correlation_id() -> String {
    let mut buf = [0u8; 27];
    rand::thread_rng().fill(&mut buf);
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(buf)
}

pub fn pkce_pair() -> (String, String) {
    use sha2::{Digest, Sha256};
    let mut buf = [0u8; 32];
    rand::thread_rng().fill(&mut buf);
    let verifier = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(buf);
    let hash = Sha256::digest(verifier.as_bytes());
    let challenge = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(hash);
    (verifier, challenge)
}

pub fn device_profile_ref_id() -> String {
    fn rand_chars(rng: &mut impl Rng, n: usize, allow_underscore: bool) -> String {
        let pool: Vec<char> = if allow_underscore {
            "abcdefghijklmnopqrstuvwxyz0123456789_".chars().collect()
        } else {
            "abcdefghijklmnopqrstuvwxyz0123456789".chars().collect()
        };
        (0..n).map(|_| *pool.choose(rng).unwrap()).collect()
    }
    let mut rng = rand::thread_rng();
    format!(
        "{}-{}",
        rand_chars(&mut rng, 13, false),
        rand_chars(&mut rng, 35, true)
    )
}

mod hex {
    pub fn encode_lower(bytes: impl AsRef<[u8]>) -> String {
        let mut s = String::with_capacity(bytes.as_ref().len() * 2);
        for b in bytes.as_ref() {
            s.push(char::from_digit((b >> 4) as u32, 16).unwrap());
            s.push(char::from_digit((b & 0xf) as u32, 16).unwrap());
        }
        s
    }
}
