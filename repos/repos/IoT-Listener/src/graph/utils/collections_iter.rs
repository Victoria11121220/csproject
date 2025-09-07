use crate::graph::types::{GraphPayloadCollections, GraphPayloadObjects};

/// Iterator over the payload collections.
/// It returns a collection of payload objects for each iteration.
/// This is equivalent to taking vertical slices across all collections and returning them as payload objects.
/// The iterator assumes that all collections have the same length.
/// If the collections have different lengths, the iterator will stop at the shortest collection.
pub struct PayloadCollectionsIterator<'a> {
    payload_collections: &'a GraphPayloadCollections,
    current_idx: usize,
    len: usize,
}

impl<'a> PayloadCollectionsIterator<'a> {
    pub fn new(payload: &'a GraphPayloadCollections) -> Self {
        let min_len = payload.values().map(|x| x.len()).min().unwrap_or(0);

        PayloadCollectionsIterator {
            payload_collections: payload,
            current_idx: 0,
            len: min_len,
        }
    }
}

impl Iterator for PayloadCollectionsIterator<'_> {
    type Item = GraphPayloadObjects;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current_idx >= self.len {
            return None;
        }

        let mut payload_objects = GraphPayloadObjects::new();
        for (input_handle, collection) in self.payload_collections.iter() {
            payload_objects.insert(input_handle.clone(), collection[self.current_idx].clone());
        }

        self.current_idx += 1;
        Some(payload_objects)
    }
}

#[cfg_attr(coverage_nightly, coverage(off))]
#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::types::NodeDataObject;

    #[test]
    fn test_payload_collections_iterator() {
        let mut payload_collections = GraphPayloadCollections::new();
        payload_collections.insert(
            "input1".to_string(),
            [1., 2., 3.].iter().map(|x| NodeDataObject::from(*x)).collect(),
        );
        payload_collections.insert(
            "input2".to_string(),
            [1., 2., 3.].iter().map(|x| NodeDataObject::from(*x)).collect(),
        );

        let payload_collections_iterator = PayloadCollectionsIterator::new(&payload_collections);
        for payload_objects in payload_collections_iterator {
            assert_eq!(payload_objects.len(), 2);
            assert_eq!(payload_objects.get("input1").unwrap(), payload_objects.get("input2").unwrap());
        }
    }

    #[test]
    fn test_uneven_collections() {
        let mut payload_collections = GraphPayloadCollections::new();
        payload_collections.insert(
            "input1".to_string(),
            [1., 2., 3.].iter().map(|x| NodeDataObject::from(*x)).collect(),
        );
        payload_collections.insert("input2".to_string(), [1.].iter().map(|x| NodeDataObject::from(*x)).collect());

        let payload_collections_iterator = PayloadCollectionsIterator::new(&payload_collections);
        assert!(payload_collections_iterator.count() == 1);
    }
}