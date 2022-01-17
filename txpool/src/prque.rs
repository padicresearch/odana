use std::cmp::Ordering;
use std::collections::{BinaryHeap, BTreeSet};
use std::hash::{Hash, Hasher};

struct Element<Value> where Value: Eq {
    value: Value,
    priority: i64,
}

impl<Value> Element<Value> where Value: Eq {
    fn new(value: Value, priority: i64) -> Self {
        Self {
            value,
            priority,
        }
    }
}

impl<Value> Ord for Element<Value> where Value: Eq {
    fn cmp(&self, other: &Self) -> Ordering {
        self.priority.cmp(&other.priority)
    }
}


impl<Value> PartialOrd for Element<Value> where Value: Eq {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.priority.partial_cmp(&other.priority)
    }
}


impl<Value> Eq for Element<Value> where Value: Eq {}

impl<Value> PartialEq for Element<Value> where Value: Eq {
    fn eq(&self, other: &Self) -> bool {
        self.value.eq(&other.value)
    }
}

pub struct PriorityQueue<V> where V: Eq {
    heap: BinaryHeap<Element<V>>,
}

impl<V> PriorityQueue<V> where V: Eq {
    pub fn new() -> Self {
        Self {
            heap: Default::default()
        }
    }
    pub fn push(&mut self, value: V, priority: i64) {
        let element = Element::new(value, priority);
        self.heap.push(element);
    }

    pub fn pop(&mut self) -> Option<(V, i64)> {
        let el = self.heap.pop();
        el.map(|el| (el.value, el.priority))
    }

    pub fn len(&self) -> usize {
        self.heap.len()
    }

    pub fn is_empty(&self) -> bool {
        self.heap.is_empty()
    }

}

#[cfg(test)]
mod test {
    use crate::prque::PriorityQueue;

    #[test]
    fn test_prque() {
        let mut offenders = PriorityQueue::new();
        offenders.push("alice", 10);
        offenders.push("bob", 30);
        offenders.push("bob", 20);
        offenders.push("jake", 1);

        let expected = vec![("bob", 30), ("bob", 20), ("alice", 10), ("jake", 1)];
        let mut got = Vec::with_capacity(offenders.len());
        while let Some((name, priority)) = offenders.pop() {
            got.push((name, priority))
        }

        assert_eq!(got, expected)
    }
}