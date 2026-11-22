#![cfg(feature = "std")]

extern crate roaring;

use roaring::RoaringBitmap;

fn serialized_size(bitmap: &RoaringBitmap) -> usize {
    let mut buffer = Vec::with_capacity(bitmap.serialized_size());
    bitmap.serialize_into(&mut buffer).unwrap();
    assert_eq!(buffer.len(), bitmap.serialized_size());
    buffer.len()
}

#[test]
fn consecutive_inserts_are_run_encoded() {
    let mut bitmap = RoaringBitmap::new();
    for value in 0..2_000_000 {
        assert!(bitmap.insert(value));
    }

    let statistics = bitmap.statistics();
    assert_eq!(statistics.n_bitset_containers, 0);
    assert_eq!(statistics.n_run_containers, 31);
    assert!(serialized_size(&bitmap) < 1024);

    let mut buffer = Vec::new();
    bitmap.serialize_into(&mut buffer).unwrap();
    assert_eq!(RoaringBitmap::deserialize_from(&buffer[..]).unwrap(), bitmap);
}

#[test]
fn month_of_consecutive_ids_serializes_small() {
    let bitmap = (0..30_000_000).collect::<RoaringBitmap>();

    let statistics = bitmap.statistics();
    assert_eq!(statistics.n_bitset_containers, 0);
    assert_eq!(statistics.n_run_containers, 458);
    assert!(serialized_size(&bitmap) < 10_000);
}

#[test]
fn small_contiguous_array_is_run_encoded() {
    let bitmap = (0..1_000).collect::<RoaringBitmap>();

    assert_eq!(bitmap.statistics().n_run_containers, 1);
    assert!(serialized_size(&bitmap) < 100);
}

#[test]
fn segmented_inserts_are_run_encoded() {
    let mut bitmap = RoaringBitmap::new();
    for key in 0..8 {
        for segment in 0..4 {
            let start = key * (1 << 16) + segment * 16_000;
            for value in start..start + 1_100 {
                bitmap.insert(value);
            }
        }
    }

    let statistics = bitmap.statistics();
    assert_eq!(statistics.n_bitset_containers, 0);
    assert_eq!(statistics.n_run_containers, 8);
    assert!(serialized_size(&bitmap) < 8 * 8192);
}

#[test]
fn fragmented_run_is_demoted() {
    let mut bitmap = RoaringBitmap::from_iter(0..1_000);
    for value in (10_000..65_000).step_by(4) {
        bitmap.insert(value);
    }

    assert_eq!(bitmap.statistics().n_run_containers, 0);
    assert!(serialized_size(&bitmap) < 2 * 8192);
}

#[test]
fn insert_range_reuses_existing_run() {
    let mut bitmap = RoaringBitmap::new();
    for value in (0..4090).step_by(2) {
        bitmap.insert(value);
    }
    assert_eq!(bitmap.statistics().n_array_containers, 1);

    bitmap.insert_range(4090..50_000);

    let statistics = bitmap.statistics();
    assert_eq!(statistics.n_run_containers, 1);
    assert_eq!(bitmap.len(), 2_045 + 45_910);
}

#[test]
fn sparse_inserts_stay_bitmap_encoded() {
    let mut bitmap = RoaringBitmap::new();
    for value in (0..65_000).step_by(13) {
        bitmap.insert(value);
    }

    let statistics = bitmap.statistics();
    assert_eq!(statistics.n_run_containers, 0);
    assert_eq!(statistics.n_bitset_containers, 1);
    assert!(serialized_size(&bitmap) < 2 * 8192);
}

#[test]
fn extend_of_consecutive_ids_is_run_encoded() {
    let mut bitmap = RoaringBitmap::new();
    bitmap.extend(0..1_000_000);

    let statistics = bitmap.statistics();
    assert_eq!(statistics.n_bitset_containers, 0);
    assert_eq!(statistics.n_run_containers, 16);
    assert!(serialized_size(&bitmap) < 1024);
}

#[test]
fn append_of_consecutive_ids_is_run_encoded() {
    let mut bitmap = RoaringBitmap::new();
    assert_eq!(bitmap.append(0..1_000_000), Ok(1_000_000));

    let statistics = bitmap.statistics();
    assert_eq!(statistics.n_bitset_containers, 0);
    assert_eq!(statistics.n_run_containers, 16);
    assert!(serialized_size(&bitmap) < 1024);
}

#[test]
fn extend_across_container_boundary_is_run_encoded() {
    let mut bitmap = RoaringBitmap::from_iter(0..1_000);
    bitmap.extend(1_000..70_000);

    let statistics = bitmap.statistics();
    assert_eq!(statistics.n_bitset_containers, 0);
    assert_eq!(statistics.n_run_containers, 2);
    assert_eq!(bitmap.len(), 70_000);
}

#[test]
fn batch_filled_bitmap_is_run_encoded() {
    let mut bitmap = RoaringBitmap::new();
    // The first batch is sparse enough to require a bitmap container.
    bitmap.extend((0..60_000).step_by(4));
    assert_eq!(bitmap.statistics().n_bitset_containers, 1);

    // The second batch fills the gaps, a run is now the smallest encoding.
    bitmap.extend(0..60_000);

    let statistics = bitmap.statistics();
    assert_eq!(statistics.n_bitset_containers, 0);
    assert_eq!(statistics.n_run_containers, 1);
    assert_eq!(bitmap.len(), 60_000);
    assert!(serialized_size(&bitmap) < 100);
}

#[test]
fn fragmented_batch_is_demoted() {
    let mut bitmap = RoaringBitmap::from_iter(0..1_000);
    assert_eq!(bitmap.statistics().n_run_containers, 1);

    bitmap.extend((10_000..65_000).step_by(4));

    let statistics = bitmap.statistics();
    assert_eq!(statistics.n_run_containers, 0);
    assert_eq!(statistics.n_bitset_containers, 1);
    assert!(serialized_size(&bitmap) < 2 * 8192);
}

#[test]
fn extend_matches_repeated_insert() {
    let values =
        (0..100_000u32).map(|i| i.wrapping_mul(2_654_435_761) % 200_000).collect::<Vec<_>>();
    let mut extended = RoaringBitmap::new();
    extended.extend(values.iter().copied());

    let mut inserted = RoaringBitmap::new();
    for &value in &values {
        inserted.insert(value);
    }

    assert_eq!(extended, inserted);
    assert_eq!(extended.statistics().n_containers, inserted.statistics().n_containers);
}
