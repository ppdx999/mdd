use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Graph definition (input)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct LayoutNode {
    pub name: String,
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone)]
pub struct LayoutEdge {
    pub from: String,
    pub to: String,
    pub label: String,
}

#[derive(Debug, Clone)]
pub struct LayoutGroup {
    pub name: String,
    pub children: Vec<LayoutElement>,
}

#[derive(Debug, Clone)]
pub enum LayoutElement {
    NodeRef(usize),
    GroupRef(usize),
}

#[derive(Debug)]
pub struct LayoutGraph {
    pub nodes: Vec<LayoutNode>,
    pub edges: Vec<LayoutEdge>,
    pub groups: Vec<LayoutGroup>,
    pub top_level: Vec<LayoutElement>,
}

impl LayoutGraph {
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            edges: Vec::new(),
            groups: Vec::new(),
            top_level: Vec::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Layout configuration
// ---------------------------------------------------------------------------

pub struct LayoutConfig {
    pub padding: f64,
    pub node_sep: f64,       // 0.0 = auto-compute from edge labels
    pub rank_sep: f64,       // 0.0 = auto-compute
    pub group_h_pad: f64,
    pub group_v_pad: f64,
    pub group_header_h: f64,
    pub default_node_w: f64,
    pub default_node_h: f64,
}

impl Default for LayoutConfig {
    fn default() -> Self {
        Self {
            padding: 40.0,
            node_sep: 0.0,
            rank_sep: 0.0,
            group_h_pad: 16.0,
            group_v_pad: 12.0,
            group_header_h: 28.0,
            default_node_w: 100.0,
            default_node_h: 70.0,
        }
    }
}

// ---------------------------------------------------------------------------
// Layout result (output)
// ---------------------------------------------------------------------------

pub struct LayoutResult {
    /// Node and group positions: name → (x, y, width, height)
    pub positions: HashMap<String, (f64, f64, f64, f64)>,
    /// Edge waypoints for long edges: "from→to" → [(x, y), ...]
    pub edge_waypoints: HashMap<String, Vec<(f64, f64)>>,
}

// ---------------------------------------------------------------------------
// Internal types
// ---------------------------------------------------------------------------

/// A flattened node with its cluster membership chain (outermost first).
/// node_index == usize::MAX indicates a virtual node.
#[derive(Debug, Clone)]
pub(crate) struct FlatNode {
    pub node_index: usize,
    pub cluster_chain: Vec<usize>,
}

impl FlatNode {
    pub fn is_virtual(&self) -> bool {
        self.node_index == usize::MAX
    }
}

