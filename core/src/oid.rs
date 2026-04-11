/// Content address. The identity of a thing is its content.
/// Two values with the same bytes have the same Oid.
/// Oids are the nodes in every graph this system builds.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Oid(String);

impl Oid {
    pub fn new(s: impl Into<String>) -> Self {
        Oid(s.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for Oid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AsRef<str> for Oid {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl From<&str> for Oid {
    fn from(s: &str) -> Self {
        Oid(s.to_owned())
    }
}

impl From<String> for Oid {
    fn from(s: String) -> Self {
        Oid(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn new_and_as_str() {
        let oid = Oid::new("abc123");
        assert_eq!(oid.as_str(), "abc123");
    }

    #[test]
    fn display() {
        let oid = Oid::new("hello");
        assert_eq!(format!("{}", oid), "hello");
    }

    #[test]
    fn equality() {
        let a = Oid::new("foo");
        let b = Oid::new("foo");
        let c = Oid::new("bar");
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn ordering() {
        let a = Oid::new("apple");
        let b = Oid::new("banana");
        assert!(a < b);
        assert!(b > a);
    }

    #[test]
    fn from_str() {
        let oid: Oid = "test".into();
        assert_eq!(oid.as_str(), "test");
    }

    #[test]
    fn from_string() {
        let oid: Oid = String::from("owned").into();
        assert_eq!(oid.as_str(), "owned");
    }

    #[test]
    fn as_ref() {
        let oid = Oid::new("reftest");
        let s: &str = oid.as_ref();
        assert_eq!(s, "reftest");
    }

    #[test]
    fn hash_set_dedup() {
        let mut set = HashSet::new();
        set.insert(Oid::new("x"));
        set.insert(Oid::new("x"));
        set.insert(Oid::new("y"));
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn clone() {
        let a = Oid::new("original");
        let b = a.clone();
        assert_eq!(a, b);
    }
}
