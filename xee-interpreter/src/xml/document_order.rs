// Document order for XML nodes. This maintains both a document id (so we can
// distinguish between nodes from different documents) as well as as a document
// preorder (so we can sort nodes from the same document).
//
// We create annotations on the fly as needed, so that nodes even if
// dynamically created (as happens with XSLT) also get them.
//
// To this end, we do a reverse-preorder traversal of the document tree up to
// the root. From the root (which may not be a Xot Document node) we can
// determine a unique document id.
//
// This means we need to traverse the tree twice: once backwards to get to the
// root and count the nodes, and once forward again to assign the annotations.
//
// As an optimization we stop the traversal as soon as we run into an already
// annotated node. From this we can determine the document id as well as the
// preorder count of this node.

use std::cell::RefCell;

use ahash::{HashMap, HashMapExt};
use xot::Xot;

#[derive(Debug, Clone, Copy, Eq, PartialEq, PartialOrd, Ord, Hash)]
pub(crate) struct DocumentOrder(usize, usize);

impl DocumentOrder {
    pub(crate) fn generate_id(&self) -> String {
        // must be alphanumeric and start with alphabetic character, so we
        // cannot use _ or - as separators
        format!("id{}s{}", self.0, self.1)
    }
}

pub(crate) struct DocumentOrderAccess<'a> {
    pub(crate) xot: &'a Xot,
    pub(crate) annotations: &'a DocumentOrderAnnotations,
}

impl<'a> DocumentOrderAccess<'a> {
    pub(crate) fn new(xot: &'a Xot, annotations: &'a DocumentOrderAnnotations) -> Self {
        Self { xot, annotations }
    }

    pub(crate) fn get(&self, node: xot::Node) -> DocumentOrder {
        self.annotations.get(node, self.xot)
    }
}

#[derive(Debug, Clone)]
pub(crate) struct DocumentOrderAnnotations {
    // each document has a different id, so track this
    document_id: RefCell<usize>,
    map: RefCell<HashMap<xot::Node, DocumentOrder>>,
}

impl DocumentOrderAnnotations {
    pub(crate) fn new() -> Self {
        Self {
            map: RefCell::new(HashMap::new()),
            document_id: RefCell::new(0),
        }
    }

    pub(crate) fn access<'a>(&'a self, xot: &'a Xot) -> DocumentOrderAccess<'a> {
        DocumentOrderAccess::new(xot, self)
    }

    pub(crate) fn get(&self, node: xot::Node, xot: &xot::Xot) -> DocumentOrder {
        let document_order = self.map.borrow().get(&node).cloned();
        if let Some(document_order) = document_order {
            document_order
        } else {
            let (document_order, found_node) =
                find_node_with_document_order(&self.map.borrow(), node, xot);
            if let Some(document_order) = document_order {
                annotation_with_document_order(
                    &mut self.map.borrow_mut(),
                    document_order,
                    found_node,
                    node,
                    xot,
                )
            } else {
                // if we increment document id, we should end up at
                // a new one for this new fragment/document
                *self.document_id.borrow_mut() += 1;

                let document_order = DocumentOrder(*self.document_id.borrow(), 0);

                let mut map = self.map.borrow_mut();
                map.insert(found_node, document_order);
                // now create annotations for everything up to node
                annotation_with_document_order(&mut map, document_order, found_node, node, xot)
            }
        }
    }
}

// this always returns a node; either it's the first node that has a document order
// annotation, or alternatively it's the root node without annotation
fn find_node_with_document_order(
    map: &HashMap<xot::Node, DocumentOrder>,
    node: xot::Node,
    xot: &xot::Xot,
) -> (Option<DocumentOrder>, xot::Node) {
    let mut last_node = node;
    for node in xot.all_reverse_preorder(node) {
        // if we find a node that already has an annotation,
        // we're done
        if let Some(document_order) = map.get(&node) {
            return (Some(*document_order), node);
        }
        last_node = node;
    }
    (None, last_node)
}

fn annotation_with_document_order(
    map: &mut HashMap<xot::Node, DocumentOrder>,
    document_order: DocumentOrder,
    root_node: xot::Node,
    node: xot::Node,
    xot: &Xot,
) -> DocumentOrder {
    if root_node == node {
        // if the root node is the same as the node, we can just return
        // the document order
        return document_order;
    }
    // we know the document order to start with
    let document_id = document_order.0;
    // we need to visit all descendants, then all following nodes
    let mut iter = xot
        .all_descendants(root_node)
        .chain(xot.all_following(root_node));
    // we don't need to revisit the root node itself
    iter.next();
    // so we start one beyond the previous document order
    let start = document_order.1 + 1;
    for (i, descendant) in iter.enumerate() {
        let document_order = DocumentOrder(document_id, start + i);
        map.insert(descendant, document_order);
        if descendant == node {
            return document_order;
        }
    }
    // we should not be able to get here; as following back
    // from the found node should always eventually reach node
    unreachable!()
}
