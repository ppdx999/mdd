//! mdd-layout: Shared compound graph layout engine for mdd plugins.
//!
//! Provides a Sugiyama-style hierarchical layout with cluster (group) support.
//!
//! # Usage
//!
//! ```rust
//! use mdd_layout::{LayoutGraph, LayoutNode, LayoutEdge, LayoutGroup, LayoutElement, LayoutConfig, layout};
//!
//! let mut graph = LayoutGraph::new();
//! // ... add nodes, edges, groups ...
//! let result = layout(&graph, &LayoutConfig::default());
//! // result.positions: HashMap<String, (x, y, w, h)>
//! // result.edge_waypoints: HashMap<String, Vec<(x, y)>>
//! ```

mod types;
mod rank;
mod order;
mod position;

pub mod text;
pub mod edge;

pub use types::{
    Direction, LayoutConfig, LayoutEdge, LayoutElement, LayoutGraph, LayoutGroup, LayoutNode,
    LayoutResult,
};

/// Run the compound layout algorithm on the given graph.
///
/// This performs:
/// 1. Node flattening with cluster membership tracking
/// 2. Longest-path rank assignment
/// 3. Virtual node insertion for long edges
/// 4. Median heuristic crossing minimization with cluster contiguity
/// 5. Coordinate assignment with dynamic spacing
/// 6. Cluster boundary computation
/// 7. Overlap correction
/// 8. Post-overlap barycenter re-adjustment
pub fn layout(graph: &LayoutGraph, config: &LayoutConfig) -> LayoutResult {
    position::compound_layout(graph, config)
}
