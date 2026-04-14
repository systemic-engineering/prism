//! Store — where crystals live.

#[cfg(test)]
mod tests {
    use super::*;
    use crate::oid::{Oid, Addressable};
    use crate::crystal::Crystal;
    use crate::luminosity::Luminosity;
    use terni::{Imperfect, Loss};
    use std::collections::HashMap;

    #[derive(Debug, Clone, PartialEq)]
    struct TestPrism(Vec<u8>);

    impl Addressable for TestPrism {
        fn oid(&self) -> Oid {
            Oid::hash(&self.0)
        }
    }

    /// In-memory store for testing.
    struct MemoryStore {
        data: HashMap<Oid, Vec<u8>>,
    }

    #[derive(Debug, Clone, Default, PartialEq)]
    struct MemoryLoss(u32);

    impl Loss for MemoryLoss {
        fn zero() -> Self { MemoryLoss(0) }
        fn total() -> Self { MemoryLoss(u32::MAX) }
        fn is_zero(&self) -> bool { self.0 == 0 }
        fn combine(self, other: Self) -> Self { MemoryLoss(self.0 + other.0) }
    }

    impl Store for MemoryStore {
        type Error = String;
        type L = MemoryLoss;

        fn get(&self, oid: &Oid) -> Imperfect<Vec<u8>, Self::Error, Self::L> {
            match self.data.get(oid) {
                Some(data) => Imperfect::Success(data.clone()),
                None => Imperfect::Failure(
                    format!("not found: {:?}", oid),
                    MemoryLoss::zero(),
                ),
            }
        }

        fn put(&mut self, oid: Oid, data: Vec<u8>) -> Imperfect<Oid, Self::Error, Self::L> {
            self.data.insert(oid.clone(), data);
            Imperfect::Success(oid)
        }

        fn has(&self, oid: &Oid) -> Imperfect<Luminosity, Self::Error, Self::L> {
            if self.data.contains_key(oid) {
                Imperfect::Success(Luminosity::Light)
            } else {
                Imperfect::Success(Luminosity::Dark)
            }
        }
    }

    #[test]
    fn store_put_get_roundtrip() {
        let mut store = MemoryStore { data: HashMap::new() };
        let prism = TestPrism(b"hello".to_vec());
        let oid = prism.oid();

        let put_result = store.put(oid.clone(), prism.0.clone());
        assert!(put_result.is_ok());

        let get_result = store.get(&oid);
        assert!(get_result.is_ok());
        assert_eq!(get_result.ok(), Some(b"hello".to_vec()));
    }

    #[test]
    fn store_get_missing_is_failure() {
        let store = MemoryStore { data: HashMap::new() };
        let result = store.get(&Oid::hash(b"nonexistent"));
        assert!(result.is_err());
    }

    #[test]
    fn store_has_returns_luminosity() {
        let mut store = MemoryStore { data: HashMap::new() };
        let oid = Oid::hash(b"test");

        let before = store.has(&oid);
        assert_eq!(before.ok(), Some(Luminosity::Dark));

        store.put(oid.clone(), b"data".to_vec());

        let after = store.has(&oid);
        assert_eq!(after.ok(), Some(Luminosity::Light));
    }
}
