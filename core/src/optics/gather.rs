//! Gather — strategies for collapsing `Vec<T>` into `T`.
//!
//! Different strategies make different decisions about how to aggregate
//! values. Used by Traversal and Fold optics via smap to collapse
//! multi-element results.

/// A strategy for collapsing `Vec<T>` into a single `T`.
pub trait Gather<T> {
    fn gather(&self, items: Vec<T>) -> T;
}

/// Gather by concatenating string results.
#[derive(Clone)]
pub struct ConcatGather;

impl Gather<String> for ConcatGather {
    fn gather(&self, items: Vec<String>) -> String {
        items.concat()
    }
}

/// Gather by summing via `std::ops::Add`. Generic over any type that
/// has a zero element (Default) and addition.
#[derive(Clone)]
pub struct AddGather;

impl<T> Gather<T> for AddGather
where
    T: Clone + Default + std::ops::Add<Output = T>,
{
    fn gather(&self, items: Vec<T>) -> T {
        items.into_iter().fold(T::default(), |acc, x| acc + x)
    }
}

/// Gather by taking the first element. Returns `T::default()` if empty.
#[derive(Clone)]
pub struct FirstGather;

impl<T: Clone + Default> Gather<T> for FirstGather {
    fn gather(&self, items: Vec<T>) -> T {
        items.into_iter().next().unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn concat_gather_joins_strings() {
        let g = ConcatGather;
        let result = g.gather(vec!["hello".to_string(), " ".to_string(), "world".to_string()]);
        assert_eq!(result, "hello world");
    }

    #[test]
    fn concat_gather_empty() {
        let g = ConcatGather;
        let result = g.gather(vec![]);
        assert_eq!(result, "");
    }

    #[test]
    fn add_gather_sums_i32() {
        let g = AddGather;
        let result = g.gather(vec![10, 20, 30]);
        assert_eq!(result, 60);
    }

    #[test]
    fn add_gather_empty() {
        let g = AddGather;
        let result: i32 = g.gather(vec![]);
        assert_eq!(result, 0);
    }

    #[test]
    fn first_gather_takes_first() {
        let g = FirstGather;
        let result = g.gather(vec!["first".to_string(), "second".to_string()]);
        assert_eq!(result, "first");
    }

    #[test]
    fn first_gather_empty() {
        let g = FirstGather;
        let result: String = g.gather(vec![]);
        assert_eq!(result, "");
    }

    #[test]
    fn add_gather_with_f64() {
        let g = AddGather;
        let result = g.gather(vec![1.5, 2.5, 3.0]);
        assert_eq!(result, 7.0);
    }

    #[test]
    fn add_gather_extensible_to_user_types() {
        #[derive(Clone, Debug, PartialEq, Default)]
        struct Points(i32);

        impl std::ops::Add for Points {
            type Output = Self;
            fn add(self, other: Self) -> Self {
                Points(self.0 + other.0)
            }
        }

        let g = AddGather;
        let result = g.gather(vec![Points(10), Points(20), Points(30)]);
        assert_eq!(result, Points(60));
    }

    #[test]
    fn first_gather_takes_first_i32() {
        let g = FirstGather;
        let result = g.gather(vec![42, 99]);
        assert_eq!(result, 42);
    }
}
