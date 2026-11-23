// Ported from ardia packages/core/src/anonymousIdentity.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! The anonymous name and avatar colours that a string id maps to.

use super::js::{js_max, js_min, js_round};

pub const IDENTITY_ADJECTIVES: [&str; 128] = [
    "Agile", "Amber", "Amiable", "Astute", "Arctic", "Artful", "Autumn", "Azure", "Balmy",
    "Beaming", "Blissful", "Bold", "Bouncy", "Brave", "Breezy", "Bright", "Brisk", "Bubbly",
    "Buoyant", "Calm", "Candid", "Carefree", "Charming", "Cheerful", "Chipper", "Classy", "Clever",
    "Coastal", "Cordial", "Cosmic", "Cozy", "Crimson", "Curious", "Dapper", "Daring", "Dazzling",
    "Dreamy", "Eager", "Earnest", "Elated", "Electric", "Elegant", "Epic", "Fabled", "Fair",
    "Fearless", "Festive", "Fluent", "Fond", "Friendly", "Frosty", "Gallant", "Gentle", "Gilded",
    "Glad", "Gleaming", "Glowing", "Golden", "Graceful", "Grand", "Happy", "Hardy", "Harmonic",
    "Hearty", "Helpful", "Honest", "Hopeful", "Humble", "Jaunty", "Jolly", "Jovial", "Joyful",
    "Keen", "Kind", "Lively", "Loyal", "Lucky", "Lunar", "Magic", "Mellow", "Merry", "Mighty",
    "Misty", "Modest", "Mossy", "Nimble", "Noble", "Northern", "Ocean", "Peppy", "Placid",
    "Playful", "Plucky", "Polite", "Proud", "Quick", "Quiet", "Radiant", "Rapid", "Ready", "Regal",
    "Rosy", "Royal", "Rustic", "Serene", "Silver", "Sincere", "Snowy", "Solar", "Sparkly",
    "Spirited", "Steady", "Stellar", "Sterling", "Sunny", "Swift", "Tidy", "Tranquil", "Trusty",
    "Upbeat", "Valiant", "Velvet", "Vivid", "Warm", "Wise", "Witty", "Zany", "Zesty",
];

pub const IDENTITY_ANIMALS: [&str; 128] = [
    "Albatross",
    "Alpaca",
    "Antelope",
    "Armadillo",
    "Badger",
    "Beaver",
    "Bison",
    "Bluebird",
    "Bobcat",
    "Buffalo",
    "Bunny",
    "Butterfly",
    "Camel",
    "Canary",
    "Capybara",
    "Caribou",
    "Cheetah",
    "Chickadee",
    "Chinchilla",
    "Chipmunk",
    "Cockatoo",
    "Condor",
    "Cardinal",
    "Cougar",
    "Coyote",
    "Crane",
    "Cricket",
    "Dingo",
    "Dolphin",
    "Dove",
    "Dragonfly",
    "Duckling",
    "Eagle",
    "Egret",
    "Elephant",
    "Elk",
    "Falcon",
    "Fawn",
    "Ferret",
    "Finch",
    "Firefly",
    "Flamingo",
    "Fox",
    "Gazelle",
    "Gecko",
    "Gibbon",
    "Giraffe",
    "Goldfinch",
    "Gopher",
    "Gorilla",
    "Grouse",
    "Hare",
    "Hawk",
    "Hedgehog",
    "Heron",
    "Hummingbird",
    "Ibex",
    "Ibis",
    "Iguana",
    "Impala",
    "Jackrabbit",
    "Jaguar",
    "Jay",
    "Kangaroo",
    "Kestrel",
    "Kingfisher",
    "Kitten",
    "Kiwi",
    "Koala",
    "Ladybug",
    "Lark",
    "Lemur",
    "Leopard",
    "Lion",
    "Llama",
    "Lobster",
    "Lynx",
    "Macaw",
    "Magpie",
    "Manatee",
    "Marmot",
    "Marten",
    "Meerkat",
    "Mink",
    "Mongoose",
    "Moose",
    "Narwhal",
    "Newt",
    "Nightingale",
    "Ocelot",
    "Octopus",
    "Oriole",
    "Osprey",
    "Ostrich",
    "Otter",
    "Owl",
    "Panda",
    "Panther",
    "Parrot",
    "Pelican",
    "Penguin",
    "Pheasant",
    "Platypus",
    "Puffin",
    "Puma",
    "Quail",
    "Quokka",
    "Rabbit",
    "Raccoon",
    "Raven",
    "Reindeer",
    "Robin",
    "Salmon",
    "Seahorse",
    "Seal",
    "Sparrow",
    "Squirrel",
    "Starling",
    "Stork",
    "Swallow",
    "Swan",
    "Tiger",
    "Toucan",
    "Turtle",
    "Walrus",
    "Whale",
    "Wombat",
    "Wren",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnonymousAvatar {
    pub colors: [String; 3],
    pub angle: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnonymousIdentity {
    pub name: String,
    pub avatar: AnonymousAvatar,
}

/// `fnv1a32` (`anonymousIdentity.ts:41`) over the UTF-16 code units of `text`.
fn fnv1a32(text: &str) -> u32 {
    text.encode_utf16().fold(0x811c_9dc5, |hash, unit| {
        (hash ^ u32::from(unit)).wrapping_mul(0x0100_0193)
    })
}

/// `sequence` (`anonymousIdentity.ts:49`): a SplitMix32-style generator.
struct Sequence(u32);

impl Sequence {
    fn next(&mut self) -> u32 {
        self.0 = self.0.wrapping_add(0x9e37_79b9);
        let mut z = self.0;
        z = (z ^ (z >> 16)).wrapping_mul(0x85eb_ca6b);
        z = (z ^ (z >> 13)).wrapping_mul(0xc2b2_ae35);
        z ^ (z >> 16)
    }
}

/// `hex` (`anonymousIdentity.ts:60`): `Math.round(value * 255).toString(16).padStart(2, "0")`.
/// `Math.round` rounds half up; `toString(16)` of a double is exact, `NaN` and the
/// infinities give their names.
fn hex(value: f64) -> String {
    let r = js_round(value * 255.0);
    let text = if r.is_nan() {
        "NaN".to_owned()
    } else if r.is_infinite() {
        (if r < 0.0 { "-Infinity" } else { "Infinity" }).to_owned()
    } else if r == 0.0 {
        "0".to_owned()
    } else {
        // r = mantissa * 2^exp with an integer mantissa; the hex digits of the shifted mantissa
        // followed by zeros are exact.
        let bits = r.abs().to_bits();
        // `r` is a nonzero integer, so it is a normal number.
        let mantissa = (bits & ((1 << 52) - 1)) | (1 << 52);
        let exp = ((bits >> 52) & 0x7ff) as i32 - 1075;
        let digits = if exp >= 0 {
            format!(
                "{:x}{}",
                mantissa << (exp % 4),
                "0".repeat((exp / 4) as usize)
            )
        } else {
            format!("{:x}", mantissa >> -exp)
        };
        format!("{}{digits}", if r < 0.0 { "-" } else { "" })
    };
    format!("{text:0>2}")
}

/// Port of `hslHex` (`anonymousIdentity.ts:62`).
pub fn hsl_hex(hue: f64, saturation: f64, lightness: f64) -> String {
    let s = saturation / 100.0;
    let l = lightness / 100.0;
    let k = |n: f64| (n + hue / 30.0) % 12.0;
    let a = s * js_min(l, 1.0 - l);
    let channel = |n: f64| l - a * js_max(-1.0, js_min(js_min(k(n) - 3.0, 9.0 - k(n)), 1.0));
    format!(
        "#{}{}{}",
        hex(channel(0.0)),
        hex(channel(8.0)),
        hex(channel(4.0))
    )
}

fn gradient_stop(hue: i64, saturation: i64, lightness: i64) -> String {
    let h = (hue + 360) % 360;
    let bright = (45..=200).contains(&h);
    hsl_hex(
        h as f64,
        saturation as f64,
        if bright { lightness - 10 } else { lightness } as f64,
    )
}

/// Port of `anonymousIdentity` (`anonymousIdentity.ts:80`). The id is trimmed and lowercased
/// as JavaScript does.
pub fn anonymous_identity(id: &str) -> AnonymousIdentity {
    let id = id.trim_matches(crate::fuzzy::is_js_space).to_lowercase();
    let mut next = Sequence(fnv1a32(&id));
    let adjective = IDENTITY_ADJECTIVES[(next.next() % 128) as usize];
    let animal = IDENTITY_ANIMALS[(next.next() % 128) as usize];
    let hue = i64::from(next.next() % 360);
    let spread = 28 + i64::from(next.next() % 30);
    let direction = if next.next() % 2 == 1 { 1 } else { -1 };
    let saturation = 68 + i64::from(next.next() % 18);
    let lightness = 54 + i64::from(next.next() % 10);
    let colors = [
        gradient_stop(hue, saturation, lightness),
        gradient_stop(hue + direction * spread, saturation, lightness + 4),
        gradient_stop(hue + direction * spread * 2, saturation - 6, lightness - 4),
    ];
    AnonymousIdentity {
        name: format!("{adjective} {animal}"),
        avatar: AnonymousAvatar {
            colors,
            angle: next.next() % 360,
        },
    }
}
