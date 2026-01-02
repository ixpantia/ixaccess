use std::hash::Hash;
use std::hash::Hasher;
use std::num::NonZeroU32;

#[derive(
    Copy, Clone, Debug, PartialEq, Eq, bincode::Encode, bincode::Decode, Hash, PartialOrd, Ord,
)]
pub struct Index(NonZeroU32);

impl Index {
    pub(crate) fn from_usize_index(v: usize) -> Index {
        let v = (v + 1) as u32;
        Index(unsafe { NonZeroU32::new_unchecked(v) })
    }
    #[inline(always)]
    pub fn into_usize(self) -> usize {
        self.0.get() as usize
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, bincode::Encode, bincode::Decode)]
struct StringLocation {
    start: u32,
    end: u32,
}

const BUCKETS: usize = 128;

#[derive(Debug)]
pub struct Interner {
    // String -> Index
    buckets: [Vec<(StringLocation, usize)>; BUCKETS],
    // Index -> String
    strings: Vec<StringLocation>,
    arena: Vec<u8>,
}

impl bincode::Encode for Interner {
    fn encode<E: bincode::enc::Encoder>(
        &self,
        encoder: &mut E,
    ) -> core::result::Result<(), bincode::error::EncodeError> {
        bincode::Encode::encode(&self.strings, encoder)?;
        bincode::Encode::encode(&self.arena, encoder)?;
        core::result::Result::Ok(())
    }
}
impl<Context> bincode::Decode<Context> for Interner {
    fn decode<D: bincode::de::Decoder<Context = Context>>(
        decoder: &mut D,
    ) -> Result<Self, bincode::error::DecodeError> {
        let strings: Vec<StringLocation> = bincode::Decode::decode(decoder)?;
        let arena: Vec<u8> = bincode::Decode::decode(decoder)?;

        let mut buckets: [Vec<(StringLocation, usize)>; BUCKETS] =
            std::array::from_fn(|_| Vec::new());

        // Rebuild the bucket index from the strings
        for (index, &location) in strings.iter().enumerate() {
            let val_str = unsafe {
                std::str::from_utf8_unchecked(
                    &arena[location.start as usize..location.end as usize],
                )
            };
            let mut hasher = std::hash::DefaultHasher::new();
            val_str.hash(&mut hasher);
            let hash = hasher.finish();
            let bucket_index = hash as usize % BUCKETS;
            buckets[bucket_index].push((location, index));
        }

        Ok(Self {
            buckets,
            strings,
            arena,
        })
    }
}

impl Interner {
    pub fn new() -> Self {
        Self {
            buckets: std::array::from_fn(|_| Vec::new()),
            strings: Vec::new(),
            arena: Vec::new(),
        }
    }

    fn get_str(&self, location: StringLocation) -> &str {
        unsafe {
            std::str::from_utf8_unchecked(
                &self.arena[location.start as usize..location.end as usize],
            )
        }
    }

    pub fn len(&self) -> usize {
        self.strings.len()
    }

    pub fn resolve(&self, index: Index) -> Option<&str> {
        self.strings
            .get((index.0.get() - 1) as usize)
            .map(|&l| self.get_str(l))
    }

    fn alloc(&mut self, val: &str) -> (StringLocation, usize) {
        let start = self.arena.len() as u32;
        let end = start + val.len() as u32;
        let location = StringLocation { start, end };
        self.arena.extend_from_slice(val.as_bytes());
        let index = self.strings.len();
        self.strings.push(location);
        (location, index)
    }
    pub fn get_or_intern(&mut self, val: impl AsRef<str>) -> Index {
        let val_str = val.as_ref();
        let mut hasher = std::hash::DefaultHasher::new();
        val_str.hash(&mut hasher);
        let hash = hasher.finish();
        let bucket_index = hash as usize % BUCKETS;
        let index = self.buckets[bucket_index].iter().find_map(|(v, index)| {
            (val_str == self.get_str(*v)).then_some(Index::from_usize_index(*index))
        });
        match index {
            Some(i) => i,
            None => {
                let (location, index) = self.alloc(val_str);
                self.buckets[bucket_index].push((location, index));
                Index::from_usize_index(index)
            }
        }
    }
    pub fn get(&self, val: impl AsRef<str>) -> Option<Index> {
        let val_str = val.as_ref();
        let mut hasher = std::hash::DefaultHasher::new();
        val_str.hash(&mut hasher);
        let hash = hasher.finish();
        let bucket_index = hash as usize % BUCKETS;
        let bucket = &self.buckets[bucket_index];
        bucket.iter().find_map(|(v, index)| {
            (val_str == self.get_str(*v)).then_some(Index::from_usize_index(*index))
        })
    }
    pub fn strings<'a>(&'a self) -> impl Iterator<Item = &'a str> {
        self.strings.iter().map(|&sl| self.get_str(sl))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quickcheck::TestResult;
    use quickcheck_macros::quickcheck;

    #[test]
    fn test_interner_new() {
        let interner = Interner::new();
        assert_eq!(interner.strings.len(), 0);
        assert_eq!(interner.arena.len(), 0);
    }

    #[test]
    fn test_get_or_intern_single_string() {
        let mut interner = Interner::new();
        let index = interner.get_or_intern("hello");
        assert_eq!(index, Index::from_usize_index(0));
        assert_eq!(interner.resolve(index), Some("hello"));
    }

    #[test]
    fn test_get_or_intern_returns_same_index() {
        let mut interner = Interner::new();
        let index1 = interner.get_or_intern("hello");
        let index2 = interner.get_or_intern("hello");
        assert_eq!(index1, index2);
    }

    #[test]
    fn test_get_or_intern_multiple_strings() {
        let mut interner = Interner::new();
        let index1 = interner.get_or_intern("hello");
        let index2 = interner.get_or_intern("world");
        let index3 = interner.get_or_intern("rust");

        assert_eq!(interner.resolve(index1), Some("hello"));
        assert_eq!(interner.resolve(index2), Some("world"));
        assert_eq!(interner.resolve(index3), Some("rust"));
    }

    #[test]
    fn test_get_existing_string() {
        let mut interner = Interner::new();
        let index = interner.get_or_intern("hello");
        let found_index = interner.get("hello").unwrap();
        assert_eq!(index, found_index);
    }

    #[test]
    fn test_get_nonexistent_string() {
        let interner = Interner::new();
        let result = interner.get("nonexistent");
        assert!(result.is_none());
    }

    #[test]
    fn test_get_after_intern() {
        let mut interner = Interner::new();
        interner.get_or_intern("hello");
        interner.get_or_intern("world");

        let index = interner.get("hello").unwrap();
        assert_eq!(interner.resolve(index), Some("hello"));
    }

    #[test]
    fn test_empty_string() {
        let mut interner = Interner::new();
        let index = interner.get_or_intern("");
        assert_eq!(interner.resolve(index), Some(""));
    }

    #[test]
    fn test_unicode_strings() {
        let mut interner = Interner::new();
        let index1 = interner.get_or_intern("こんにちは");
        let index2 = interner.get_or_intern("🦀");

        assert_eq!(interner.resolve(index1), Some("こんにちは"));
        assert_eq!(interner.resolve(index2), Some("🦀"));
    }

    #[test]
    fn test_many_strings() {
        let mut interner = Interner::new();
        let mut indices = Vec::new();

        for i in 0..1000 {
            let s = format!("string_{}", i);
            let index = interner.get_or_intern(&s);
            indices.push((index, s));
        }

        for (index, s) in indices {
            assert_eq!(interner.resolve(index), Some(s.as_str()));
        }
    }

    #[test]
    fn test_serialize_deserialize_empty() {
        let interner = Interner::new();
        let encoded = bincode::encode_to_vec(&interner, bincode::config::standard()).unwrap();
        let (deserialized, _): (Interner, usize) =
            bincode::decode_from_slice(&encoded, bincode::config::standard()).unwrap();

        assert_eq!(deserialized.strings.len(), 0);
        assert_eq!(deserialized.arena.len(), 0);
    }

    #[test]
    fn test_serialize_deserialize_single_string() {
        let mut interner = Interner::new();
        let index = interner.get_or_intern("hello");

        let encoded = bincode::encode_to_vec(&interner, bincode::config::standard()).unwrap();
        let (deserialized, _): (Interner, usize) =
            bincode::decode_from_slice(&encoded, bincode::config::standard()).unwrap();

        assert_eq!(deserialized.strings.len(), 1);
        assert_eq!(deserialized.resolve(index), Some("hello"));
    }

    #[test]
    fn test_serialize_deserialize_multiple_strings() {
        let mut interner = Interner::new();
        let index1 = interner.get_or_intern("hello");
        let index2 = interner.get_or_intern("world");
        let index3 = interner.get_or_intern("rust");

        let encoded = bincode::encode_to_vec(&interner, bincode::config::standard()).unwrap();
        let (deserialized, _): (Interner, usize) =
            bincode::decode_from_slice(&encoded, bincode::config::standard()).unwrap();

        assert_eq!(deserialized.strings.len(), 3);
        assert_eq!(deserialized.resolve(index1), Some("hello"));
        assert_eq!(deserialized.resolve(index2), Some("world"));
        assert_eq!(deserialized.resolve(index3), Some("rust"));
    }

    #[test]
    fn test_serialize_deserialize_get_works() {
        let mut interner = Interner::new();
        interner.get_or_intern("hello");
        interner.get_or_intern("world");

        let encoded = bincode::encode_to_vec(&interner, bincode::config::standard()).unwrap();
        let (deserialized, _): (Interner, usize) =
            bincode::decode_from_slice(&encoded, bincode::config::standard()).unwrap();

        let found = deserialized.get("hello").unwrap();
        assert_eq!(deserialized.resolve(found), Some("hello"));

        let found2 = deserialized.get("world").unwrap();
        assert_eq!(deserialized.resolve(found2), Some("world"));
    }

    #[test]
    fn test_serialize_deserialize_unicode() {
        let mut interner = Interner::new();
        let index1 = interner.get_or_intern("こんにちは");
        let index2 = interner.get_or_intern("🦀");

        let encoded = bincode::encode_to_vec(&interner, bincode::config::standard()).unwrap();
        let (deserialized, _): (Interner, usize) =
            bincode::decode_from_slice(&encoded, bincode::config::standard()).unwrap();

        assert_eq!(deserialized.resolve(index1), Some("こんにちは"));
        assert_eq!(deserialized.resolve(index2), Some("🦀"));
    }

    #[test]
    fn test_serialize_deserialize_empty_string() {
        let mut interner = Interner::new();
        let index = interner.get_or_intern("");

        let encoded = bincode::encode_to_vec(&interner, bincode::config::standard()).unwrap();
        let (deserialized, _): (Interner, usize) =
            bincode::decode_from_slice(&encoded, bincode::config::standard()).unwrap();

        assert_eq!(deserialized.resolve(index), Some(""));
    }

    #[test]
    fn test_serialize_deserialize_many_strings() {
        let mut interner = Interner::new();
        let mut indices = Vec::new();

        for i in 0..100 {
            let s = format!("string_{}", i);
            let index = interner.get_or_intern(&s);
            indices.push((index, s));
        }

        let encoded = bincode::encode_to_vec(&interner, bincode::config::standard()).unwrap();
        let (deserialized, _): (Interner, usize) =
            bincode::decode_from_slice(&encoded, bincode::config::standard()).unwrap();

        for (index, s) in indices {
            assert_eq!(deserialized.resolve(index), Some(s.as_str()));
        }
    }

    // Property-based tests
    #[quickcheck]
    fn prop_intern_and_resolve_roundtrip(s: String) -> bool {
        let mut interner = Interner::new();
        let index = interner.get_or_intern(&s);
        interner.resolve(index) == Some(s.as_str())
    }

    #[quickcheck]
    fn prop_same_string_same_index(s: String) -> bool {
        let mut interner = Interner::new();
        let index1 = interner.get_or_intern(&s);
        let index2 = interner.get_or_intern(&s);
        index1 == index2
    }

    #[quickcheck]
    fn prop_different_strings_different_indices(s1: String, s2: String) -> TestResult {
        if s1 == s2 {
            return TestResult::discard();
        }
        let mut interner = Interner::new();
        let index1 = interner.get_or_intern(&s1);
        let index2 = interner.get_or_intern(&s2);
        TestResult::from_bool(index1 != index2)
    }

    #[quickcheck]
    fn prop_get_returns_interned_string(s: String) -> bool {
        let mut interner = Interner::new();
        let index1 = interner.get_or_intern(&s);
        let index2 = interner.get(&s);
        index2 == Some(index1)
    }

    #[quickcheck]
    fn prop_get_returns_none_for_nonexistent(s1: String, s2: String) -> TestResult {
        if s1 == s2 {
            return TestResult::discard();
        }
        let mut interner = Interner::new();
        interner.get_or_intern(&s1);
        TestResult::from_bool(interner.get(&s2).is_none())
    }

    #[quickcheck]
    fn prop_serialize_deserialize_preserves_strings(strings: Vec<String>) -> bool {
        let mut interner = Interner::new();
        let mut indices = Vec::new();

        for s in &strings {
            let index = interner.get_or_intern(s);
            indices.push(index);
        }

        let encoded = bincode::encode_to_vec(&interner, bincode::config::standard()).unwrap();
        let (deserialized, _): (Interner, usize) =
            bincode::decode_from_slice(&encoded, bincode::config::standard()).unwrap();

        for (index, s) in indices.iter().zip(strings.iter()) {
            if deserialized.resolve(*index) != Some(s.as_str()) {
                return false;
            }
        }
        true
    }

    #[quickcheck]
    fn prop_serialize_deserialize_get_works(strings: Vec<String>) -> bool {
        let mut interner = Interner::new();

        for s in &strings {
            interner.get_or_intern(s);
        }

        let encoded = bincode::encode_to_vec(&interner, bincode::config::standard()).unwrap();
        let (deserialized, _): (Interner, usize) =
            bincode::decode_from_slice(&encoded, bincode::config::standard()).unwrap();

        for s in &strings {
            match deserialized.get(s) {
                Some(index) => {
                    if deserialized.resolve(index) != Some(s.as_str()) {
                        return false;
                    }
                }
                None => return false,
            }
        }
        true
    }

    #[quickcheck]
    fn prop_arena_size_grows_monotonically(strings: Vec<String>) -> bool {
        let mut interner = Interner::new();
        let mut prev_size = 0;

        for s in &strings {
            interner.get_or_intern(s);
            let current_size = interner.arena.len();
            if current_size < prev_size {
                return false;
            }
            prev_size = current_size;
        }
        true
    }

    #[quickcheck]
    fn prop_interning_order_independent(strings: Vec<String>) -> bool {
        if strings.is_empty() {
            return true;
        }

        let mut interner1 = Interner::new();
        let mut interner2 = Interner::new();

        // Intern in original order
        for s in &strings {
            interner1.get_or_intern(s);
        }

        // Intern in reverse order
        for s in strings.iter().rev() {
            interner2.get_or_intern(s);
        }

        // Check that get returns the same strings (indices may differ)
        for s in &strings {
            let idx1 = interner1.get(s);
            let idx2 = interner2.get(s);

            match (idx1, idx2) {
                (Some(i1), Some(i2)) => {
                    if interner1.resolve(i1) != interner2.resolve(i2) {
                        return false;
                    }
                }
                _ => return false,
            }
        }
        true
    }

    #[quickcheck]
    fn prop_multiple_interns_dont_increase_count(s: String, n: u8) -> bool {
        let mut interner = Interner::new();
        let n = n.max(1) as usize; // At least 1 iteration

        for _ in 0..n {
            interner.get_or_intern(&s);
        }

        interner.strings.len() == 1
    }

    #[quickcheck]
    fn prop_unique_strings_count_matches(strings: Vec<String>) -> bool {
        use std::collections::HashSet;

        let mut interner = Interner::new();
        let unique: HashSet<_> = strings.iter().collect();

        for s in &strings {
            interner.get_or_intern(s);
        }

        interner.strings.len() == unique.len()
    }

    #[quickcheck]
    fn prop_deserialized_interner_can_intern_new_strings(
        initial_strings: Vec<String>,
        new_string: String,
    ) -> TestResult {
        // Make sure new_string is not in initial_strings
        if initial_strings.contains(&new_string) {
            return TestResult::discard();
        }

        let mut interner = Interner::new();
        for s in &initial_strings {
            interner.get_or_intern(s);
        }

        let encoded = bincode::encode_to_vec(&interner, bincode::config::standard()).unwrap();
        let (mut deserialized, _): (Interner, usize) =
            bincode::decode_from_slice(&encoded, bincode::config::standard()).unwrap();

        let index = deserialized.get_or_intern(&new_string);
        TestResult::from_bool(deserialized.resolve(index) == Some(new_string.as_str()))
    }

    #[quickcheck]
    fn prop_empty_strings_handled_correctly(empty_count: u8) -> bool {
        let mut interner = Interner::new();
        let count = empty_count.max(1) as usize;

        for _ in 0..count {
            interner.get_or_intern("");
        }

        // Should only have one empty string interned
        interner.strings.len() == 1 && interner.resolve(Index::from_usize_index(0)) == Some("")
    }

    #[quickcheck]
    fn prop_whitespace_strings_preserved(s: String) -> TestResult {
        // Only test strings that are whitespace
        if !s.chars().all(|c| c.is_whitespace()) || s.is_empty() {
            return TestResult::discard();
        }

        let mut interner = Interner::new();
        let index = interner.get_or_intern(&s);
        TestResult::from_bool(interner.resolve(index) == Some(s.as_str()))
    }

    #[quickcheck]
    fn prop_serialize_preserves_arena_structure(strings: Vec<String>) -> bool {
        let mut interner = Interner::new();

        for s in &strings {
            interner.get_or_intern(s);
        }

        let encoded = bincode::encode_to_vec(&interner, bincode::config::standard()).unwrap();
        let (deserialized, _): (Interner, usize) =
            bincode::decode_from_slice(&encoded, bincode::config::standard()).unwrap();

        // Arena should be identical
        interner.arena == deserialized.arena
    }

    #[test]
    fn test_strings_with_null_bytes_in_content() {
        // Now that we serialize string metadata, null bytes are handled correctly
        let mut interner = Interner::new();
        let s = "hello\0world";
        let index = interner.get_or_intern(s);
        let resolved = interner.resolve(index);
        assert_eq!(resolved, Some("hello\0world"));

        // Serialization should preserve null bytes correctly
        let encoded = bincode::encode_to_vec(&interner, bincode::config::standard()).unwrap();
        let (deserialized, _): (Interner, usize) =
            bincode::decode_from_slice(&encoded, bincode::config::standard()).unwrap();
        assert_eq!(deserialized.strings.len(), 1);
        assert_eq!(deserialized.resolve(index), Some("hello\0world"));

        // Get should also work
        let found = deserialized.get(s).unwrap();
        assert_eq!(deserialized.resolve(found), Some(s));
    }

    #[test]
    fn test_very_long_string() {
        let mut interner = Interner::new();
        let long_string = "a".repeat(100_000);
        let index = interner.get_or_intern(&long_string);
        assert_eq!(interner.resolve(index), Some(long_string.as_str()));
    }

    #[test]
    fn test_special_characters() {
        let mut interner = Interner::new();
        let special = vec!["\n\r\t", "\\n\\r\\t", "\"'`", "<>&", "{}[]", "!@#$%^&*()"];

        for s in &special {
            let index = interner.get_or_intern(s);
            assert_eq!(interner.resolve(index), Some(*s));
        }
    }

    #[test]
    fn test_mixed_unicode_and_ascii() {
        let mut interner = Interner::new();
        let mixed = vec!["hello世界", "🦀Rust💻", "Émoji🎉test", "αβγδε123"];

        for s in &mixed {
            let index = interner.get_or_intern(s);
            assert_eq!(interner.resolve(index), Some(*s));
        }
    }

    #[test]
    fn test_interleaved_intern_and_get() {
        let mut interner = Interner::new();

        let idx1 = interner.get_or_intern("first");
        assert_eq!(interner.get("first"), Some(idx1));

        let idx2 = interner.get_or_intern("second");
        assert_eq!(interner.get("first"), Some(idx1));
        assert_eq!(interner.get("second"), Some(idx2));

        let idx3 = interner.get_or_intern("first"); // Re-intern
        assert_eq!(idx1, idx3);
    }

    #[quickcheck]
    fn prop_all_interned_strings_retrievable(strings: Vec<String>) -> bool {
        let mut interner = Interner::new();
        let mut indices = Vec::new();

        for s in &strings {
            let idx = interner.get_or_intern(s);
            indices.push(idx);
        }

        for (idx, s) in indices.iter().zip(strings.iter()) {
            if interner.resolve(*idx) != Some(s.as_str()) {
                return false;
            }
        }
        true
    }

    #[quickcheck]
    fn prop_consecutive_duplicate_interns(s: String, count: u8) -> bool {
        if count == 0 {
            return true;
        }
        let mut interner = Interner::new();
        let first_idx = interner.get_or_intern(&s);

        for _ in 1..count {
            let idx = interner.get_or_intern(&s);
            if idx != first_idx {
                return false;
            }
        }
        true
    }

    #[quickcheck]
    fn prop_strings_remain_valid_after_many_operations(ops: Vec<(bool, String)>) -> bool {
        let mut interner = Interner::new();
        let mut interned = std::collections::HashMap::new();

        for (should_intern, s) in ops {
            if should_intern {
                let idx = interner.get_or_intern(&s);
                interned.insert(s.clone(), idx);
            } else if let Some(&idx) = interned.get(&s) {
                if interner.resolve(idx) != Some(s.as_str()) {
                    return false;
                }
            }
        }
        true
    }
}
