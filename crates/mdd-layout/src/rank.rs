use std::collections::HashMap;

use crate::types::{FlatNode, LayoutElement, LayoutGraph, LayoutGroup, LayoutNode};

/// Flatten all nodes from the element tree, recording cluster membership.
pub(crate) fn flatten_nodes(
    elements: &[LayoutElement],
    nodes: &[LayoutNode],
    groups: &[LayoutGroup],
    parent_chain: &[usize],
) -> Vec<FlatNode> {
    let mut result = Vec::new();
    for elem in elements {
        match elem {
            LayoutElement::NodeRef(ni) => {
                result.push(FlatNode {
                    node_index: *ni,
                    cluster_chain: parent_chain.to_vec(),
                });
            }
            LayoutElement::GroupRef(gi) => {
                let mut chain = parent_chain.to_vec();
                chain.push(*gi);
                result.extend(flatten_nodes(&groups[*gi].children, nodes, groups, &chain));
            }
        }
    }
    result
}

/// Longest-path rank assignment.
/// Returns a rank (layer index) for each flat node.
pub(crate) fn assign_ranks(
    flat_nodes: &[FlatNode],
    graph: &LayoutGraph,
) -> Vec<usize> {
    let n = flat_nodes.len();
    let mut name_to_flat: HashMap<&str, usize> = HashMap::new();
    for (fi, fnode) in flat_nodes.iter().enumerate() {
        name_to_flat.insert(&graph.nodes[fnode.node_index].name, fi);
    }

    let mut successors: Vec<Vec<usize>> = vec![vec![]; n];
    let mut in_degree: Vec<usize> = vec![0; n];
    for edge in &graph.edges {
        if let (Some(&from), Some(&to)) = (
            name_to_flat.get(edge.from.as_str()),
            name_to_flat.get(edge.to.as_str()),
        ) {
            if from != to {
                successors[from].push(to);
                in_degree[to] += 1;
            }
        }
    }

    let mut rank = vec![0usize; n];
    let mut queue: std::collections::VecDeque<usize> = std::collections::VecDeque::new();
    for i in 0..n {
        if in_degree[i] == 0 {
            queue.push_back(i);
        }
    }

    while let Some(u) = queue.pop_front() {
        for &v in &successors[u] {
            rank[v] = rank[v].max(rank[u] + 1);
            in_degree[v] -= 1;
            if in_degree[v] == 0 {
                queue.push_back(v);
            }
        }
    }

    rank
}
