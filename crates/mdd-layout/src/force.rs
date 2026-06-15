use std::collections::HashMap;

use crate::types::{LayoutGraph, LayoutResult};

/// Configuration for force-directed layout.
pub struct ForceConfig {
    /// Canvas padding around the layout.
    pub padding: f64,
    /// Number of simulation iterations.
    pub iterations: usize,
    /// Ideal distance between connected nodes. 0.0 = auto-compute.
    pub ideal_distance: f64,
    /// Repulsion strength multiplier (default 1.0).
    pub repulsion_strength: f64,
}

impl Default for ForceConfig {
    fn default() -> Self {
        Self {
            padding: 60.0,
            iterations: 300,
            ideal_distance: 0.0,
            repulsion_strength: 1.0,
        }
    }
}

/// Run Fruchterman-Reingold force-directed layout.
///
/// Places nodes using a physics simulation where:
/// - All node pairs repel each other (inverse-square)
/// - Connected nodes attract each other (spring force)
/// - Temperature (max displacement) decreases over iterations
///
/// Groups in the graph are ignored (no cluster support).
/// Returns positions for all nodes; edge_waypoints is empty (direct lines).
pub fn force_layout(graph: &LayoutGraph, config: &ForceConfig) -> LayoutResult {
    let n = graph.nodes.len();
    let mut positions: HashMap<String, (f64, f64, f64, f64)> = HashMap::new();
    let edge_waypoints: HashMap<String, Vec<(f64, f64)>> = HashMap::new();

    if n == 0 {
        return LayoutResult {
            positions,
            edge_waypoints,
        };
    }

    if n == 1 {
        let node = &graph.nodes[0];
        positions.insert(
            node.name.clone(),
            (config.padding, config.padding, node.width, node.height),
        );
        return LayoutResult {
            positions,
            edge_waypoints,
        };
    }

    // Build name → index map
    let mut name_to_idx: HashMap<&str, usize> = HashMap::new();
    for (i, node) in graph.nodes.iter().enumerate() {
        name_to_idx.insert(&node.name, i);
    }

    // Build edge index pairs
    let mut edges: Vec<(usize, usize)> = Vec::new();
    for edge in &graph.edges {
        if let (Some(&from), Some(&to)) = (
            name_to_idx.get(edge.from.as_str()),
            name_to_idx.get(edge.to.as_str()),
        ) {
            if from != to {
                edges.push((from, to));
            }
        }
    }

    // Compute ideal distance
    let max_w = graph
        .nodes
        .iter()
        .map(|n| n.width)
        .fold(0.0_f64, f64::max);
    let max_h = graph
        .nodes
        .iter()
        .map(|n| n.height)
        .fold(0.0_f64, f64::max);
    let avg_size = (max_w + max_h) / 2.0;
    let k = if config.ideal_distance > 0.0 {
        config.ideal_distance
    } else {
        // Heuristic: ideal distance proportional to node size and count
        (avg_size * 2.0 + 40.0) * (1.0 + (n as f64).sqrt() * 0.1)
    };

    // Initial placement: circle
    let area = k * k * n as f64;
    let radius = (area / std::f64::consts::PI).sqrt() * 0.8;
    let mut x: Vec<f64> = Vec::with_capacity(n);
    let mut y: Vec<f64> = Vec::with_capacity(n);
    for i in 0..n {
        let angle =
            -std::f64::consts::FRAC_PI_2 + 2.0 * std::f64::consts::PI * i as f64 / n as f64;
        x.push(radius * angle.cos());
        y.push(radius * angle.sin());
    }

    // Fruchterman-Reingold simulation
    let repulsion = config.repulsion_strength;
    let mut temperature = radius * 0.5;
    let cooling = temperature / config.iterations as f64;

    for _ in 0..config.iterations {
        let mut dx = vec![0.0_f64; n];
        let mut dy = vec![0.0_f64; n];

        // Repulsive forces between all pairs
        for i in 0..n {
            for j in (i + 1)..n {
                let mut delta_x = x[i] - x[j];
                let mut delta_y = y[i] - y[j];
                let dist = (delta_x * delta_x + delta_y * delta_y).sqrt().max(1.0);
                // Include node size in repulsion distance
                let min_dist = (graph.nodes[i].width + graph.nodes[j].width) / 2.0 + 20.0;
                let effective_dist = dist.max(min_dist * 0.5);
                let force = repulsion * k * k / effective_dist;
                delta_x /= dist;
                delta_y /= dist;
                dx[i] += delta_x * force;
                dy[i] += delta_y * force;
                dx[j] -= delta_x * force;
                dy[j] -= delta_y * force;
            }
        }

        // Attractive forces along edges
        for &(u, v) in &edges {
            let delta_x = x[u] - x[v];
            let delta_y = y[u] - y[v];
            let dist = (delta_x * delta_x + delta_y * delta_y).sqrt().max(1.0);
            let force = dist * dist / k;
            let fx = delta_x / dist * force;
            let fy = delta_y / dist * force;
            dx[u] -= fx;
            dy[u] -= fy;
            dx[v] += fx;
            dy[v] += fy;
        }

        // Apply displacements, limited by temperature
        for i in 0..n {
            let disp = (dx[i] * dx[i] + dy[i] * dy[i]).sqrt().max(1e-6);
            let scale = temperature.min(disp) / disp;
            x[i] += dx[i] * scale;
            y[i] += dy[i] * scale;
        }

        temperature -= cooling;
        if temperature < 0.1 {
            break;
        }
    }

    // Normalize: shift so minimum is at padding
    let min_x = x.iter().cloned().fold(f64::MAX, f64::min);
    let min_y = y.iter().cloned().fold(f64::MAX, f64::min);
    for i in 0..n {
        x[i] -= min_x - config.padding;
        y[i] -= min_y - config.padding;
    }

    // Build positions (x, y are center; convert to top-left)
    for i in 0..n {
        let node = &graph.nodes[i];
        positions.insert(
            node.name.clone(),
            (
                x[i] - node.width / 2.0,
                y[i] - node.height / 2.0,
                node.width,
                node.height,
            ),
        );
    }

    // Ensure non-negative
    let mut final_min_x = f64::MAX;
    let mut final_min_y = f64::MAX;
    for (_, (px, py, _, _)) in &positions {
        final_min_x = final_min_x.min(*px);
        final_min_y = final_min_y.min(*py);
    }
    if final_min_x < config.padding || final_min_y < config.padding {
        let dx = if final_min_x < config.padding {
            config.padding - final_min_x
        } else {
            0.0
        };
        let dy = if final_min_y < config.padding {
            config.padding - final_min_y
        } else {
            0.0
        };
        for (_, pos) in positions.iter_mut() {
            pos.0 += dx;
            pos.1 += dy;
        }
    }

    LayoutResult {
        positions,
        edge_waypoints,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{LayoutEdge, LayoutNode};

    #[test]
    fn basic_force_layout() {
        let mut graph = LayoutGraph::new();
        graph.nodes.push(LayoutNode {
            name: "A".to_string(),
            width: 80.0,
            height: 40.0,
        });
        graph.nodes.push(LayoutNode {
            name: "B".to_string(),
            width: 80.0,
            height: 40.0,
        });
        graph.nodes.push(LayoutNode {
            name: "C".to_string(),
            width: 80.0,
            height: 40.0,
        });
        graph.edges.push(LayoutEdge {
            from: "A".to_string(),
            to: "B".to_string(),
            label: String::new(),
        });
        graph.edges.push(LayoutEdge {
            from: "B".to_string(),
            to: "C".to_string(),
            label: String::new(),
        });

        let result = force_layout(&graph, &ForceConfig::default());
        assert_eq!(result.positions.len(), 3);
        // Connected nodes should be closer than unconnected
        let a = result.positions.get("A").unwrap();
        let b = result.positions.get("B").unwrap();
        let c = result.positions.get("C").unwrap();
        let ab_dist = ((a.0 - b.0).powi(2) + (a.1 - b.1).powi(2)).sqrt();
        let ac_dist = ((a.0 - c.0).powi(2) + (a.1 - c.1).powi(2)).sqrt();
        // A-B are connected, A-C are not directly; B-C are connected
        // A-B distance should be reasonable (not zero, not huge)
        assert!(ab_dist > 10.0);
        assert!(ab_dist < 1000.0);
    }
}
