use crate::oid::Oid;

/// A type that has a content address.
pub trait ContentAddressed {
    fn oid(&self) -> Oid;
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Thing {
        id: String,
    }

    impl ContentAddressed for Thing {
        fn oid(&self) -> Oid {
            Oid::new(format!("thing:{}", self.id))
        }
    }

    #[test]
    fn content_addressed_impl() {
        let t = Thing { id: "abc".to_owned() };
        assert_eq!(t.oid().as_str(), "thing:abc");
    }
}
