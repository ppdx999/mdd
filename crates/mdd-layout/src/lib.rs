//! mdd-layout: Shared graph layout engine for mdd plugins.
//!
//! Two layout algorithms:
//! - **Sugiyama** (`layout()`): Hierarchical layout with cluster (group) support.
//!   Best for flow diagrams (infra, ER, DFD).
//! - **Force-directed** (`force_layout()`): Physics-based layout where connected
//!   nodes attract and all nodes repel. Best for network/concept diagrams.
//!
//! # Usage
//!
//! ```rust
//! use mdd_layout::{LayoutGraph, LayoutNode, LayoutEdge, LayoutConfig, layout};
//!
//! let mut graph = LayoutGraph::new();
//! // ... add nodes, edges, groups ...
//! let result = layout(&graph, &LayoutConfig::default());
//! // result.positions: HashMap<String, (x, y, w, h)>
//! ```

mod types;
mod rank;
mod order;
mod position;

pub mod text;
pub mod edge;
pub mod force;

pub use types::{
    Direction, LayoutConfig, LayoutEdge, LayoutElement, LayoutGraph, LayoutGroup, LayoutNode,
    LayoutResult,
};
pub use force::ForceConfig;

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
/// Run the Sugiyama compound layout algorithm on the given graph.
pub fn layout(graph: &LayoutGraph, config: &LayoutConfig) -> LayoutResult {
    position::compound_layout(graph, config)
}

/// Run the force-directed layout algorithm on the given graph.
pub fn force_layout(graph: &LayoutGraph, config: &ForceConfig) -> LayoutResult {
    force::force_layout(graph, config)
}
