mod bus;
mod graph;
mod link;
mod mir;
mod node;
mod nodes;
mod owner;
mod utils;
pub extern crate derive_ir;

pub use bus::Bus;
pub use derive_ir::Builder;
pub use graph::Graph;
pub use link::{BackLink, Link, Singleton};
pub use mir::Mir;
pub use node::Node;
pub use nodes::*;
pub use owner::Owner;
pub use utils::*;
/// A trait for nodes that can have children
/// This is used with the Child trait to allow for easy traversal and manipulation of the graph
pub trait Parent {
    type Child;
    /// Get a view of the children of the current node.
    fn children(&self) -> Link<Vec<Link<Self::Child>>>;
}

impl<T> Parent for Link<T>
where
    T: Parent,
{
    type Child = T::Child;
    fn children(&self) -> Link<Vec<Link<Self::Child>>> {
        self.borrow().children()
    }
}

impl<T> Parent for BackLink<T>
where
    Link<T>: Parent,
{
    type Child = <Link<T> as Parent>::Child;
    fn children(&self) -> Link<Vec<Link<Self::Child>>> {
        match self.to_link() {
            Some(link) => link.children(),
            None => Link::new(Vec::new()),
        }
    }
}

/// A trait for nodes that can have a parent
/// This is used with the Parent trait to allow for easy traversal and manipulation of the graph
pub trait Child: Clone + Into<Link<Self>> + PartialEq {
    type Parent;
    /// Get a view of the parents of the current node.
    fn get_parents(&self) -> Vec<BackLink<Self::Parent>>;
    /// Add a parent to the current node
    fn add_parent(&mut self, parent: Link<Self::Parent>);
    /// Remove a parent from the current node
    fn remove_parent(&mut self, parent: Link<Self::Parent>);
}

impl<T> Child for Link<T>
where
    T: Child,
{
    type Parent = T::Parent;

    fn get_parents(&self) -> Vec<BackLink<Self::Parent>> {
        self.borrow().get_parents()
    }
    fn add_parent(&mut self, parent: Link<Self::Parent>) {
        self.borrow_mut().add_parent(parent)
    }
    fn remove_parent(&mut self, parent: Link<Self::Parent>) {
        self.borrow_mut().remove_parent(parent)
    }
}

impl<T> Child for BackLink<T>
where
    Link<T>: Child,
{
    type Parent = <Link<T> as Child>::Parent;
    fn get_parents(&self) -> Vec<BackLink<Self::Parent>> {
        match self.to_link() {
            Some(link) => link.get_parents(),
            None => Vec::new(),
        }
    }
    fn add_parent(&mut self, parent: Link<Self::Parent>) {
        if let Some(ref mut link) = self.to_link() {
            link.add_parent(parent)
        }
    }
    fn remove_parent(&mut self, parent: Link<Self::Parent>) {
        if let Some(ref mut link) = self.to_link() {
            link.remove_parent(parent)
        }
    }
}

/// A trait implemented by all nodes.
/// Will be derivable later. The implementation and type-safe builder is currently manual while we tweak the design
pub trait Builder {
    type Empty;
    type Full;
    /// Create a new empty builder that exposes all fields
    fn builder() -> Self::Empty;
}
