use std::collections::{HashMap, HashSet};

use crate::types::FlatNode;

/// Median heuristic for ordering nodes within each rank.
/// `graph_edges` are (from_flat_idx, to_flat_idx) pairs.
/// Returns ordered node indices per rank.
pub(crate) fn order_ranks(
    flat_nodes: &[FlatNode],
    ranks: &[usize],
    graph_edges: &[(usize, usize)],
) -> Vec<Vec<usize>> {
    let max_rank = ranks.iter().copied().max().unwrap_or(0);

    let mut rank_buckets: Vec<Vec<usize>> = vec![vec![]; max_rank + 1];
    for (fi, _) in flat_nodes.iter().enumerate() {
        rank_buckets[ranks[fi]].push(fi);
    }

    let n = flat_nodes.len();
    let mut predecessors: Vec<Vec<usize>> = vec![vec![]; n];
    let mut successors: Vec<Vec<usize>> = vec![vec![]; n];
    for &(from, to) in graph_edges {
        if from != to && from < n && to < n {
            successors[from].push(to);
            predecessors[to].push(from);
        }
    }

    let mut pos: Vec<usize> = vec![0; n];
    for bucket in &rank_buckets {
        for (p, &fi) in bucket.iter().enumerate() {
            pos[fi] = p;
        }
    }

    let max_iter = 24;
    for iter in 0..max_iter {
        if iter % 2 == 0 {
            // Down sweep
            for r in 1..=max_rank {
                let mut med_vals: Vec<(f64, usize)> = Vec::new();
                let mut med_map: HashMap<usize, f64> = HashMap::new();
                for &fi in &rank_buckets[r] {
                    let preds = &predecessors[fi];
                    if preds.is_empty() {
                        med_vals.push((pos[fi] as f64, fi));
                        med_map.insert(fi, pos[fi] as f64);
                    } else {
                        let mut positions: Vec<f64> =
                            preds.iter().map(|&p| pos[p] as f64).collect();
                        positions.sort_by(|a, b| a.partial_cmp(b).unwrap());
                        let med = median_value(&positions);
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
            // Up sweep
            for r in (0..max_rank).rev() {
                let mut med_vals: Vec<(f64, usize)> = Vec::new();
                let mut med_map: HashMap<usize, f64> = HashMap::new();
                for &fi in &rank_buckets[r] {
                    let succs = &successors[fi];
                    if succs.is_empty() {
                        med_vals.push((pos[fi] as f64, fi));
                        med_map.insert(fi, pos[fi] as f64);
                    } else {
                        let mut positions: Vec<f64> =
                            succs.iter().map(|&s| pos[s] as f64).collect();
                        positions.sort_by(|a, b| a.partial_cmp(b).unwrap());
                        let med = median_value(&positions);
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

        // Transpose step: within-cluster swaps + between-cluster block swaps
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
            let mut blocks: Vec<(Option<usize>, usize, usize)> = Vec::new();
            if !bucket.is_empty() {
                let mut block_start = 0;
                let mut current_cluster = flat_nodes[bucket[0]].cluster_chain.last().copied();
                for i in 1..bucket.len() {
                    let c = flat_nodes[bucket[i]].cluster_chain.last().copied();
                    if c != current_cluster {
                        blocks.push((current_cluster, block_start, i));
                        block_start = i;
                        current_cluster = c;
                    }
                }
                blocks.push((current_cluster, block_start, bucket.len()));
            }

            let mut block_improved = true;
            while block_improved && blocks.len() > 1 {
                block_improved = false;
                for bi in 0..blocks.len() - 1 {
                    let (_, a_start, a_end) = blocks[bi];
                    let (_, b_start, b_end) = blocks[bi + 1];
                    let block_a: Vec<usize> = bucket[a_start..a_end].to_vec();
                    let block_b: Vec<usize> = bucket[b_start..b_end].to_vec();

                    let cross_ab = count_block_crossings(
                        &block_a, &block_b, &successors, &predecessors, &pos,
                    );
                    let b_len = block_b.len();
                    for (j, &fi) in block_b.iter().enumerate() {
                        pos[fi] = a_start + j;
                    }
                    for (j, &fi) in block_a.iter().enumerate() {
                        pos[fi] = a_start + b_len + j;
                    }
                    let cross_ba = count_block_crossings(
                        &block_b, &block_a, &successors, &predecessors, &pos,
                    );

                    if cross_ba < cross_ab {
                        let mut new_section = block_b.clone();
                        new_section.extend(&block_a);
                        bucket[a_start..b_end].copy_from_slice(&new_section);
                        blocks[bi] = (blocks[bi + 1].0, a_start, a_start + b_len);
                        blocks[bi + 1] = (blocks[bi].0, a_start + b_len, b_end);
                        block_improved = true;
                    } else {
                        let a_len = block_a.len();
                        for (j, &fi) in block_a.iter().enumerate() {
                            pos[fi] = a_start + j;
                        }
                        for (j, &fi) in block_b.iter().enumerate() {
                            pos[fi] = a_start + a_len + j;
                        }
                    }
                }
            }
            for (i, &fi) in bucket.iter().enumerate() {
                pos[fi] = i;
            }
        }
    }

    rank_buckets
}

fn median_value(sorted: &[f64]) -> f64 {
    if sorted.len() % 2 == 1 {
        sorted[sorted.len() / 2]
    } else if sorted.len() >= 2 {
        let l = sorted.len() / 2 - 1;
        (sorted[l] + sorted[l + 1]) / 2.0
    } else {
        sorted.first().copied().unwrap_or(0.0)
    }
}

fn same_cluster(a: usize, b: usize, flat_nodes: &[FlatNode]) -> bool {
    flat_nodes[a].cluster_chain.last() == flat_nodes[b].cluster_chain.last()
}

/// Enforce cluster contiguity: group nodes by their innermost cluster,
/// ordering cluster blocks by the median of their members' heuristic values.
fn enforce_cluster_contiguity(
    bucket: &mut Vec<usize>,
    flat_nodes: &[FlatNode],
    medians: &HashMap<usize, f64>,
) {
    if bucket.len() <= 1 {
        return;
    }

    let mut cluster_order: Vec<Option<usize>> = Vec::new();
    let mut cluster_groups: HashMap<Option<usize>, Vec<usize>> = HashMap::new();
    let mut seen_clusters: HashSet<Option<usize>> = HashSet::new();

    for &fi in bucket.iter() {
        let cluster = flat_nodes[fi].cluster_chain.last().copied();
        if seen_clusters.insert(cluster) {
            cluster_order.push(cluster);
        }
        cluster_groups.entry(cluster).or_default().push(fi);
    }

    let mut block_medians: Vec<(f64, Option<usize>)> = Vec::new();
    for &cluster in &cluster_order {
        if let Some(members) = cluster_groups.get(&cluster) {
            let mut vals: Vec<f64> = members
                .iter()
                .map(|&fi| *medians.get(&fi).unwrap_or(&(fi as f64)))
                .collect();
            vals.sort_by(|a, b| a.partial_cmp(b).unwrap());
            let med = median_value(&vals);
            block_medians.push((med, cluster));
        }
    }
    block_medians.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

    bucket.clear();
    for (_, cluster) in &block_medians {
        if let Some(group) = cluster_groups.get(cluster) {
            bucket.extend(group);
        }
    }
}

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
            for &sa in &successors[a] {
                for &sb in &successors[b] {
                    if pos[sa] > pos[sb] {
                        count += 1;
                    }
                }
            }
            for &pa in &predecessors[a] {
                for &pb in &predecessors[b] {
                    if pos[pa] > pos[pb] {
                        count += 1;
                    }
                }
            }
        }
    }
    count
}
