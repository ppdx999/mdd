use std::collections::HashMap;

use crate::order::order_ranks;
use crate::rank::{assign_ranks, flatten_nodes};
use crate::text::text_width;
use crate::types::*;

/// Assign coordinates and compute cluster boundaries.
/// Returns `LayoutResult { positions, edge_waypoints }`.
pub fn compound_layout(graph: &LayoutGraph, config: &LayoutConfig) -> LayoutResult {
    let mut positions: HashMap<String, (f64, f64, f64, f64)> = HashMap::new();
    let mut edge_waypoints: HashMap<String, Vec<(f64, f64)>> = HashMap::new();

    // Phase 1: Flatten
    let real_nodes = flatten_nodes(
        &graph.top_level,
        &graph.nodes,
        &graph.groups,
        &[],
    );

    if real_nodes.is_empty() {
        return LayoutResult { positions, edge_waypoints };
    }

    let real_count = real_nodes.len();

    // Phase 2: Rank assignment (real nodes only)
    let real_ranks = assign_ranks(&real_nodes, graph);

    // Phase 2.5: Insert virtual nodes for long edges
    // flat_nodes = real_nodes + virtual_nodes
    // Virtual nodes have node_index = usize::MAX and inherit the cluster chain
    // of the edge's source (arbitrary but reasonable choice).
    let mut flat_nodes: Vec<FlatNode> = real_nodes.clone();
    let mut ranks: Vec<usize> = real_ranks.clone();

    // Build name→flat_index for real nodes
    let mut name_to_flat: HashMap<&str, usize> = HashMap::new();
    for (fi, fnode) in real_nodes.iter().enumerate() {
        name_to_flat.insert(&graph.nodes[fnode.node_index].name, fi);
    }

    // Build graph edges (index-based), splitting long edges with virtual nodes
    let mut graph_edges: Vec<(usize, usize)> = Vec::new();
    // Track which virtual nodes belong to which original edge: edge_key → vec of virtual fi
    let mut virtual_chains: HashMap<String, Vec<usize>> = HashMap::new();

    for edge in &graph.edges {
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
                let cluster_chain = flat_nodes[from].cluster_chain.clone();
                for r in (from_rank + 1)..to_rank {
                    let vi = flat_nodes.len();
                    flat_nodes.push(FlatNode {
                        node_index: usize::MAX,
                        cluster_chain: cluster_chain.clone(),
                    });
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
    // Compute spacing from edge label lengths so labels fit between nodes
    let max_label_w = graph.edges.iter()
        .filter(|e| !e.label.is_empty())
        .map(|e| text_width(&e.label))
        .fold(0.0_f64, f64::max);
    // Compute average node dimensions for spacing heuristic
    let avg_node_w = if real_count > 0 {
        graph.nodes.iter().take(real_count).map(|n| n.width).sum::<f64>() / real_count as f64
    } else {
        config.default_node_w
    };
    let avg_node_h = if real_count > 0 {
        graph.nodes.iter().take(real_count).map(|n| n.height).sum::<f64>() / real_count as f64
    } else {
        config.default_node_h
    };

    let rank_sep = if config.rank_sep > 0.0 {
        config.rank_sep
    } else {
        // Enough room for edge labels + proportional to node height
        (60.0_f64).max(max_label_w * 0.5 + 20.0).max(avg_node_h * 0.5 + 30.0)
    };
    let node_sep = if config.node_sep > 0.0 {
        config.node_sep
    } else {
        // Enough room for labels + proportional to node width
        (30.0_f64).max(max_label_w * 0.4 + 10.0).max(avg_node_w * 0.3 + 10.0)
    };
    let cluster_sep = config.group_h_pad * 2.0 + 16.0;
    let virtual_w = 2.0; // virtual nodes are thin

    // Width of a flat node (real or virtual)
    let node_w = |fi: usize| -> f64 {
        if fi < real_count && !flat_nodes[fi].is_virtual() {
            graph.nodes[flat_nodes[fi].node_index].width
        } else {
            virtual_w
        }
    };

    // Compute the minimum spacing between two adjacent nodes in a rank.
    let spacing = |a: usize, b: usize| -> f64 {
        let ca = &flat_nodes[a].cluster_chain;
        let cb = &flat_nodes[b].cluster_chain;
        if ca == cb {
            node_sep
        } else {
            let shared = ca.iter().zip(cb.iter()).take_while(|(x, y)| x == y).count();
            let boundaries = (ca.len() - shared) + (cb.len() - shared);
            node_sep + cluster_sep * (boundaries as f64).max(1.0)
        }
    };

    // Compute rank heights, accounting for cluster headers.
    let node_h = config.default_node_h;
    let rank_height: Vec<f64> = vec![node_h; max_rank + 1];

    let mut cluster_min_rank: HashMap<usize, usize> = HashMap::new();
    for (fi, fnode) in flat_nodes.iter().enumerate() {
        if fnode.is_virtual() { continue; } // skip virtual for cluster rank computation
        for &gi in &fnode.cluster_chain {
            let entry = cluster_min_rank.entry(gi).or_insert(ranks[fi]);
            *entry = (*entry).min(ranks[fi]);
        }
    }

    let mut rank_extra_top: Vec<f64> = vec![0.0; max_rank + 1];
    for (_, &min_r) in &cluster_min_rank {
        rank_extra_top[min_r] += config.group_header_h + config.group_v_pad;
    }
    for extra in &mut rank_extra_top {
        *extra = extra.min(config.group_header_h * 2.0 + config.group_v_pad * 2.0);
    }

    // Y coordinate per rank
    let mut rank_y: Vec<f64> = vec![0.0; max_rank + 1];
    rank_y[0] = config.padding + rank_extra_top[0];
    for r in 1..=max_rank {
        rank_y[r] = rank_y[r - 1] + rank_height[r - 1] + rank_sep + rank_extra_top[r];
    }

    // X coordinate: place nodes left to right within each rank
    let mut node_x: Vec<f64> = vec![0.0; flat_nodes.len()];
    let mut node_y: Vec<f64> = vec![0.0; flat_nodes.len()];

    for r in 0..=max_rank {
        let mut x_cursor = config.padding;
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
                    config.padding
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

            // Enforce minimum spacing: left-to-right sweep to fix any overlaps
            for i in 1..bucket.len() {
                let prev = bucket[i - 1];
                let fi = bucket[i];
                let min_x = node_x[prev] + node_w(prev) + spacing(prev, fi);
                if node_x[fi] < min_x {
                    node_x[fi] = min_x;
                }
            }
        }
    }

    // Store real node positions only
    for fi in 0..real_count {
        let ni = flat_nodes[fi].node_index;
        positions.insert(
            graph.nodes[ni].name.clone(),
            (node_x[fi], node_y[fi], node_w(fi), node_h),
        );
    }

    // Build edge waypoints from virtual node positions
    for (edge_key, chain) in &virtual_chains {
        let waypoints: Vec<(f64, f64)> = chain
            .iter()
            .map(|&vi| (node_x[vi] + virtual_w / 2.0, node_y[vi] + node_h / 2.0))
            .collect();
        edge_waypoints.insert(edge_key.clone(), waypoints);
    }

    // Phase 5: Compute cluster boundaries from node positions
    compute_cluster_bounds(
        &graph.top_level,
        &graph.nodes,
        &graph.groups,
        config,
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
            elem: &LayoutElement, nodes_list: &[LayoutNode], groups: &[LayoutGroup],
            flat_nodes: &[FlatNode], node_x: &[f64], ranks: &[usize],
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
            elem: &LayoutElement, nodes_list: &[LayoutNode], groups: &[LayoutGroup],
            flat_nodes: &[FlatNode], real_count: usize,
            out: &mut Vec<usize>,
        ) {
            match elem {
                LayoutElement::NodeRef(ni) => {
                    for fi in 0..real_count {
                        if flat_nodes[fi].node_index == *ni {
                            out.push(fi);
                        }
                    }
                }
                LayoutElement::GroupRef(gi) => {
                    for child in &groups[*gi].children {
                        collect_flat_indices(child, nodes_list, groups, flat_nodes, real_count, out);
                    }
                }
            }
        }

        for elem in &graph.top_level {
            let name = element_name(elem, &graph.nodes, &graph.groups);
            let key = compute_sort_key(elem, &graph.nodes, &graph.groups, &flat_nodes, &node_x, &ranks, real_count);
            elem_sort_keys.insert(name, key);
        }
    }

    for _ in 0..5 {
        let shifts = find_cluster_shifts(
            &graph.top_level,
            &graph.nodes,
            &graph.groups,
            &positions,
            &elem_sort_keys,
            config,
        );
        if shifts.is_empty() {
            break;
        }
        for fi in 0..real_count {
            let ni = flat_nodes[fi].node_index;
            if let Some(&dx) = shifts.get(&graph.nodes[ni].name) {
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
            let ni = flat_nodes[fi].node_index;
            positions.insert(
                graph.nodes[ni].name.clone(),
                (node_x[fi], node_y[fi], node_w(fi), node_h),
            );
        }
        // Update waypoints
        for (edge_key, chain) in &virtual_chains {
            let waypoints: Vec<(f64, f64)> = chain
                .iter()
                .map(|&vi| (node_x[vi] + virtual_w / 2.0, node_y[vi] + node_h / 2.0))
                .collect();
            edge_waypoints.insert(edge_key.clone(), waypoints);
        }
        compute_cluster_bounds(
            &graph.top_level,
            &graph.nodes,
            &graph.groups,
            config,
            &mut positions,
        );
    }

    // Phase 7: Post-overlap barycenter re-adjustment.
    // Only adjust standalone nodes (not in any cluster) and virtual nodes,
    // since cluster members have been correctly positioned by Phase 6.
    for _pass in 0..4 {
        for r in 0..=max_rank {
            let bucket = &rank_buckets[r];
            if bucket.is_empty() { continue; }

            for (idx_in_bucket, &fi) in bucket.iter().enumerate() {
                // Only adjust standalone nodes and virtual nodes
                if fi < real_count && !flat_nodes[fi].cluster_chain.is_empty() {
                    continue; // skip cluster member nodes
                }

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
                    config.padding
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

            // Enforce minimum spacing: left-to-right sweep to fix any overlaps
            for i in 1..bucket.len() {
                let prev = bucket[i - 1];
                let fi = bucket[i];
                let min_x = node_x[prev] + node_w(prev) + spacing(prev, fi);
                if node_x[fi] < min_x {
                    node_x[fi] = min_x;
                }
            }
        }
    }

    // Phase 8: Re-run overlap correction after Phase 7's barycenter re-adjustment.
    // Phase 7 may have pulled standalone nodes back into cluster bounding boxes.
    positions.clear();
    for fi in 0..real_count {
        let ni = flat_nodes[fi].node_index;
        positions.insert(
            graph.nodes[ni].name.clone(),
            (node_x[fi], node_y[fi], node_w(fi), node_h),
        );
    }
    compute_cluster_bounds(
        &graph.top_level,
        &graph.nodes,
        &graph.groups,
        config,
        &mut positions,
    );
    for _ in 0..3 {
        let shifts = find_cluster_shifts(
            &graph.top_level,
            &graph.nodes,
            &graph.groups,
            &positions,
            &elem_sort_keys,
            config,
        );
        if shifts.is_empty() {
            break;
        }
        for fi in 0..real_count {
            let ni = flat_nodes[fi].node_index;
            if let Some(&dx) = shifts.get(&graph.nodes[ni].name) {
                node_x[fi] += dx;
            }
        }
        for (edge_key, chain) in &virtual_chains {
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
            let ni = flat_nodes[fi].node_index;
            positions.insert(
                graph.nodes[ni].name.clone(),
                (node_x[fi], node_y[fi], node_w(fi), node_h),
            );
        }
        compute_cluster_bounds(
            &graph.top_level,
            &graph.nodes,
            &graph.groups,
            config,
            &mut positions,
        );
    }

    // Final pass: enforce minimum spacing across all ranks to fix any
    // overlaps introduced by overlap correction shifts.
    for r in 0..=max_rank {
        let bucket = &rank_buckets[r];
        for i in 1..bucket.len() {
            let prev = bucket[i - 1];
            let fi = bucket[i];
            let min_x = node_x[prev] + node_w(prev) + spacing(prev, fi);
            if node_x[fi] < min_x {
                node_x[fi] = min_x;
            }
        }
    }

    // Recompute positions after final spacing enforcement
    positions.clear();
    for fi in 0..real_count {
        let ni = flat_nodes[fi].node_index;
        positions.insert(
            graph.nodes[ni].name.clone(),
            (node_x[fi], node_y[fi], node_w(fi), node_h),
        );
    }
    compute_cluster_bounds(
        &graph.top_level,
        &graph.nodes,
        &graph.groups,
        config,
        &mut positions,
    );

    // Final waypoints
    for (edge_key, chain) in &virtual_chains {
        let waypoints: Vec<(f64, f64)> = chain
            .iter()
            .map(|&vi| (node_x[vi] + virtual_w / 2.0, node_y[vi] + node_h / 2.0))
            .collect();
        edge_waypoints.insert(edge_key.clone(), waypoints);
    }

    LayoutResult { positions, edge_waypoints }
}

/// Recursively compute cluster (group) bounding boxes from the positions of their children.
fn compute_cluster_bounds(
    elements: &[LayoutElement],
    nodes: &[LayoutNode],
    groups: &[LayoutGroup],
    config: &LayoutConfig,
    positions: &mut HashMap<String, (f64, f64, f64, f64)>,
) {
    for elem in elements {
        if let LayoutElement::GroupRef(gi) = elem {
            let g = &groups[*gi];
            // Recurse first so nested groups have bounds
            compute_cluster_bounds(&g.children, nodes, groups, config, positions);

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
                let gx = min_x - config.group_h_pad;
                let gy = min_y - config.group_header_h - config.group_v_pad;
                let gw = (max_x - min_x + config.group_h_pad * 2.0)
                    .max(text_width(&g.name) + config.group_h_pad * 2.0);
                let gh = max_y - min_y + config.group_header_h + config.group_v_pad * 2.0;
                positions.insert(g.name.clone(), (gx, gy, gw, gh));
            }
        }
    }
}

/// Find x-shifts needed to eliminate overlaps between sibling elements (groups and standalone nodes).
/// `elem_sort_keys` maps element name -> sort key (e.g. avg x of highest-rank nodes).
/// Returns a map of node_name -> dx shift for all nodes that need to move.
fn find_cluster_shifts(
    elements: &[LayoutElement],
    nodes: &[LayoutNode],
    groups: &[LayoutGroup],
    positions: &HashMap<String, (f64, f64, f64, f64)>,
    elem_sort_keys: &HashMap<String, f64>,
    config: &LayoutConfig,
) -> HashMap<String, f64> {
    let mut shifts: HashMap<String, f64> = HashMap::new();
    let gap = config.group_h_pad;

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

    // For each pair (not just consecutive), ensure no overlap.
    // Check all pairs since non-adjacent elements can overlap when they span different Y ranges.
    for i in 0..sibling_bounds.len() {
        for j in (i + 1)..sibling_bounds.len() {
            let (_, _, ax, ay, aw, ah) = sibling_bounds[i];
            let (ej, _, bx, by, _bw, bh) = sibling_bounds[j].clone();

            // Check if they overlap vertically (y ranges intersect)
            let y_overlap = ay < by + bh && by < ay + ah;
            if !y_overlap {
                continue;
            }

            let needed_x = ax + aw + gap;
            if bx < needed_x {
                let dx = needed_x - bx;
                collect_node_shifts(&elements[ej], nodes, groups, dx, &mut shifts);
                // Update this entry's x for cascading checks
                sibling_bounds[j].2 += dx;
            }
        }
    }

    // Recurse into groups to fix overlaps among their children
    for elem in elements {
        if let LayoutElement::GroupRef(gi) = elem {
            let child_shifts = find_cluster_shifts(&groups[*gi].children, nodes, groups, positions, elem_sort_keys, config);
            for (name, dx) in child_shifts {
                *shifts.entry(name).or_insert(0.0) += dx;
            }
        }
    }

    shifts
}

/// Collect all node names under an element and assign them a dx shift.
fn collect_node_shifts(
    elem: &LayoutElement,
    nodes: &[LayoutNode],
    groups: &[LayoutGroup],
    dx: f64,
    shifts: &mut HashMap<String, f64>,
) {
    match elem {
        LayoutElement::NodeRef(ni) => {
            *shifts.entry(nodes[*ni].name.clone()).or_insert(0.0) += dx;
        }
        LayoutElement::GroupRef(gi) => {
            for child in &groups[*gi].children {
                collect_node_shifts(child, nodes, groups, dx, shifts);
            }
        }
    }
}

/// Get the name of a LayoutElement.
fn element_name(elem: &LayoutElement, nodes: &[LayoutNode], groups: &[LayoutGroup]) -> String {
    match elem {
        LayoutElement::NodeRef(i) => nodes[*i].name.clone(),
        LayoutElement::GroupRef(i) => groups[*i].name.clone(),
    }
}
