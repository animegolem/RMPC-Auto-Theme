#[derive(Debug, Clone, Copy)]
pub struct RawCluster {
    pub centroid: [f32; 3],
    pub count: usize,
}

#[derive(Debug, Clone, Copy)]
struct MergeCluster {
    centroid: [f32; 3],
    count: usize,
    radius: f32,
}

pub fn merge_clusters_by_threshold(clusters: Vec<RawCluster>, threshold: f32) -> Vec<RawCluster> {
    let count = clusters.len();
    if count < 2 || threshold <= 0.0 {
        return clusters;
    }

    let mut working: Vec<MergeCluster> = clusters
        .into_iter()
        .map(|cluster| MergeCluster {
            centroid: cluster.centroid,
            count: cluster.count,
            radius: 0.0,
        })
        .collect();

    loop {
        let len = working.len();
        if len < 2 {
            break;
        }

        let mut pairs: Vec<(f32, usize, usize)> = Vec::new();
        for i in 0..len {
            let ci = working[i].centroid;
            for j in (i + 1)..len {
                let cj = working[j].centroid;
                let dist = centroid_distance(ci, cj);
                let guarded = dist + working[i].radius + working[j].radius;
                if guarded <= threshold {
                    pairs.push((dist, i, j));
                }
            }
        }

        if pairs.is_empty() {
            break;
        }

        pairs.sort_by(|a, b| {
            a.0.partial_cmp(&b.0)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.1.cmp(&b.1))
                .then_with(|| a.2.cmp(&b.2))
        });

        let mut merged_flags = vec![false; len];
        let mut next: Vec<MergeCluster> = Vec::with_capacity(len);

        for (_dist, i, j) in pairs {
            if merged_flags[i] || merged_flags[j] {
                continue;
            }
            let a = working[i];
            let b = working[j];
            let total = (a.count + b.count) as f32;
            let inv = 1.0 / total;
            let centroid = [
                (a.centroid[0] * a.count as f32 + b.centroid[0] * b.count as f32) * inv,
                (a.centroid[1] * a.count as f32 + b.centroid[1] * b.count as f32) * inv,
                (a.centroid[2] * a.count as f32 + b.centroid[2] * b.count as f32) * inv,
            ];
            let radius = {
                let dist_a = centroid_distance(a.centroid, centroid);
                let dist_b = centroid_distance(b.centroid, centroid);
                (a.radius + dist_a).max(b.radius + dist_b)
            };
            next.push(MergeCluster {
                centroid,
                count: a.count + b.count,
                radius,
            });
            merged_flags[i] = true;
            merged_flags[j] = true;
        }

        for (idx, cluster) in working.iter().enumerate() {
            if !merged_flags[idx] {
                next.push(*cluster);
            }
        }

        if next.len() == len {
            break;
        }
        working = next;
    }

    let mut merged: Vec<RawCluster> = working
        .into_iter()
        .map(|cluster| RawCluster {
            centroid: cluster.centroid,
            count: cluster.count,
        })
        .collect();
    merged.sort_by_key(|c| std::cmp::Reverse(c.count));
    merged
}

pub fn centroid_distance(a: [f32; 3], b: [f32; 3]) -> f32 {
    let dx = a[0] - b[0];
    let dy = a[1] - b[1];
    let dz = a[2] - b[2];
    (dx * dx + dy * dy + dz * dz).sqrt()
}
