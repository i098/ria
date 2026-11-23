// Ported from ardia packages/core/src/bangList.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! The bundled slim bang lists, in lookup order.

const TSV: &str = include_str!("bang_list.tsv");
// Byte length of Ardia's primary string, before the TSV separator.
const PRIMARY_BYTES: usize = 685_905;
const LISTS: (&str, &str) = TSV.split_at(TSV.len() - 1).0.split_at(PRIMARY_BYTES);

/// The primary list followed by the Unicode fallback list.
pub const BANG_LISTS: [&str; 2] = [LISTS.0, LISTS.1.split_at(1).1];

#[cfg(test)]
mod tests {
    use super::BANG_LISTS;

    #[test]
    fn bundled_lists_match_frozen_ardia_samples() {
        let oracle = include_str!("../../../../tools/bang-tsv/samples.tsv");
        let mut expected = oracle.lines();
        for (name, list) in ["BANG_LIST", "BANG_LIST_UNICODE"]
            .into_iter()
            .zip(BANG_LISTS)
        {
            let count = expected.next().unwrap().parse::<usize>().unwrap();
            let rows: Vec<_> = list.lines().collect();
            assert_eq!(rows.len(), count, "{name} row count");
            for sample in 0..100 {
                let index = sample * (count - 1) / 99;
                assert_eq!(rows[index], expected.next().unwrap(), "{name} row {index}");
            }
        }
        assert_eq!(expected.next(), None);
    }
}
