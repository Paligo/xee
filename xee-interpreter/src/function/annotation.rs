// Annotate a function bytecode array with additional data
// not to be confused with the annotations to XML nodes

#[derive(Debug, Clone)]
pub struct Annotations<T: PartialEq> {
    boundaries: Vec<usize>,
    data: Vec<T>,
}

impl<T: PartialEq> Annotations<T> {
    pub fn new() -> Self {
        Annotations {
            boundaries: vec![0],
            data: Vec::new(),
        }
    }

    pub fn emit(&mut self, size: usize, data: T) {
        // if what we're pushing is the same as the last time
        if self.data.last() == Some(&data) {
            // we just extend the last boundary
            let last = self
                .boundaries
                .last_mut()
                .expect("There should always be a last boundary");
            *last += size;
            return;
        }

        let last = self
            .boundaries
            .last()
            .expect("There should always be a last boundary");
        self.boundaries.push(last + size);
        self.data.push(data);
    }

    pub(crate) fn get(&self, index: usize) -> Option<&T> {
        // use binary search to find the right boundary
        let pos = self
            .boundaries
            .binary_search(&index)
            // if we find the exact position, we have found the data
            // if we find an insertion point, we substract 1. This is
            // always safe as we cannot find 0 as the insertion point
            .unwrap_or_else(|x| x - 1);
        self.data.get(pos)
    }
}

impl Default for Annotations<()> {
    fn default() -> Self {
        Annotations::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, PartialEq)]
    struct DummyData(usize);

    #[test]
    fn test_simple() {
        let mut annotations: Annotations<DummyData> = Annotations::new();
        annotations.emit(4, DummyData(1));

        assert_eq!(annotations.get(0), Some(&DummyData(1)));
        assert_eq!(annotations.get(1), Some(&DummyData(1)));
        assert_eq!(annotations.get(2), Some(&DummyData(1)));
        assert_eq!(annotations.get(3), Some(&DummyData(1)));
        assert_eq!(annotations.get(4), None);
    }

    #[test]
    fn test_multiple() {
        let mut annotations: Annotations<DummyData> = Annotations::new();
        annotations.emit(4, DummyData(1));
        annotations.emit(2, DummyData(2));
        annotations.emit(3, DummyData(3));

        assert_eq!(annotations.get(0), Some(&DummyData(1)));
        assert_eq!(annotations.get(1), Some(&DummyData(1)));
        assert_eq!(annotations.get(2), Some(&DummyData(1)));
        assert_eq!(annotations.get(3), Some(&DummyData(1)));
        assert_eq!(annotations.get(4), Some(&DummyData(2)));
        assert_eq!(annotations.get(5), Some(&DummyData(2)));
        assert_eq!(annotations.get(6), Some(&DummyData(3)));
        assert_eq!(annotations.get(7), Some(&DummyData(3)));
        assert_eq!(annotations.get(8), Some(&DummyData(3)));
        assert_eq!(annotations.get(9), None);
    }

    #[test]
    fn test_extend() {
        let mut annotations: Annotations<DummyData> = Annotations::new();
        annotations.emit(4, DummyData(1));
        annotations.emit(2, DummyData(1));
        annotations.emit(3, DummyData(3));

        // we peek behind the curtain and see we only have 1 copy of DummyData(1)
        assert_eq!(annotations.data.len(), 2);
        assert_eq!(annotations.get(0), Some(&DummyData(1)));
        assert_eq!(annotations.get(1), Some(&DummyData(1)));
        assert_eq!(annotations.get(2), Some(&DummyData(1)));
        assert_eq!(annotations.get(3), Some(&DummyData(1)));
        assert_eq!(annotations.get(4), Some(&DummyData(1)));
        assert_eq!(annotations.get(5), Some(&DummyData(1)));
        assert_eq!(annotations.get(6), Some(&DummyData(3)));
        assert_eq!(annotations.get(7), Some(&DummyData(3)));
        assert_eq!(annotations.get(8), Some(&DummyData(3)));
        assert_eq!(annotations.get(9), None);
    }
}
