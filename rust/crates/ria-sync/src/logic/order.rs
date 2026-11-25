// Ported from ardia packages/sync/src/order.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! Fractional positions in base 62: keep the longest increasing run of the given positions and
//! give every other item a new position between its neighbours.
//!
//! Changes from Ardia:
//! - The work runs on UTF-16 code units, as JS strings do (`assign_positions_utf16`). The
//!   `String` form `assign_positions` turns a lone surrogate into U+FFFD, and the error text
//!   always does.

const DIGITS: &[u8; 62] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
const BASE: i32 = 62;
const ZERO: u16 = b'0' as u16;

/// Port of `digit` (`packages/sync/src/order.ts:3`): `-1` for a unit that is not a digit.
fn digit(c: Option<&u16>) -> i32 {
    match c {
        None => 0,
        Some(&c) => DIGITS
            .iter()
            .position(|&d| u16::from(d) == c)
            .map_or(-1, |i| i as i32),
    }
}

/// Port of `positionBetween` (`packages/sync/src/order.ts:5`).
fn position_between(a: &[u16], b: Option<&[u16]>) -> Result<Vec<u16>, String> {
    if let Some(b) = b {
        if a >= b {
            return Err(format!(
                "positionBetween: {} >= {}",
                String::from_utf16_lossy(a),
                String::from_utf16_lossy(b)
            ));
        }
        let mut n = 0;
        while b.get(n) == Some(a.get(n).unwrap_or(&ZERO)) {
            n += 1;
        }
        if n > 0 {
            let mut out = b[..n].to_vec();
            out.extend(position_between(
                a.get(n..).unwrap_or_default(),
                Some(&b[n..]),
            )?);
            return Ok(out);
        }
    }
    let da = digit(a.first());
    let db = b.map_or(BASE, |b| digit(b.first()));
    if db - da > 1 {
        // `da + db` is not negative here, so `round` matches `Math.round`.
        return Ok(vec![u16::from(
            DIGITS[(f64::from(da + db) / 2.0).round() as usize],
        )]);
    }
    if let Some(b) = b.filter(|b| b.len() > 1) {
        return Ok(b[..1].to_vec());
    }
    // `DIGITS[-1]` is `undefined`, which the concatenation prints.
    let mut out: Vec<u16> = match usize::try_from(da) {
        Ok(i) => vec![u16::from(DIGITS[i])],
        Err(_) => "undefined".encode_utf16().collect(),
    };
    out.extend(position_between(a.get(1..).unwrap_or_default(), None)?);
    Ok(out)
}

/// Port of `assignPositions` (`packages/sync/src/order.ts:19`). `None` is `undefined`.
pub fn assign_positions(existing: &[Option<String>]) -> Result<Vec<String>, String> {
    let units: Vec<Option<Vec<u16>>> = existing
        .iter()
        .map(|p| p.as_deref().map(|p| p.encode_utf16().collect()))
        .collect();
    let out = assign_positions_utf16(&units)?;
    Ok(out.iter().map(|p| String::from_utf16_lossy(p)).collect())
}

/// `assign_positions` on the UTF-16 code units of each position, which keeps lone surrogates.
pub fn assign_positions_utf16(existing: &[Option<Vec<u16>>]) -> Result<Vec<Vec<u16>>, String> {
    let mut tails: Vec<usize> = Vec::new();
    let mut previous: Vec<Option<usize>> = vec![None; existing.len()];
    for (i, p) in existing.iter().enumerate() {
        let Some(p) = p
            .as_deref()
            .filter(|p| p.last().is_some_and(|&c| c != ZERO))
        else {
            continue;
        };
        let lo = tails.partition_point(|&t| existing[t].as_deref().is_some_and(|e| e < p));
        if lo > 0 {
            previous[i] = Some(tails[lo - 1]);
        }
        if lo == tails.len() {
            tails.push(i);
        } else {
            tails[lo] = i;
        }
    }
    let mut keep = vec![false; existing.len()];
    let mut at = tails.last().copied();
    while let Some(i) = at {
        keep[i] = true;
        at = previous[i];
    }

    let mut out: Vec<Vec<u16>> = existing
        .iter()
        .zip(&keep)
        .map(|(p, &k)| match p {
            Some(p) if k => p.clone(),
            _ => Vec::new(),
        })
        .collect();
    let mut next_kept: Vec<Option<usize>> = vec![None; existing.len()];
    let mut next = None;
    for i in (0..existing.len()).rev() {
        next_kept[i] = next;
        if keep[i] {
            next = Some(i);
        }
    }
    let mut before: usize = usize::MAX;
    for i in 0..existing.len() {
        if !keep[i] {
            let a: &[u16] = if before == usize::MAX {
                &[]
            } else {
                &out[before]
            };
            let pos = position_between(a, next_kept[i].map(|n| out[n].as_slice()))?;
            out[i] = pos;
        }
        before = i;
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn some(list: &[&str]) -> Vec<Option<String>> {
        list.iter().map(|s| Some((*s).to_owned())).collect()
    }

    #[test]
    fn new_items_fall_between_kept_ones() {
        assert_eq!(assign_positions(&[None]).unwrap(), vec!["V"]);
        assert_eq!(
            assign_positions(&[Some("1".into()), None, Some("2".into())]).unwrap(),
            vec!["1", "1V", "2"]
        );
        assert_eq!(
            assign_positions(&some(&["!", "", "~"])).unwrap(),
            vec!["!", "undefinedV", "~"]
        );
    }

    #[test]
    fn a_bad_neighbour_throws() {
        let got = assign_positions(&[Some("1".into()), None, None, Some("1!".into())]);
        assert_eq!(got, Err("positionBetween: 10V >= 1!".to_owned()));
    }
}
