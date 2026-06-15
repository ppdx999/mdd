/// Check if a line segment from p1 to p2 intersects a rectangle
/// centered at (cx, cy) with half-dimensions (hw, hh).
pub fn segment_intersects_rect(
    p1: (f64, f64),
    p2: (f64, f64),
    cx: f64,
    cy: f64,
    hw: f64,
    hh: f64,
) -> bool {
    let margin = 4.0;
    let hw = hw + margin;
    let hh = hh + margin;

    let (x1, y1) = p1;
    let (x2, y2) = p2;

    let min_x = x1.min(x2);
    let max_x = x1.max(x2);
    let min_y = y1.min(y2);
    let max_y = y1.max(y2);
    if max_x < cx - hw || min_x > cx + hw || max_y < cy - hh || min_y > cy + hh {
        return false;
    }

    let dx = x2 - x1;
    let dy = y2 - y1;
    let edges = [
        (cx - hw, cy - hh, cx + hw, cy - hh),
        (cx + hw, cy - hh, cx + hw, cy + hh),
        (cx - hw, cy + hh, cx + hw, cy + hh),
        (cx - hw, cy - hh, cx - hw, cy + hh),
    ];
    for (ex1, ey1, ex2, ey2) in &edges {
        let edx = ex2 - ex1;
        let edy = ey2 - ey1;
        let denom = dx * edy - dy * edx;
        if denom.abs() < 1e-12 {
            continue;
        }
        let t = ((ex1 - x1) * edy - (ey1 - y1) * edx) / denom;
        let u = ((ex1 - x1) * dy - (ey1 - y1) * dx) / denom;
        if t >= 0.01 && t <= 0.99 && u >= 0.0 && u <= 1.0 {
            return true;
        }
    }
    false
}

/// Route an edge from (sx, sy) to (ex, ey) around blocking nodes.
/// Returns a list of waypoints including start and end.
pub fn route_around_nodes(
    sx: f64,
    sy: f64,
    ex: f64,
    ey: f64,
    from_name: &str,
    to_name: &str,
    all_bounds: &[(String, f64, f64, f64, f64)], // (name, cx, cy, hw, hh)
    offset: f64,
) -> Vec<(f64, f64)> {
    let dx = ex - sx;
    let dy = ey - sy;
    let len = (dx * dx + dy * dy).sqrt().max(1e-10);

    let (mut sx, mut sy, mut ex, mut ey) = (sx, sy, ex, ey);
    if offset.abs() > 0.1 {
        let nx = -dy / len * offset;
        let ny = dx / len * offset;
        sx += nx;
        sy += ny;
        ex += nx;
        ey += ny;
    }

    let mut blockers: Vec<usize> = Vec::new();
    for (i, (name, bcx, bcy, bhw, bhh)) in all_bounds.iter().enumerate() {
        if name == from_name || name == to_name {
            continue;
        }
        if segment_intersects_rect((sx, sy), (ex, ey), *bcx, *bcy, *bhw, *bhh) {
            blockers.push(i);
        }
    }

    if blockers.is_empty() {
        return vec![(sx, sy), (ex, ey)];
    }

    let mut waypoints = vec![(sx, sy)];
    blockers.sort_by(|&a, &b| {
        let (_, ax, ay, _, _) = all_bounds[a];
        let (_, bx, by, _, _) = all_bounds[b];
        let da = (ax - sx).powi(2) + (ay - sy).powi(2);
        let db = (bx - sx).powi(2) + (by - sy).powi(2);
        da.partial_cmp(&db).unwrap()
    });

    for &bi in &blockers {
        let (_, bcx, bcy, bhw, bhh) = all_bounds[bi];
        let cross = (bcx - sx) * (ey - sy) - (bcy - sy) * (ex - sx);
        let size_ratio = bhw.max(bhh) / len;
        if size_ratio > 0.01 {
            let margin = 12.0;
            if (ey - sy).abs() > (ex - sx).abs() {
                let side = if cross > 0.0 { -1.0 } else { 1.0 };
                waypoints.push((bcx + side * (bhw + margin), bcy));
            } else {
                let side = if cross > 0.0 { 1.0 } else { -1.0 };
                waypoints.push((bcx, bcy + side * (bhh + margin)));
            }
        }
    }
    waypoints.push((ex, ey));
    waypoints
}

/// Build a smooth SVG path string through waypoints using quadratic Bézier curves.
pub fn build_smooth_path(points: &[(f64, f64)]) -> String {
    if points.len() < 2 {
        return String::new();
    }
    if points.len() == 2 {
        return format!(
            "M{},{} L{},{}",
            points[0].0, points[0].1, points[1].0, points[1].1
        );
    }
    let mut d = format!("M{},{}", points[0].0, points[0].1);
    for i in 1..points.len() - 1 {
        let ctrl = points[i];
        let end = if i + 1 < points.len() - 1 {
            (
                (points[i].0 + points[i + 1].0) / 2.0,
                (points[i].1 + points[i + 1].1) / 2.0,
            )
        } else {
            points[i + 1]
        };
        d.push_str(&format!(
            " Q{},{} {},{}",
            ctrl.0, ctrl.1, end.0, end.1
        ));
    }
    d
}

/// Sample points along a smooth path for intersection checks.
pub fn sample_smooth_path(points: &[(f64, f64)], samples_per_segment: usize) -> Vec<(f64, f64)> {
    if points.len() < 2 {
        return points.to_vec();
    }
    if points.len() == 2 {
        return points.to_vec();
    }
    let mut result = Vec::new();
    let last = *points.last().unwrap();
    for i in 0..points.len() - 2 {
        let start = if i == 0 {
            points[0]
        } else {
            (
                (points[i].0 + points[i - 1].0) / 2.0,
                (points[i].1 + points[i - 1].1) / 2.0,
            )
        };
        let ctrl = points[i + 1];
        let end = if i + 2 < points.len() - 1 {
            (
                (points[i + 1].0 + points[i + 2].0) / 2.0,
                (points[i + 1].1 + points[i + 2].1) / 2.0,
            )
        } else {
            points[i + 2]
        };
        let per_seg = samples_per_segment;
        for j in 0..per_seg {
            let t = j as f64 / per_seg as f64;
            let mt = 1.0 - t;
            result.push((
                mt * mt * start.0 + 2.0 * mt * t * ctrl.0 + t * t * end.0,
                mt * mt * start.1 + 2.0 * mt * t * ctrl.1 + t * t * end.1,
            ));
        }
    }
    result.push(last);
    result
}

/// Find the midpoint along a smooth path by arc length.
pub fn midpoint_on_path(points: &[(f64, f64)]) -> (f64, f64) {
    if points.len() <= 1 {
        return points.first().copied().unwrap_or((0.0, 0.0));
    }
    if points.len() == 2 {
        return (
            (points[0].0 + points[1].0) / 2.0,
            (points[0].1 + points[1].1) / 2.0,
        );
    }
    let samples = sample_smooth_path(points, 64);
    let mut lengths = vec![0.0_f64];
    for i in 1..samples.len() {
        let dx = samples[i].0 - samples[i - 1].0;
        let dy = samples[i].1 - samples[i - 1].1;
        lengths.push(lengths[i - 1] + (dx * dx + dy * dy).sqrt());
    }
    let half = *lengths.last().unwrap() / 2.0;
    for i in 1..lengths.len() {
        if lengths[i] >= half {
            let t = (half - lengths[i - 1]) / (lengths[i] - lengths[i - 1]).max(1e-10);
            return (
                samples[i - 1].0 + (samples[i].0 - samples[i - 1].0) * t,
                samples[i - 1].1 + (samples[i].1 - samples[i - 1].1) * t,
            );
        }
    }
    *samples.last().unwrap()
}

/// Clip a line from (cx, cy) toward (tx, ty) to the edge of a rectangle
/// centered at (cx, cy) with half-dimensions (hw, hh).
pub fn clip_to_rect(cx: f64, cy: f64, tx: f64, ty: f64, hw: f64, hh: f64) -> (f64, f64) {
    let dx = tx - cx;
    let dy = ty - cy;
    if dx.abs() < 1e-9 && dy.abs() < 1e-9 {
        return (cx, cy);
    }
    let mut t = f64::MAX;
    if dx.abs() > 1e-9 {
        t = t.min(hw / dx.abs());
    }
    if dy.abs() > 1e-9 {
        t = t.min(hh / dy.abs());
    }
    (cx + dx * t, cy + dy * t)
}
