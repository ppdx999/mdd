use std::collections::HashMap;
use std::io::{self, Read};

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
enum NodeType {
    Server,
    Db,
    Lb,
    Cache,
    Queue,
    Storage,
    Cdn,
    Network,
    User,
    Phone,
    Cloud,
    Generic,
}

impl NodeType {
    fn from_str(s: &str) -> Self {
        match s {
            "server" => NodeType::Server,
            "db" | "database" => NodeType::Db,
            "lb" | "loadbalancer" => NodeType::Lb,
            "cache" => NodeType::Cache,
            "queue" => NodeType::Queue,
            "storage" => NodeType::Storage,
            "cdn" => NodeType::Cdn,
            "network" | "vpc" | "subnet" => NodeType::Network,
            "user" | "client" => NodeType::User,
            "phone" | "telephone" => NodeType::Phone,
            "cloud" | "internet" | "pstn" => NodeType::Cloud,
            _ => NodeType::Generic,
        }
    }
}

#[derive(Debug)]
struct Node {
    name: String,
    node_type: NodeType,
}

#[derive(Debug)]
struct Group {
    name: String,
    children: Vec<Element>,
}

#[derive(Debug)]
enum Element {
    NodeRef(usize),   // index into flat nodes vec
    GroupRef(usize),  // index into flat groups vec
}

#[derive(Debug)]
struct Edge {
    from: String,
    to: String,
    label: String,
}

#[derive(Debug)]
struct Diagram {
    nodes: Vec<Node>,
    groups: Vec<Group>,
    top_level: Vec<Element>,
    edges: Vec<Edge>,
    show_type: bool,
}

// ---------------------------------------------------------------------------
// Parser
// ---------------------------------------------------------------------------

fn parse(input: &str) -> Result<Diagram, String> {
    let mut nodes: Vec<Node> = Vec::new();
    let mut groups: Vec<Group> = Vec::new();
    let mut top_level: Vec<Element> = Vec::new();
    let mut edges: Vec<Edge> = Vec::new();
    let mut name_to_node: HashMap<String, usize> = HashMap::new();
    let mut name_to_group: HashMap<String, usize> = HashMap::new();
    let mut show_type = true;

    // Stack for nested groups: (group_index, children_so_far)
    let mut group_stack: Vec<(usize, Vec<Element>)> = Vec::new();

    for line in input.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if line == "hide type" {
            show_type = false;
            continue;
        }

        if line == "}" {
            if let Some((gidx, children)) = group_stack.pop() {
                groups[gidx].children = children;
                let elem = Element::GroupRef(gidx);
                if let Some(parent) = group_stack.last_mut() {
                    parent.1.push(elem);
                } else {
                    top_level.push(elem);
                }
            } else {
                return Err("Unexpected }".to_string());
            }
            continue;
        }

        if line.starts_with("group ") {
            let rest = line.strip_prefix("group ").unwrap();
            if let Some(name) = rest.strip_suffix(" {") {
                let name = name.trim().trim_matches('"').to_string();
                let gidx = groups.len();
                name_to_group.insert(name.clone(), gidx);
                groups.push(Group {
                    name,
                    children: Vec::new(),
                });
                group_stack.push((gidx, Vec::new()));
                continue;
            }
            return Err(format!("Invalid group syntax: {}", line));
        }

        if line.starts_with("node ") {
            let rest = line.strip_prefix("node ").unwrap();
            let (name, node_type) = if let Some((name_part, type_part)) = rest.split_once(" type=") {
                (name_part.trim().to_string(), NodeType::from_str(type_part.trim()))
            } else {
                (rest.trim().to_string(), NodeType::Generic)
            };

            let nidx = nodes.len();
            name_to_node.insert(name.clone(), nidx);
            nodes.push(Node { name, node_type });

            let elem = Element::NodeRef(nidx);
            if let Some(parent) = group_stack.last_mut() {
                parent.1.push(elem);
            } else {
                top_level.push(elem);
            }
            continue;
        }

        if line.contains(" -> ") {
            let parts: Vec<&str> = line.splitn(2, " -> ").collect();
            let from = parts[0].trim().trim_matches('"').to_string();
            let rest = parts[1];
            let (to, label) = if let Some((to_part, label_part)) = rest.split_once(" : ") {
                (
                    to_part.trim().trim_matches('"').to_string(),
                    label_part.trim().trim_matches('"').to_string(),
                )
            } else {
                (rest.trim().trim_matches('"').to_string(), String::new())
            };
            edges.push(Edge { from, to, label });
            continue;
        }

        return Err(format!("Unknown syntax: {}", line));
    }

    if !group_stack.is_empty() {
        return Err("Unclosed group block".to_string());
    }

    Ok(Diagram {
        nodes,
        groups,
        top_level,
        edges,
        show_type,
    })
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const CHAR_WIDTH: f64 = 8.0;
const PADDING: f64 = 40.0;

const NODE_W: f64 = 100.0;
const NODE_H: f64 = 70.0;
const ICON_SIZE: f64 = 32.0;

const GROUP_H_PAD: f64 = 16.0;
const GROUP_V_PAD: f64 = 12.0;
const GROUP_HEADER_H: f64 = 28.0;
const COLOR_DARK: &str = "#333";
const COLOR_EDGE: &str = "#666";
const COLOR_GROUP_FILL: &str = "#fafafa";
const COLOR_GROUP_STROKE: &str = "#bbb";

// Node type colors
fn node_colors(nt: &NodeType) -> (&'static str, &'static str) {
    match nt {
        NodeType::Server => ("#e3f2fd", "#1565c0"),
        NodeType::Db => ("#e8f5e9", "#2e7d32"),
        NodeType::Lb => ("#fff3e0", "#e65100"),
        NodeType::Cache => ("#fce4ec", "#c62828"),
        NodeType::Queue => ("#f3e5f5", "#6a1b9a"),
        NodeType::Storage => ("#e0f2f1", "#00695c"),
        NodeType::Cdn => ("#fff8e1", "#f9a825"),
        NodeType::Network => ("#e8eaf6", "#283593"),
        NodeType::User => ("#fafafa", "#616161"),
        NodeType::Phone => ("#e8eaf6", "#4527a0"),
        NodeType::Cloud => ("#e0f7fa", "#00838f"),
        NodeType::Generic => ("#f5f5f5", "#757575"),
    }
}

// ---------------------------------------------------------------------------
// Text utilities
// ---------------------------------------------------------------------------

fn text_width(s: &str) -> f64 {
    s.chars()
        .map(|c| if c.is_ascii() { CHAR_WIDTH } else { 14.0 })
        .sum()
}

// ---------------------------------------------------------------------------
// Compound Layout Engine (Graphviz-style global Sugiyama with cluster support)
// ---------------------------------------------------------------------------

/// Get the name of an element (node or group)
fn element_name(elem: &Element, nodes: &[Node], groups: &[Group]) -> String {
    match elem {
        Element::NodeRef(i) => nodes[*i].name.clone(),
        Element::GroupRef(i) => groups[*i].name.clone(),
    }
}

/// Flatten all nodes from the element tree, recording cluster membership.
/// Returns (flat_node_indices, cluster_chains) where cluster_chains[i] is
/// the list of group indices the node belongs to (outermost first).
fn flatten_nodes(
    elements: &[Element],
    nodes: &[Node],
    groups: &[Group],
    parent_chain: &[usize],
) -> Vec<(usize, Vec<usize>)> {
    let mut result = Vec::new();
    for elem in elements {
        match elem {
            Element::NodeRef(ni) => {
                result.push((*ni, parent_chain.to_vec()));
            }
            Element::GroupRef(gi) => {
                let mut chain = parent_chain.to_vec();
                chain.push(*gi);
                result.extend(flatten_nodes(&groups[*gi].children, nodes, groups, &chain));
            }
        }
    }
    result
}

/// Longest-path rank assignment. Returns rank for each flat node index.
fn assign_ranks(
    flat_nodes: &[(usize, Vec<usize>)],
    edges: &[Edge],
    nodes: &[Node],
) -> Vec<usize> {
    let n = flat_nodes.len();
    // Map node name → flat index
    let mut name_to_flat: HashMap<&str, usize> = HashMap::new();
    for (fi, (ni, _)) in flat_nodes.iter().enumerate() {
        name_to_flat.insert(&nodes[*ni].name, fi);
    }

    // Build adjacency
    let mut successors: Vec<Vec<usize>> = vec![vec![]; n];
    let mut in_degree: Vec<usize> = vec![0; n];
    for edge in edges {
        if let (Some(&from), Some(&to)) = (name_to_flat.get(edge.from.as_str()), name_to_flat.get(edge.to.as_str())) {
            if from != to {
                successors[from].push(to);
                in_degree[to] += 1;
            }
        }
    }

    // Topological sort + longest path
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

/// Median heuristic for ordering nodes within each rank.
/// Iterates up/down sweeps to minimize crossings.
/// `graph_edges` are (from_flat_idx, to_flat_idx) pairs.
/// Returns ordered node indices per rank.
fn order_ranks(
    flat_nodes: &[(usize, Vec<usize>)],
    ranks: &[usize],
    graph_edges: &[(usize, usize)],
) -> Vec<Vec<usize>> {
    let max_rank = ranks.iter().copied().max().unwrap_or(0);

    // Build rank buckets: rank → list of flat indices
    let mut rank_buckets: Vec<Vec<usize>> = vec![vec![]; max_rank + 1];
    for (fi, _) in flat_nodes.iter().enumerate() {
        rank_buckets[ranks[fi]].push(fi);
    }

    // Build predecessor/successor lists
    let n = flat_nodes.len();
    let mut predecessors: Vec<Vec<usize>> = vec![vec![]; n];
    let mut successors: Vec<Vec<usize>> = vec![vec![]; n];
    for &(from, to) in graph_edges {
        if from != to && from < n && to < n {
            successors[from].push(to);
            predecessors[to].push(from);
        }
    }

    // Position lookup: flat_index → position within its rank
    let mut pos: Vec<usize> = vec![0; n];
    for bucket in &rank_buckets {
        for (p, &fi) in bucket.iter().enumerate() {
            pos[fi] = p;
        }
    }

    let max_iter = 24;
    for iter in 0..max_iter {
        if iter % 2 == 0 {
            // Down sweep: for each rank from 1..max, order by median of predecessors
            for r in 1..=max_rank {
                let mut med_vals: Vec<(f64, usize)> = Vec::new();
                let mut med_map: HashMap<usize, f64> = HashMap::new();
                for &fi in &rank_buckets[r] {
                    let preds = &predecessors[fi];
                    if preds.is_empty() {
                        med_vals.push((pos[fi] as f64, fi));
                        med_map.insert(fi, pos[fi] as f64);
                    } else {
                        let mut positions: Vec<f64> = preds.iter().map(|&p| pos[p] as f64).collect();
                        positions.sort_by(|a, b| a.partial_cmp(b).unwrap());
                        let med = if positions.len() % 2 == 1 {
                            positions[positions.len() / 2]
                        } else {
                            let l = positions.len() / 2 - 1;
                            (positions[l] + positions[l + 1]) / 2.0
                        };
                        med_vals.push((med, fi));
                        med_map.insert(fi, med);
                    }
                }
                med_vals.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
                rank_buckets[r] = med_vals.iter().map(|&(_, fi)| fi).collect();
                enforce_cluster_contiguity(&mut rank_buckets[r], flat_nodes, &med_map);
                for (p, &fi) in rank_buckets[r].iter().enumerate() {
                    pos[fi] = p;
                }
            }
        } else {
            // Up sweep: for each rank from max-1..0, order by median of successors
            for r in (0..max_rank).rev() {
                let mut med_vals: Vec<(f64, usize)> = Vec::new();
                let mut med_map: HashMap<usize, f64> = HashMap::new();
                for &fi in &rank_buckets[r] {
                    let succs = &successors[fi];
                    if succs.is_empty() {
                        med_vals.push((pos[fi] as f64, fi));
                        med_map.insert(fi, pos[fi] as f64);
                    } else {
                        let mut positions: Vec<f64> = succs.iter().map(|&s| pos[s] as f64).collect();
                        positions.sort_by(|a, b| a.partial_cmp(b).unwrap());
                        let med = if positions.len() % 2 == 1 {
                            positions[positions.len() / 2]
                        } else {
                            let l = positions.len() / 2 - 1;
                            (positions[l] + positions[l + 1]) / 2.0
                        };
                        med_vals.push((med, fi));
                        med_map.insert(fi, med);
                    }
                }
                med_vals.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
                rank_buckets[r] = med_vals.iter().map(|&(_, fi)| fi).collect();
                enforce_cluster_contiguity(&mut rank_buckets[r], flat_nodes, &med_map);
                for (p, &fi) in rank_buckets[r].iter().enumerate() {
                    pos[fi] = p;
                }
            }
        }

        // Transpose step: swap adjacent nodes (within same cluster) or
        // adjacent cluster blocks (between different clusters)
        for r in 0..=max_rank {
            // Within-cluster swaps
            let bucket = &mut rank_buckets[r];
            let mut improved = true;
            while improved {
                improved = false;
                for i in 0..bucket.len().saturating_sub(1) {
                    let v = bucket[i];
                    let w = bucket[i + 1];
                    if !same_cluster(v, w, flat_nodes) {
                        continue;
                    }
                    let cross_before = count_crossings_pair(v, w, &successors, &pos);
                    let cross_after = count_crossings_pair(w, v, &successors, &pos);
                    if cross_after < cross_before {
                        bucket.swap(i, i + 1);
                        pos[v] = i + 1;
                        pos[w] = i;
                        improved = true;
                    }
                }
            }

            // Between-cluster block swaps
            // Identify contiguous cluster blocks
            let mut blocks: Vec<(Option<usize>, usize, usize)> = Vec::new(); // (cluster, start, end)
            if !bucket.is_empty() {
                let mut block_start = 0;
                let mut current_cluster = flat_nodes[bucket[0]].1.last().copied();
                for i in 1..bucket.len() {
                    let c = flat_nodes[bucket[i]].1.last().copied();
                    if c != current_cluster {
                        blocks.push((current_cluster, block_start, i));
                        block_start = i;
                        current_cluster = c;
                    }
                }
                blocks.push((current_cluster, block_start, bucket.len()));
            }

            // Try swapping adjacent blocks
            let mut block_improved = true;
            while block_improved && blocks.len() > 1 {
                block_improved = false;
                for bi in 0..blocks.len() - 1 {
                    let (_, a_start, a_end) = blocks[bi];
                    let (_, b_start, b_end) = blocks[bi + 1];
                    let block_a: Vec<usize> = bucket[a_start..a_end].to_vec();
                    let block_b: Vec<usize> = bucket[b_start..b_end].to_vec();

                    let cross_ab = count_block_crossings(&block_a, &block_b, &successors, &predecessors, &pos);
                    // Temporarily swap positions to count crossings in reverse
                    let a_len = block_a.len();
                    let b_len = block_b.len();
                    for (j, &fi) in block_b.iter().enumerate() { pos[fi] = a_start + j; }
                    for (j, &fi) in block_a.iter().enumerate() { pos[fi] = a_start + b_len + j; }
                    let cross_ba = count_block_crossings(&block_b, &block_a, &successors, &predecessors, &pos);

                    if cross_ba < cross_ab {
                        // Apply swap: [B, A] in bucket
                        let mut new_section = block_b.clone();
                        new_section.extend(&block_a);
                        bucket[a_start..b_end].copy_from_slice(&new_section);
                        // Update blocks
                        blocks[bi] = (blocks[bi + 1].0, a_start, a_start + b_len);
                        blocks[bi + 1] = (blocks[bi].0, a_start + b_len, b_end);
                        block_improved = true;
                    } else {
                        // Restore positions
                        for (j, &fi) in block_a.iter().enumerate() { pos[fi] = a_start + j; }
                        for (j, &fi) in block_b.iter().enumerate() { pos[fi] = a_start + a_len + j; }
                    }
                }
            }
            // Sync positions after block swaps
            for (i, &fi) in bucket.iter().enumerate() {
                pos[fi] = i;
            }
        }
    }

    rank_buckets
}

/// Check if two flat nodes are in the same innermost cluster (or both unclustered)
fn same_cluster(a: usize, b: usize, flat_nodes: &[(usize, Vec<usize>)]) -> bool {
    let ca = flat_nodes[a].1.last();
    let cb = flat_nodes[b].1.last();
    ca == cb
}

/// Enforce cluster contiguity: group nodes by their innermost cluster,
/// ordering cluster blocks by the median of their members' heuristic values.
/// `medians` maps flat_index → median value computed from adjacent rank connections.
fn enforce_cluster_contiguity(
    bucket: &mut Vec<usize>,
    flat_nodes: &[(usize, Vec<usize>)],
    medians: &HashMap<usize, f64>,
) {
    if bucket.len() <= 1 {
        return;
    }

    // Group by innermost cluster
    let mut cluster_order: Vec<Option<usize>> = Vec::new();
    let mut cluster_groups: HashMap<Option<usize>, Vec<usize>> = HashMap::new();
    let mut seen_clusters: std::collections::HashSet<Option<usize>> = std::collections::HashSet::new();

    for &fi in bucket.iter() {
        let cluster = flat_nodes[fi].1.last().copied();
        if seen_clusters.insert(cluster) {
            cluster_order.push(cluster);
        }
        cluster_groups.entry(cluster).or_default().push(fi);
    }

    // Order cluster blocks by the median of their members' heuristic values
    let mut block_medians: Vec<(f64, Option<usize>)> = Vec::new();
    for &cluster in &cluster_order {
        if let Some(members) = cluster_groups.get(&cluster) {
            let mut vals: Vec<f64> = members
                .iter()
                .map(|&fi| *medians.get(&fi).unwrap_or(&(fi as f64)))
                .collect();
            vals.sort_by(|a, b| a.partial_cmp(b).unwrap());
            let med = if vals.len() % 2 == 1 {
                vals[vals.len() / 2]
            } else if vals.len() >= 2 {
                let l = vals.len() / 2 - 1;
                (vals[l] + vals[l + 1]) / 2.0
            } else {
                vals.first().copied().unwrap_or(0.0)
            };
            block_medians.push((med, cluster));
        }
    }
    block_medians.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

    // Rebuild bucket: cluster blocks in median order, nodes within cluster in sorted order
    bucket.clear();
    for (_, cluster) in &block_medians {
        if let Some(group) = cluster_groups.get(cluster) {
            bucket.extend(group);
        }
    }
}

/// Count edge crossings between two nodes assuming v is at position before w
fn count_crossings_pair(v: usize, w: usize, successors: &[Vec<usize>], pos: &[usize]) -> usize {
    let mut count = 0;
    for &sv in &successors[v] {
        for &sw in &successors[w] {
            if pos[sv] > pos[sw] {
                count += 1;
            }
        }
    }
    count
}

/// Count total crossings between two cluster blocks (all nodes in block_a vs all in block_b)
fn count_block_crossings(
    block_a: &[usize],
    block_b: &[usize],
    successors: &[Vec<usize>],
    predecessors: &[Vec<usize>],
    pos: &[usize],
) -> usize {
    let mut count = 0;
    for &a in block_a {
        for &b in block_b {
            // Count successor crossings
            for &sa in &successors[a] {
                for &sb in &successors[b] {
                    if pos[sa] > pos[sb] { count += 1; }
                }
            }
            // Count predecessor crossings
            for &pa in &predecessors[a] {
                for &pb in &predecessors[b] {
                    if pos[pa] > pos[pb] { count += 1; }
                }
            }
        }
    }
    count
}

/// Assign coordinates and compute cluster boundaries.
/// Returns (positions, edge_waypoints) where edge_waypoints maps
/// "from_name->to_name" to a list of (x,y) waypoints for routing.
fn compound_layout(diagram: &Diagram) -> (HashMap<String, (f64, f64, f64, f64)>, HashMap<String, Vec<(f64, f64)>>) {
    let mut positions: HashMap<String, (f64, f64, f64, f64)> = HashMap::new();
    let mut edge_waypoints: HashMap<String, Vec<(f64, f64)>> = HashMap::new();

    // Phase 1: Flatten
    let real_nodes = flatten_nodes(
        &diagram.top_level,
        &diagram.nodes,
        &diagram.groups,
        &[],
    );

    if real_nodes.is_empty() {
        return (positions, edge_waypoints);
    }

    let real_count = real_nodes.len();

    // Phase 2: Rank assignment (real nodes only)
    let real_ranks = assign_ranks(&real_nodes, &diagram.edges, &diagram.nodes);

    // Phase 2.5: Insert virtual nodes for long edges
    // flat_nodes = real_nodes + virtual_nodes
    // Virtual nodes have node_index = usize::MAX and inherit the cluster chain
    // of the edge's source (arbitrary but reasonable choice).
    let mut flat_nodes: Vec<(usize, Vec<usize>)> = real_nodes.clone();
    let mut ranks: Vec<usize> = real_ranks.clone();

    // Build name→flat_index for real nodes
    let mut name_to_flat: HashMap<&str, usize> = HashMap::new();
    for (fi, (ni, _)) in real_nodes.iter().enumerate() {
        name_to_flat.insert(&diagram.nodes[*ni].name, fi);
    }

    // Build graph edges (index-based), splitting long edges with virtual nodes
    let mut graph_edges: Vec<(usize, usize)> = Vec::new();
    // Track which virtual nodes belong to which original edge: edge_key → vec of virtual fi
    let mut virtual_chains: HashMap<String, Vec<usize>> = HashMap::new();

    for edge in &diagram.edges {
        let from_fi = name_to_flat.get(edge.from.as_str()).copied();
        let to_fi = name_to_flat.get(edge.to.as_str()).copied();
        if let (Some(from), Some(to)) = (from_fi, to_fi) {
            if from == to {
                continue;
            }
            let from_rank = ranks[from];
            let to_rank = ranks[to];
            if to_rank <= from_rank {
                // Back edge or same rank — add direct edge
                graph_edges.push((from, to));
                continue;
            }
            let span = to_rank - from_rank;
            if span <= 1 {
                // Adjacent ranks — no virtual nodes needed
                graph_edges.push((from, to));
            } else {
                // Long edge: insert virtual nodes at intermediate ranks
                let edge_key = format!("{}→{}", edge.from, edge.to);
                let mut chain = Vec::new();
                let mut prev = from;
                // Inherit cluster chain from the edge's source
                let cluster_chain = flat_nodes[from].1.clone();
                for r in (from_rank + 1)..to_rank {
                    let vi = flat_nodes.len();
                    flat_nodes.push((usize::MAX, cluster_chain.clone())); // usize::MAX = virtual
                    ranks.push(r);
                    graph_edges.push((prev, vi));
                    chain.push(vi);
                    prev = vi;
                }
                graph_edges.push((prev, to));
                virtual_chains.insert(edge_key, chain);
            }
        }
    }

    // Phase 3: Order within ranks (including virtual nodes)
    let rank_buckets = order_ranks(&flat_nodes, &ranks, &graph_edges);

    // Phase 4: Coordinate assignment
    let max_rank = ranks.iter().copied().max().unwrap_or(0);
    let rank_sep = 40.0;
    let node_sep = 20.0;
    let cluster_sep = GROUP_H_PAD * 2.0 + 16.0;
    let virtual_w = 2.0; // virtual nodes are thin

    // Width of a flat node (real or virtual)
    let node_w = |fi: usize| -> f64 {
        if flat_nodes[fi].0 == usize::MAX { virtual_w } else { NODE_W }
    };

    // Compute the minimum spacing between two adjacent nodes in a rank.
    let spacing = |a: usize, b: usize| -> f64 {
        let ca = &flat_nodes[a].1;
        let cb = &flat_nodes[b].1;
        if ca == cb {
            node_sep
        } else {
            let shared = ca.iter().zip(cb.iter()).take_while(|(x, y)| x == y).count();
            let boundaries = (ca.len() - shared) + (cb.len() - shared);
            node_sep + cluster_sep * (boundaries as f64).max(1.0)
        }
    };

    // Compute rank heights, accounting for cluster headers.
    let rank_height: Vec<f64> = vec![NODE_H; max_rank + 1];

    let mut cluster_min_rank: HashMap<usize, usize> = HashMap::new();
    for (fi, (ni, chain)) in flat_nodes.iter().enumerate() {
        if *ni == usize::MAX { continue; } // skip virtual for cluster rank computation
        for &gi in chain {
            let entry = cluster_min_rank.entry(gi).or_insert(ranks[fi]);
            *entry = (*entry).min(ranks[fi]);
        }
    }

    let mut rank_extra_top: Vec<f64> = vec![0.0; max_rank + 1];
    for (_, &min_r) in &cluster_min_rank {
        rank_extra_top[min_r] += GROUP_HEADER_H + GROUP_V_PAD;
    }
    for extra in &mut rank_extra_top {
        *extra = extra.min(GROUP_HEADER_H * 2.0 + GROUP_V_PAD * 2.0);
    }

    // Y coordinate per rank
    let mut rank_y: Vec<f64> = vec![0.0; max_rank + 1];
    rank_y[0] = PADDING + rank_extra_top[0];
    for r in 1..=max_rank {
        rank_y[r] = rank_y[r - 1] + rank_height[r - 1] + rank_sep + rank_extra_top[r];
    }

    // X coordinate: place nodes left to right within each rank
    let mut node_x: Vec<f64> = vec![0.0; flat_nodes.len()];
    let mut node_y: Vec<f64> = vec![0.0; flat_nodes.len()];

    for r in 0..=max_rank {
        let mut x_cursor = PADDING;
        for (idx, &fi) in rank_buckets[r].iter().enumerate() {
            if idx > 0 {
                let prev_fi = rank_buckets[r][idx - 1];
                x_cursor += spacing(prev_fi, fi);
            }
            node_x[fi] = x_cursor;
            node_y[fi] = rank_y[r];
            x_cursor += node_w(fi);
        }
    }

    // Center alignment using graph_edges (works for both real and virtual nodes)
    // Build successor/predecessor from graph_edges for coordinate refinement
    let total = flat_nodes.len();
    let mut adj_succ: Vec<Vec<usize>> = vec![vec![]; total];
    let mut adj_pred: Vec<Vec<usize>> = vec![vec![]; total];
    for &(from, to) in &graph_edges {
        if from < total && to < total {
            adj_succ[from].push(to);
            adj_pred[to].push(from);
        }
    }

    for _pass in 0..6 {
        for r in 0..=max_rank {
            let bucket = &rank_buckets[r];
            if bucket.is_empty() { continue; }

            let mut targets: Vec<(usize, f64)> = Vec::new();
            for &fi in bucket {
                let mut connected_xs: Vec<f64> = Vec::new();
                for &s in &adj_succ[fi] {
                    connected_xs.push(node_x[s] + node_w(s) / 2.0);
                }
                for &p in &adj_pred[fi] {
                    connected_xs.push(node_x[p] + node_w(p) / 2.0);
                }
                if !connected_xs.is_empty() {
                    let avg = connected_xs.iter().sum::<f64>() / connected_xs.len() as f64;
                    targets.push((fi, avg - node_w(fi) / 2.0));
                }
            }

            for (fi, target_x) in &targets {
                let idx = bucket.iter().position(|&f| f == *fi).unwrap();
                let min_x = if idx == 0 {
                    PADDING
                } else {
                    let prev = bucket[idx - 1];
                    node_x[prev] + node_w(prev) + spacing(prev, *fi)
                };
                let max_x = if idx == bucket.len() - 1 {
                    f64::MAX
                } else {
                    let next = bucket[idx + 1];
                    node_x[next] - node_w(*fi) - spacing(*fi, next)
                };
                node_x[*fi] = target_x.max(min_x).min(max_x);
            }
        }
    }

    // Store real node positions only
    for fi in 0..real_count {
        let (ni, _) = flat_nodes[fi];
        positions.insert(
            diagram.nodes[ni].name.clone(),
            (node_x[fi], node_y[fi], NODE_W, NODE_H),
        );
    }

    // Build edge waypoints from virtual node positions
    for (edge_key, chain) in &virtual_chains {
        let waypoints: Vec<(f64, f64)> = chain
            .iter()
            .map(|&vi| (node_x[vi] + virtual_w / 2.0, node_y[vi] + NODE_H / 2.0))
            .collect();
        edge_waypoints.insert(edge_key.clone(), waypoints);
    }

    // Phase 5: Compute cluster boundaries from node positions
    compute_cluster_bounds(
        &diagram.top_level,
        &diagram.nodes,
        &diagram.groups,
        &mut positions,
    );

    // Phase 6: Fix cluster overlaps by shifting nodes
    // Compute sort keys for top-level elements based on their highest-rank node positions.
    // This ensures overlap correction respects the rank-based ordering rather than
    // bounding box centers (which can be misleading for wide clusters).
    let mut elem_sort_keys: HashMap<String, f64> = HashMap::new();
    {
        // For each top-level element, find the min rank among its nodes and compute
        // the average x of nodes at that rank.
        fn compute_sort_key(
            elem: &Element, nodes_list: &[Node], groups: &[Group],
            flat_nodes: &[(usize, Vec<usize>)], node_x: &[f64], ranks: &[usize],
            real_count: usize,
        ) -> f64 {
            // Collect all real flat indices belonging to this element
            let mut indices: Vec<usize> = Vec::new();
            collect_flat_indices(elem, nodes_list, groups, flat_nodes, real_count, &mut indices);
            if indices.is_empty() { return 0.0; }
            // Find min rank
            let min_rank = indices.iter().map(|&fi| ranks[fi]).min().unwrap();
            // Average x of nodes at min rank
            let at_min: Vec<f64> = indices.iter()
                .filter(|&&fi| ranks[fi] == min_rank)
                .map(|&fi| node_x[fi])
                .collect();
            at_min.iter().sum::<f64>() / at_min.len() as f64
        }

        fn collect_flat_indices(
            elem: &Element, nodes_list: &[Node], groups: &[Group],
            flat_nodes: &[(usize, Vec<usize>)], real_count: usize,
            out: &mut Vec<usize>,
        ) {
            match elem {
                Element::NodeRef(ni) => {
                    for fi in 0..real_count {
                        if flat_nodes[fi].0 == *ni {
                            out.push(fi);
                        }
                    }
                }
                Element::GroupRef(gi) => {
                    for child in &groups[*gi].children {
                        collect_flat_indices(child, nodes_list, groups, flat_nodes, real_count, out);
                    }
                }
            }
        }

        for elem in &diagram.top_level {
            let name = element_name(elem, &diagram.nodes, &diagram.groups);
            let key = compute_sort_key(elem, &diagram.nodes, &diagram.groups, &flat_nodes, &node_x, &ranks, real_count);
            elem_sort_keys.insert(name, key);
        }
    }

    for _ in 0..5 {
        let shifts = find_cluster_shifts(
            &diagram.top_level,
            &diagram.nodes,
            &diagram.groups,
            &positions,
            &elem_sort_keys,
        );
        if shifts.is_empty() {
            break;
        }
        for fi in 0..real_count {
            let (ni, _) = flat_nodes[fi];
            if let Some(&dx) = shifts.get(&diagram.nodes[ni].name) {
                node_x[fi] += dx;
            }
        }
        // Also shift virtual nodes that belong to shifted edges
        for (edge_key, chain) in &virtual_chains {
            // Parse edge_key "from→to" to find if source was shifted
            if let Some(from_name) = edge_key.split('→').next() {
                if let Some(&dx) = shifts.get(from_name) {
                    for &vi in chain {
                        node_x[vi] += dx;
                    }
                }
            }
        }
        positions.clear();
        for fi in 0..real_count {
            let (ni, _) = flat_nodes[fi];
            positions.insert(
                diagram.nodes[ni].name.clone(),
                (node_x[fi], node_y[fi], NODE_W, NODE_H),
            );
        }
        // Update waypoints
        for (edge_key, chain) in &virtual_chains {
            let waypoints: Vec<(f64, f64)> = chain
                .iter()
                .map(|&vi| (node_x[vi] + virtual_w / 2.0, node_y[vi] + NODE_H / 2.0))
                .collect();
            edge_waypoints.insert(edge_key.clone(), waypoints);
        }
        compute_cluster_bounds(
            &diagram.top_level,
            &diagram.nodes,
            &diagram.groups,
            &mut positions,
        );
    }

    // Phase 7: Post-overlap barycenter re-adjustment.
    // After overlap correction shifts clusters, standalone nodes and virtual nodes
    // may be misaligned with their connections. Re-run barycenter to fix.
    for _pass in 0..4 {
        for r in 0..=max_rank {
            let bucket = &rank_buckets[r];
            if bucket.is_empty() { continue; }

            for (idx_in_bucket, &fi) in bucket.iter().enumerate() {
                let mut connected_xs: Vec<f64> = Vec::new();
                for &s in &adj_succ[fi] {
                    connected_xs.push(node_x[s] + node_w(s) / 2.0);
                }
                for &p in &adj_pred[fi] {
                    connected_xs.push(node_x[p] + node_w(p) / 2.0);
                }
                if connected_xs.is_empty() { continue; }

                let avg = connected_xs.iter().sum::<f64>() / connected_xs.len() as f64;
                let target_x = avg - node_w(fi) / 2.0;

                let min_x = if idx_in_bucket == 0 {
                    PADDING
                } else {
                    let prev = bucket[idx_in_bucket - 1];
                    node_x[prev] + node_w(prev) + spacing(prev, fi)
                };
                let max_x = if idx_in_bucket == bucket.len() - 1 {
                    f64::MAX
                } else {
                    let next = bucket[idx_in_bucket + 1];
                    node_x[next] - node_w(fi) - spacing(fi, next)
                };
                node_x[fi] = target_x.max(min_x).min(max_x);
            }
        }
    }

    // Recompute final positions
    positions.clear();
    for fi in 0..real_count {
        let (ni, _) = flat_nodes[fi];
        positions.insert(
            diagram.nodes[ni].name.clone(),
            (node_x[fi], node_y[fi], NODE_W, NODE_H),
        );
    }
    for (edge_key, chain) in &virtual_chains {
        let waypoints: Vec<(f64, f64)> = chain
            .iter()
            .map(|&vi| (node_x[vi] + virtual_w / 2.0, node_y[vi] + NODE_H / 2.0))
            .collect();
        edge_waypoints.insert(edge_key.clone(), waypoints);
    }
    compute_cluster_bounds(
        &diagram.top_level,
        &diagram.nodes,
        &diagram.groups,
        &mut positions,
    );

    (positions, edge_waypoints)
}

/// Recursively compute cluster (group) bounding boxes from the positions of their children.
fn compute_cluster_bounds(
    elements: &[Element],
    nodes: &[Node],
    groups: &[Group],
    positions: &mut HashMap<String, (f64, f64, f64, f64)>,
) {
    for elem in elements {
        if let Element::GroupRef(gi) = elem {
            let g = &groups[*gi];
            // Recurse first so nested groups have bounds
            compute_cluster_bounds(&g.children, nodes, groups, positions);

            // Collect bounds of all children
            let mut min_x = f64::MAX;
            let mut min_y = f64::MAX;
            let mut max_x = f64::MIN;
            let mut max_y = f64::MIN;

            for child in &g.children {
                let name = element_name(child, nodes, groups);
                if let Some(&(cx, cy, cw, ch)) = positions.get(&name) {
                    min_x = min_x.min(cx);
                    min_y = min_y.min(cy);
                    max_x = max_x.max(cx + cw);
                    max_y = max_y.max(cy + ch);
                }
            }

            if min_x < f64::MAX {
                // Add padding for group border and header
                let gx = min_x - GROUP_H_PAD;
                let gy = min_y - GROUP_HEADER_H - GROUP_V_PAD;
                let gw = (max_x - min_x + GROUP_H_PAD * 2.0)
                    .max(text_width(&g.name) + GROUP_H_PAD * 2.0);
                let gh = max_y - min_y + GROUP_HEADER_H + GROUP_V_PAD * 2.0;
                positions.insert(g.name.clone(), (gx, gy, gw, gh));
            }
        }
    }
}

/// Find x-shifts needed to eliminate overlaps between sibling elements (groups and standalone nodes).
/// `elem_sort_keys` maps element name → sort key (e.g. avg x of highest-rank nodes).
/// Returns a map of node_name → dx shift for all nodes that need to move.
fn find_cluster_shifts(
    elements: &[Element],
    nodes: &[Node],
    groups: &[Group],
    positions: &HashMap<String, (f64, f64, f64, f64)>,
    elem_sort_keys: &HashMap<String, f64>,
) -> HashMap<String, f64> {
    let mut shifts: HashMap<String, f64> = HashMap::new();
    let gap = GROUP_H_PAD;

    // Collect sibling element bounds
    let mut sibling_bounds: Vec<(usize, String, f64, f64, f64, f64)> = Vec::new();
    for (i, elem) in elements.iter().enumerate() {
        let name = element_name(elem, nodes, groups);
        if let Some(&(x, y, w, h)) = positions.get(&name) {
            sibling_bounds.push((i, name, x, y, w, h));
        }
    }
    // Sort by precomputed sort key (rank-based node positions), falling back to bbox center
    sibling_bounds.sort_by(|a, b| {
        let ka = elem_sort_keys.get(&a.1).copied().unwrap_or(a.2 + a.4 / 2.0);
        let kb = elem_sort_keys.get(&b.1).copied().unwrap_or(b.2 + b.4 / 2.0);
        ka.partial_cmp(&kb).unwrap()
    });

    // For each consecutive pair in intended order, ensure no overlap.
    // Push the right element further right if needed.
    for idx in 0..sibling_bounds.len().saturating_sub(1) {
        let (_, _, ax, ay, aw, ah) = sibling_bounds[idx];
        let (ei, _, bx, by, _bw, bh) = sibling_bounds[idx + 1].clone();

        // Check if they overlap vertically (y ranges intersect)
        let y_overlap = ay < by + bh && by < ay + ah;
        if !y_overlap {
            continue;
        }

        let needed_x = ax + aw + gap;
        if bx < needed_x {
            let dx = needed_x - bx;
            collect_node_shifts(&elements[ei], nodes, groups, dx, &mut shifts);
            // Update this entry's x for cascading checks
            sibling_bounds[idx + 1].2 += dx;
        }
    }

    // Recurse into groups to fix overlaps among their children
    for elem in elements {
        if let Element::GroupRef(gi) = elem {
            let child_shifts = find_cluster_shifts(&groups[*gi].children, nodes, groups, positions, elem_sort_keys);
            for (name, dx) in child_shifts {
                *shifts.entry(name).or_insert(0.0) += dx;
            }
        }
    }

    shifts
}

/// Collect all node names under an element and assign them a dx shift.
fn collect_node_shifts(
    elem: &Element,
    nodes: &[Node],
    groups: &[Group],
    dx: f64,
    shifts: &mut HashMap<String, f64>,
) {
    match elem {
        Element::NodeRef(ni) => {
            *shifts.entry(nodes[*ni].name.clone()).or_insert(0.0) += dx;
        }
        Element::GroupRef(gi) => {
            for child in &groups[*gi].children {
                collect_node_shifts(child, nodes, groups, dx, shifts);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Edge routing
// ---------------------------------------------------------------------------

fn segment_intersects_rect(
    p1x: f64, p1y: f64, p2x: f64, p2y: f64,
    cx: f64, cy: f64, hw: f64, hh: f64,
) -> bool {
    let margin = 4.0;
    let left = cx - hw - margin;
    let right = cx + hw + margin;
    let top = cy - hh - margin;
    let bottom = cy + hh + margin;

    if (p1x < left && p2x < left) || (p1x > right && p2x > right) {
        return false;
    }
    if (p1y < top && p2y < top) || (p1y > bottom && p2y > bottom) {
        return false;
    }

    let dx = p2x - p1x;
    let dy = p2y - p1y;
    let edges: [(f64, f64, f64, f64); 4] = [
        (left, top, right, top),
        (left, bottom, right, bottom),
        (left, top, left, bottom),
        (right, top, right, bottom),
    ];

    for (ex1, ey1, ex2, ey2) in &edges {
        let edx = ex2 - ex1;
        let edy = ey2 - ey1;
        let denom = dx * edy - dy * edx;
        if denom.abs() < 1e-10 { continue; }
        let t = ((ex1 - p1x) * edy - (ey1 - p1y) * edx) / denom;
        let u = ((ex1 - p1x) * dy - (ey1 - p1y) * dx) / denom;
        if (0.01..=0.99).contains(&t) && (0.0..=1.0).contains(&u) {
            return true;
        }
    }
    false
}

fn route_around_nodes(
    sx: f64, sy: f64, ex: f64, ey: f64,
    from_name: &str, to_name: &str,
    all_bounds: &[(String, f64, f64, f64, f64)], // (name, cx, cy, hw, hh)
    offset: f64,
) -> Vec<(f64, f64)> {
    let (sx, sy, ex, ey) = if offset.abs() > 0.1 {
        let dx = ex - sx;
        let dy = ey - sy;
        let len = (dx * dx + dy * dy).sqrt().max(1.0);
        let nx = -dy / len * offset;
        let ny = dx / len * offset;
        (sx + nx, sy + ny, ex + nx, ey + ny)
    } else {
        (sx, sy, ex, ey)
    };

    let mut blockers: Vec<usize> = Vec::new();
    for (i, (name, cx, cy, hw, hh)) in all_bounds.iter().enumerate() {
        if name == from_name || name == to_name { continue; }
        if segment_intersects_rect(sx, sy, ex, ey, *cx, *cy, *hw, *hh) {
            blockers.push(i);
        }
    }

    if blockers.is_empty() {
        return vec![(sx, sy), (ex, ey)];
    }

    let margin = 20.0;
    let mut waypoints: Vec<(f64, f64)> = vec![(sx, sy)];

    blockers.sort_by(|a, b| {
        let (_, acx, acy, _, _) = all_bounds[*a];
        let (_, bcx, bcy, _, _) = all_bounds[*b];
        let da = (acx - sx).powi(2) + (acy - sy).powi(2);
        let db = (bcx - sx).powi(2) + (bcy - sy).powi(2);
        da.partial_cmp(&db).unwrap()
    });

    for &bi in &blockers {
        let (_, cx, cy, hw, hh) = all_bounds[bi];
        let dx = ex - sx;
        let dy = ey - sy;
        let len = (dx * dx + dy * dy).sqrt().max(1.0);
        let last = waypoints.last().unwrap();
        let cross = (cx - last.0) * dy - (cy - last.1) * dx;

        if cross.abs() / len < hw + hh {
            if dy.abs() > dx.abs() {
                if cross > 0.0 { waypoints.push((cx + hw + margin, cy)); }
                else { waypoints.push((cx - hw - margin, cy)); }
            } else if cross > 0.0 { waypoints.push((cx, cy - hh - margin)); }
            else { waypoints.push((cx, cy + hh + margin)); }
        }
    }

    waypoints.push((ex, ey));
    waypoints
}

fn build_smooth_path(points: &[(f64, f64)]) -> String {
    if points.len() < 2 { return String::new(); }
    if points.len() == 2 {
        return format!("M{},{} L{},{}", points[0].0, points[0].1, points[1].0, points[1].1);
    }
    let mut d = format!("M{},{}", points[0].0, points[0].1);
    for i in 1..points.len() - 1 {
        let prev = points[i - 1];
        let curr = points[i];
        let next = points[i + 1];
        let mid_prev = ((prev.0 + curr.0) / 2.0, (prev.1 + curr.1) / 2.0);
        let mid_next = ((curr.0 + next.0) / 2.0, (curr.1 + next.1) / 2.0);
        if i == 1 { d.push_str(&format!(" L{},{}", mid_prev.0, mid_prev.1)); }
        d.push_str(&format!(" Q{},{} {},{}", curr.0, curr.1, mid_next.0, mid_next.1));
    }
    let last = points[points.len() - 1];
    d.push_str(&format!(" L{},{}", last.0, last.1));
    d
}

fn sample_smooth_path(points: &[(f64, f64)], n: usize) -> Vec<(f64, f64)> {
    if points.len() < 2 { return points.to_vec(); }
    if points.len() == 2 {
        return (0..=n).map(|i| {
            let t = i as f64 / n as f64;
            (points[0].0 + (points[1].0 - points[0].0) * t,
             points[0].1 + (points[1].1 - points[0].1) * t)
        }).collect();
    }
    let mut segments: Vec<((f64, f64), (f64, f64), (f64, f64))> = Vec::new();
    let mut cursor = points[0];
    for i in 1..points.len() - 1 {
        let prev = points[i - 1]; let curr = points[i]; let next = points[i + 1];
        let mid_prev = ((prev.0 + curr.0) / 2.0, (prev.1 + curr.1) / 2.0);
        let mid_next = ((curr.0 + next.0) / 2.0, (curr.1 + next.1) / 2.0);
        if i == 1 { segments.push((cursor, cursor, mid_prev)); cursor = mid_prev; }
        segments.push((cursor, curr, mid_next)); cursor = mid_next;
    }
    let last = *points.last().unwrap();
    segments.push((cursor, cursor, last));
    let per_seg = (n / segments.len()).max(2);
    let mut result = Vec::new();
    for (start, ctrl, end) in &segments {
        for j in 0..per_seg {
            let t = j as f64 / per_seg as f64;
            let mt = 1.0 - t;
            result.push((mt*mt*start.0 + 2.0*mt*t*ctrl.0 + t*t*end.0,
                         mt*mt*start.1 + 2.0*mt*t*ctrl.1 + t*t*end.1));
        }
    }
    result.push(last);
    result
}

fn midpoint_on_path(points: &[(f64, f64)]) -> (f64, f64) {
    if points.len() <= 1 { return points.first().copied().unwrap_or((0.0, 0.0)); }
    if points.len() == 2 {
        return ((points[0].0 + points[1].0) / 2.0, (points[0].1 + points[1].1) / 2.0);
    }
    let samples = sample_smooth_path(points, 64);
    let mut lengths = vec![0.0_f64];
    for i in 1..samples.len() {
        let dx = samples[i].0 - samples[i-1].0;
        let dy = samples[i].1 - samples[i-1].1;
        lengths.push(lengths[i-1] + (dx*dx + dy*dy).sqrt());
    }
    let half = *lengths.last().unwrap() / 2.0;
    for i in 1..lengths.len() {
        if lengths[i] >= half {
            let t = (half - lengths[i-1]) / (lengths[i] - lengths[i-1]).max(1e-10);
            return (samples[i-1].0 + (samples[i].0 - samples[i-1].0) * t,
                    samples[i-1].1 + (samples[i].1 - samples[i-1].1) * t);
        }
    }
    *samples.last().unwrap()
}

fn clip_to_rect(cx: f64, cy: f64, tx: f64, ty: f64, hw: f64, hh: f64) -> (f64, f64) {
    let dx = tx - cx; let dy = ty - cy;
    if dx.abs() < 1e-9 && dy.abs() < 1e-9 { return (cx, cy); }
    let mut t = f64::MAX;
    if dx.abs() > 1e-9 { t = t.min(hw / dx.abs()); }
    if dy.abs() > 1e-9 { t = t.min(hh / dy.abs()); }
    (cx + dx * t, cy + dy * t)
}

// ---------------------------------------------------------------------------
// SVG rendering
// ---------------------------------------------------------------------------

fn render_svg(diagram: &Diagram) -> String {
    let (positions, edge_waypoints) = compound_layout(diagram);

    // SVG dimensions
    let mut max_x: f64 = 0.0;
    let mut max_y: f64 = 0.0;
    for (_, (x, y, w, h)) in &positions {
        max_x = max_x.max(x + w);
        max_y = max_y.max(y + h);
    }

    let svg_width = max_x + PADDING;
    let svg_height = max_y + PADDING;

    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">",
        svg_width, svg_height, svg_width, svg_height
    );
    svg.push_str(&format!(
        "<rect width=\"100%\" height=\"100%\" fill=\"white\"/><style>text {{ font-family: sans-serif; font-size: 12px; fill: {}; }}</style>",
        COLOR_DARK
    ));
    svg.push_str(&format!(
        "<defs><marker id=\"arrow\" viewBox=\"0 0 10 10\" refX=\"10\" refY=\"5\" markerWidth=\"8\" markerHeight=\"8\" orient=\"auto\"><polygon points=\"0,1 10,5 0,9\" fill=\"{}\"/></marker></defs>",
        COLOR_EDGE
    ));

    // Render groups (back to front)
    render_groups_recursive(&mut svg, &diagram.top_level, &diagram.nodes, &diagram.groups, &positions, diagram.show_type);

    // Build node bounds for edge routing (nodes only, not groups —
    // edges are allowed to cross group borders since they're dashed)
    let all_bounds: Vec<(String, f64, f64, f64, f64)> = diagram
        .nodes
        .iter()
        .filter_map(|n| {
            positions.get(&n.name).map(|(x, y, w, h)| {
                (n.name.clone(), x + w / 2.0, y + h / 2.0, w / 2.0, h / 2.0)
            })
        })
        .collect();

    // Reciprocal edge counting
    let mut pair_count: HashMap<(String, String), usize> = HashMap::new();
    for e in &diagram.edges {
        let key = if e.from <= e.to { (e.from.clone(), e.to.clone()) } else { (e.to.clone(), e.from.clone()) };
        *pair_count.entry(key).or_insert(0) += 1;
    }
    let mut pair_seen: HashMap<(String, String), usize> = HashMap::new();

    // Render edges
    for edge in &diagram.edges {
        let from_pos = positions.get(&edge.from);
        let to_pos = positions.get(&edge.to);
        if from_pos.is_none() || to_pos.is_none() { continue; }

        let (fx, fy, fw, fh) = *from_pos.unwrap();
        let (tx, ty, tw, th) = *to_pos.unwrap();

        let cx1 = fx + fw / 2.0;
        let cy1 = fy + fh / 2.0;
        let cx2 = tx + tw / 2.0;
        let cy2 = ty + th / 2.0;

        let pair_key = if edge.from <= edge.to { (edge.from.clone(), edge.to.clone()) } else { (edge.to.clone(), edge.from.clone()) };
        let total = *pair_count.get(&pair_key).unwrap_or(&1);
        let idx = { let seen = pair_seen.entry(pair_key).or_insert(0); let v = *seen; *seen += 1; v };
        let offset = if total > 1 { (idx as f64 - (total as f64 - 1.0) / 2.0) * 15.0 } else { 0.0 };

        // Use virtual node waypoints if available, otherwise route around nodes
        let edge_key = format!("{}→{}", edge.from, edge.to);
        let route = if let Some(waypoints) = edge_waypoints.get(&edge_key) {
            // Build route: start → waypoints → end
            let mut r = vec![(cx1, cy1)];
            r.extend(waypoints);
            r.push((cx2, cy2));
            r
        } else {
            route_around_nodes(cx1, cy1, cx2, cy2, &edge.from, &edge.to, &all_bounds, offset)
        };

        let start_target = if route.len() > 1 { route[1] } else { (cx2, cy2) };
        let end_target = if route.len() > 1 { route[route.len() - 2] } else { (cx1, cy1) };
        let (ax1, ay1) = clip_to_rect(cx1, cy1, start_target.0, start_target.1, fw / 2.0, fh / 2.0);
        let (ax2, ay2) = clip_to_rect(cx2, cy2, end_target.0, end_target.1, tw / 2.0, th / 2.0);

        let mut clipped = vec![(ax1, ay1)];
        if route.len() > 2 { clipped.extend_from_slice(&route[1..route.len()-1]); }
        clipped.push((ax2, ay2));

        let path_d = if clipped.len() == 2 {
            format!("M{},{} L{},{}", clipped[0].0, clipped[0].1, clipped[1].0, clipped[1].1)
        } else {
            build_smooth_path(&clipped)
        };

        svg.push_str(&format!(
            "<path d=\"{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"1.5\" marker-end=\"url(#arrow)\"/>",
            path_d, COLOR_EDGE
        ));

        if !edge.label.is_empty() {
            let (mx, my) = midpoint_on_path(&clipped);
            let lw = text_width(&edge.label);
            svg.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"16\" rx=\"3\" fill=\"white\" opacity=\"0.9\"/>",
                mx - lw / 2.0 - 3.0, my - 18.0, lw + 6.0
            ));
            svg.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-size=\"11\" fill=\"{}\">{}</text>",
                mx, my - 6.0, COLOR_EDGE, escape_xml(&edge.label)
            ));
        }
    }

    svg.push_str("</svg>");
    svg
}

fn render_groups_recursive(
    svg: &mut String,
    elements: &[Element],
    nodes: &[Node],
    groups: &[Group],
    positions: &HashMap<String, (f64, f64, f64, f64)>,
    show_type: bool,
) {
    for elem in elements {
        match elem {
            Element::GroupRef(gi) => {
                let g = &groups[*gi];
                if let Some(&(x, y, w, h)) = positions.get(&g.name) {
                    // Group background
                    svg.push_str(&format!(
                        "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"6\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\" stroke-dasharray=\"6,4\"/>",
                        x, y, w, h, COLOR_GROUP_FILL, COLOR_GROUP_STROKE
                    ));
                    // Group label
                    svg.push_str(&format!(
                        "<text x=\"{}\" y=\"{}\" font-weight=\"bold\" font-size=\"13\">{}</text>",
                        x + 8.0, y + GROUP_HEADER_H * 0.7, escape_xml(&g.name)
                    ));
                }
                // Recurse into children
                render_groups_recursive(svg, &g.children, nodes, groups, positions, show_type);
            }
            Element::NodeRef(ni) => {
                let node = &nodes[*ni];
                if let Some(&(x, y, w, h)) = positions.get(&node.name) {
                    render_node(svg, x, y, w, h, node, show_type);
                }
            }
        }
    }
}

fn render_node(svg: &mut String, x: f64, y: f64, w: f64, h: f64, node: &Node, show_type: bool) {
    let (fill, stroke) = node_colors(&node.node_type);

    svg.push_str(&format!(
        "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"6\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
        x, y, w, h, fill, stroke
    ));

    // Draw icon shape
    let icon_cx = x + w / 2.0;
    let icon_cy = y + 22.0;
    render_icon(svg, icon_cx, icon_cy, &node.node_type, stroke);

    // Label
    svg.push_str(&format!(
        "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-size=\"11\">{}</text>",
        x + w / 2.0,
        y + h - 8.0,
        escape_xml(&node.name)
    ));

    // Type badge
    if show_type {
        let type_label = format!("{:?}", node.node_type).to_lowercase();
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-size=\"9\" fill=\"{}\">{}</text>",
            x + w / 2.0,
            y + h - 20.0,
            stroke,
            type_label
        ));
    }
}

fn render_icon(svg: &mut String, cx: f64, cy: f64, nt: &NodeType, color: &str) {
    let r = ICON_SIZE / 2.0;
    match nt {
        NodeType::Server => {
            // Server rack: stacked rectangles
            for i in 0..3 {
                let ry = cy - r + 2.0 + i as f64 * 10.0;
                svg.push_str(&format!(
                    "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"8\" rx=\"1\" fill=\"none\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                    cx - r * 0.6, ry, r * 1.2, color
                ));
            }
        }
        NodeType::Db => {
            // Cylinder
            let rw = r * 0.6;
            let rh = r * 0.8;
            let ell_h = 5.0;
            svg.push_str(&format!(
                "<ellipse cx=\"{}\" cy=\"{}\" rx=\"{}\" ry=\"{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                cx, cy - rh + ell_h, rw, ell_h, color
            ));
            svg.push_str(&format!(
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                cx - rw, cy - rh + ell_h, cx - rw, cy + rh - ell_h, color
            ));
            svg.push_str(&format!(
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                cx + rw, cy - rh + ell_h, cx + rw, cy + rh - ell_h, color
            ));
            svg.push_str(&format!(
                "<ellipse cx=\"{}\" cy=\"{}\" rx=\"{}\" ry=\"{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                cx, cy + rh - ell_h, rw, ell_h, color
            ));
        }
        NodeType::Lb => {
            // Load balancer: circle with arrows
            svg.push_str(&format!(
                "<circle cx=\"{}\" cy=\"{}\" r=\"{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                cx, cy, r * 0.5, color
            ));
            svg.push_str(&format!(
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                cx - r * 0.5, cy, cx - r * 0.9, cy - 6.0, color
            ));
            svg.push_str(&format!(
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                cx - r * 0.5, cy, cx - r * 0.9, cy + 6.0, color
            ));
        }
        NodeType::Cache => {
            // Lightning bolt
            svg.push_str(&format!(
                "<polyline points=\"{},{} {},{} {},{} {},{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"2\"/>",
                cx - 4.0, cy - r * 0.6,
                cx + 2.0, cy - 2.0,
                cx - 2.0, cy + 2.0,
                cx + 4.0, cy + r * 0.6,
                color
            ));
        }
        NodeType::Queue => {
            // Arrow right
            svg.push_str(&format!(
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"2\"/>",
                cx - r * 0.5, cy, cx + r * 0.3, cy, color
            ));
            svg.push_str(&format!(
                "<polyline points=\"{},{} {},{} {},{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"2\"/>",
                cx + r * 0.1, cy - 5.0, cx + r * 0.5, cy, cx + r * 0.1, cy + 5.0, color
            ));
        }
        NodeType::Storage => {
            // Bucket shape
            svg.push_str(&format!(
                "<path d=\"M{},{} L{},{} L{},{} L{},{} Z\" fill=\"none\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                cx - r * 0.5, cy - r * 0.5,
                cx + r * 0.5, cy - r * 0.5,
                cx + r * 0.4, cy + r * 0.5,
                cx - r * 0.4, cy + r * 0.5,
                color
            ));
        }
        NodeType::Cdn => {
            // Cloud shape
            svg.push_str(&format!(
                "<circle cx=\"{}\" cy=\"{}\" r=\"6\" fill=\"none\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                cx - 5.0, cy + 2.0, color
            ));
            svg.push_str(&format!(
                "<circle cx=\"{}\" cy=\"{}\" r=\"7\" fill=\"none\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                cx, cy - 3.0, color
            ));
            svg.push_str(&format!(
                "<circle cx=\"{}\" cy=\"{}\" r=\"6\" fill=\"none\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                cx + 5.0, cy + 2.0, color
            ));
        }
        NodeType::Network => {
            // Hexagon
            let s = r * 0.5;
            svg.push_str(&format!(
                "<polygon points=\"{},{} {},{} {},{} {},{} {},{} {},{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                cx, cy - s,
                cx + s * 0.87, cy - s * 0.5,
                cx + s * 0.87, cy + s * 0.5,
                cx, cy + s,
                cx - s * 0.87, cy + s * 0.5,
                cx - s * 0.87, cy - s * 0.5,
                color
            ));
        }
        NodeType::User => {
            // Stick figure
            svg.push_str(&format!(
                "<circle cx=\"{}\" cy=\"{}\" r=\"5\" fill=\"none\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                cx, cy - 8.0, color
            ));
            svg.push_str(&format!(
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                cx, cy - 3.0, cx, cy + 6.0, color
            ));
            svg.push_str(&format!(
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                cx - 7.0, cy + 1.0, cx + 7.0, cy + 1.0, color
            ));
        }
        NodeType::Phone => {
            // Landline phone: base + handset
            // Base
            svg.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"2\" fill=\"none\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                cx - r * 0.55, cy + 1.0, r * 1.1, r * 0.5, color
            ));
            // Handset (receiver arc)
            svg.push_str(&format!(
                "<path d=\"M{},{} Q{},{} {},{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"2.5\" stroke-linecap=\"round\"/>",
                cx - r * 0.45, cy + 1.0,
                cx, cy - r * 0.7,
                cx + r * 0.45, cy + 1.0,
                color
            ));
        }
        NodeType::Cloud => {
            // Network cloud shape using arcs
            let w = r * 0.8;
            let h = r * 0.5;
            svg.push_str(&format!(
                "<path d=\"M{},{} \
                 a{},{} 0 0,1 {},{} \
                 a{},{} 0 0,1 {},{} \
                 a{},{} 0 0,1 {},{} \
                 a{},{} 0 0,1 {},{} \
                 a{},{} 0 0,1 {},{}\" \
                 fill=\"none\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                cx - w * 0.6, cy + h * 0.4,
                // bottom-left bump
                h * 0.7, h * 0.7, w * 0.1, -(h * 0.8),
                // top-left bump
                h * 0.6, h * 0.6, w * 0.5, -(h * 0.3),
                // top-right bump
                h * 0.7, h * 0.7, w * 0.6, h * 0.2,
                // right bump
                h * 0.6, h * 0.6, w * 0.0, h * 0.9,
                // bottom line back
                h * 0.3, h * 0.3, -(w * 1.2), 0.0,
                color
            ));
        }
        NodeType::Generic => {
            // Simple box
            svg.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"3\" fill=\"none\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                cx - r * 0.5, cy - r * 0.4, r, r * 0.8, color
            ));
        }
    }
}

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

const HELP: &str = "\
mdd-infra - Render an infrastructure diagram as SVG

Usage: mdd-infra < input.infra

Define nodes with \"node Name [type=TYPE]\" where TYPE is one of:
server, db, lb, cache, queue, storage, cdn, network, user.
Group nodes with \"group \"Name\" { ... }\" (nesting allowed).
Connect nodes with \"A -> B\" or \"A -> B : \"label\"\".

Example:
  node Client type=user
  node WebServer type=server
  node Database type=db
  Client -> WebServer : \"HTTP\"
  WebServer -> Database : \"SQL\"
";

fn main() {
    if std::env::args().any(|a| a == "--help" || a == "-h") {
        eprint!("{}", HELP);
        return;
    }
    let mut input = String::new();
    io::stdin()
        .read_to_string(&mut input)
        .expect("Failed to read stdin");

    let diagram = match parse(&input) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("mdd-infra: {}", e);
            std::process::exit(1);
        }
    };

    let svg = render_svg(&diagram);
    print!("{}", svg);
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_node_with_type() {
        let d = parse("node App type=server\n").unwrap();
        assert_eq!(d.nodes.len(), 1);
        assert_eq!(d.nodes[0].node_type, NodeType::Server);
    }

    #[test]
    fn parse_node_generic() {
        let d = parse("node Foo\n").unwrap();
        assert_eq!(d.nodes[0].node_type, NodeType::Generic);
    }

    #[test]
    fn parse_group() {
        let input = "group \"VPC\" {\n  node App type=server\n  node DB type=db\n}\n";
        let d = parse(input).unwrap();
        assert_eq!(d.groups.len(), 1);
        assert_eq!(d.groups[0].children.len(), 2);
    }

    #[test]
    fn parse_nested_groups() {
        let input = "group \"AWS\" {\n  group \"VPC\" {\n    node App type=server\n  }\n}\n";
        let d = parse(input).unwrap();
        assert_eq!(d.groups.len(), 2);
    }

    #[test]
    fn parse_edge() {
        let input = "node A\nnode B\nA -> B : \"HTTP\"\n";
        let d = parse(input).unwrap();
        assert_eq!(d.edges.len(), 1);
        assert_eq!(d.edges[0].label, "HTTP");
    }

    #[test]
    fn render_produces_svg() {
        let input = "node A type=server\nnode B type=db\nA -> B\n";
        let d = parse(input).unwrap();
        let svg = render_svg(&d);
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("</svg>"));
    }
}
