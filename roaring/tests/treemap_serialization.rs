#![cfg(feature = "std")]

use roaring::RoaringTreemap;

fn serialize_deserialize<Dataset, I>(dataset: Dataset)
where
    Dataset: IntoIterator<Item = u64, IntoIter = I>,
    I: Iterator<Item = u64>,
{
    let rb = RoaringTreemap::from_iter(dataset);

    let mut buffer = vec![];
    rb.serialize_into(&mut buffer).unwrap();

    assert_eq!(buffer.len(), rb.serialized_size());

    let new_rb = RoaringTreemap::deserialize_from(&buffer[..]).unwrap();

    assert_eq!(rb, new_rb);
}

#[test]
fn empty() {
    serialize_deserialize(vec![])
}

#[test]
fn basic() {
    serialize_deserialize(vec![1, 2, 3, 4, 5, 100, 1000])
}

#[test]
fn basic_2() {
    serialize_deserialize(vec![1, 2, 3, 4, 5, 100, 1000, 10000, 100000, 1000000])
}

#[test]
fn basic_3() {
    let u32max = u32::MAX as u64;
    serialize_deserialize(
        vec![1, 2, 3, 4, 5, 100, 1000, 10000, 100000, 1000000, u32max + 10, u32max << 10]
            .into_iter()
            .chain(u32max..(u32max + 2 * (1 << 16))),
    )
}

// The treemap on-disk format prefixes every bitmap with a little-endian u32 high-bits key and
// starts with a little-endian u64 entry count. This pins that contract at the byte level so a
// native-endian regression cannot break cross-platform synchronization.
#[test]
fn header_and_keys_are_serialized_little_endian() {
    // A single entry whose high 32 bits are 0x0100_0000, exercising non-zero key bytes in both
    // orders. The inner bitmap contains one value in its first array container.
    let treemap = RoaringTreemap::from_iter([0x0100_0000_0000_0001u64]);

    let mut buffer = vec![];
    treemap.serialize_into(&mut buffer).unwrap();

    #[rustfmt::skip]
    let expected_prefix = [
        0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // entry count: 1 (little-endian u64)
        0x00, 0x00, 0x00, 0x01, // high bits key: 0x0100_0000 (little-endian u32)
        // Inner RoaringBitmap: no-run cookie, one container, key 0, cardinality 1, offset 16,
        // followed by the single little-endian u16 value 0x0001.
        0x3A, 0x30, 0x00, 0x00,
        0x01, 0x00, 0x00, 0x00,
        0x00, 0x00,
        0x00, 0x00,
        0x10, 0x00, 0x00, 0x00,
        0x01, 0x00,
    ];
    assert_eq!(buffer, expected_prefix);
    assert_eq!(RoaringTreemap::deserialize_from(&buffer[..]).unwrap(), treemap);
}
