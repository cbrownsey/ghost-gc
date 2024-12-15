use ghost_gc::{locked::LockedCell, Arena, Collect, Gc, Mutation, Rootable, UniqueGc};

#[derive(Debug, Clone)]
struct Graph<'b, T>(Vec<Gc<'b, Node<'b, T>>>);

impl<'b, T> Graph<'b, T> {
    fn add_node(&mut self, value: T, parent_idx: Option<usize>, mt: &Mutation<'b>) -> usize
    where
        T: Collect,
    {
        let mut node = UniqueGc::new(
            Node {
                value,
                parent: LockedCell::new(None),
            },
            mt,
        );

        let node = if let Some(parent) = parent_idx.map(|idx| self.0[idx]) {
            *node.parent.get_mut() = Some(parent);
            let node = UniqueGc::into_gc(node);

            node
        } else {
            UniqueGc::into_gc(node)
        };

        self.0.push(node);

        self.0.len() - 1
    }
}

unsafe impl<T: Collect> Collect for Graph<'_, T> {
    const NEEDS_TRACE: bool = T::NEEDS_TRACE;

    fn trace(&self, c: &ghost_gc::Collector) {
        self.0.trace(c);
    }
}

impl<T: Collect> Rootable for Graph<'static, T> {
    type Root<'l> = Graph<'l, T>;
}

#[derive(Debug)]
struct Node<'b, T> {
    value: T,
    parent: LockedCell<Option<Gc<'b, Self>>>,
}

unsafe impl<T: Collect> Collect for Node<'_, T> {
    const NEEDS_TRACE: bool = T::NEEDS_TRACE;

    fn trace(&self, c: &ghost_gc::Collector) {
        self.value.trace(c);
        self.parent.trace(c);
    }
}

#[test]
fn basic() {
    let mut a = Arena::<Graph<'_, i32>>::new(|_| Graph(vec![]));

    a.view_mut(|graph, mt| {
        graph.add_node(0, None, mt);
        graph.add_node(1, Some(0), mt);
        graph.add_node(2, Some(0), mt);
    });

    assert_eq!(a.allocations(), 3);
    a.complete_collection();
    assert_eq!(a.allocations(), 3);

    a.view(|_, mt| {
        let _ = Gc::new(100, mt);
    });

    assert_eq!(a.allocations(), 4);
    a.complete_collection();
    assert_eq!(a.allocations(), 3);
}
