use rand::{rngs::SmallRng, Rng, SeedableRng};
use rayon::prelude::*;
#[cfg(feature = "simd")]
use wide::f32x4;

#[derive(Debug, Clone)]
pub struct KMeansConfig {
    pub k: usize,
    pub max_iters: usize,
    pub tol: f32,
    pub seed: u64,
    pub warm_start: Option<Vec<[f32; 3]>>,
    pub mini_batch: Option<usize>,
}

impl Default for KMeansConfig {
    fn default() -> Self {
        Self {
            k: 8,
            max_iters: 40,
            tol: 1e-3,
            seed: 1,
            warm_start: None,
            mini_batch: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct KMeansResult {
    pub centroids: Vec<[f32; 3]>,
    pub counts: Vec<usize>,
    pub iterations: usize,
    pub inertia: f32,
}

#[derive(Debug, Clone)]
pub struct PointsSoa {
    px: Vec<f32>,
    py: Vec<f32>,
    pz: Vec<f32>,
}

impl PointsSoa {
    pub fn from_points(points: &[[f32; 3]]) -> Self {
        let mut px = Vec::with_capacity(points.len());
        let mut py = Vec::with_capacity(points.len());
        let mut pz = Vec::with_capacity(points.len());
        for p in points {
            px.push(p[0]);
            py.push(p[1]);
            pz.push(p[2]);
        }
        Self { px, py, pz }
    }

    pub fn len(&self) -> usize {
        self.px.len()
    }

    pub fn to_vec(&self) -> Vec<[f32; 3]> {
        self.px
            .iter()
            .zip(self.py.iter())
            .zip(self.pz.iter())
            .map(|((x, y), z)| [*x, *y, *z])
            .collect()
    }

    pub fn component_tuple(&self, idx: usize) -> (f32, f32, f32) {
        (self.px[idx], self.py[idx], self.pz[idx])
    }
}

#[derive(Debug, Clone)]
struct CentroidsSoa {
    cx: Vec<f32>,
    cy: Vec<f32>,
    cz: Vec<f32>,
}

impl CentroidsSoa {
    fn with_len(k: usize) -> Self {
        Self {
            cx: vec![0.0; k],
            cy: vec![0.0; k],
            cz: vec![0.0; k],
        }
    }

    fn len(&self) -> usize {
        self.cx.len()
    }

    fn set_from_soa(&mut self, centroid_idx: usize, points: &PointsSoa, point_idx: usize) {
        self.cx[centroid_idx] = points.px[point_idx];
        self.cy[centroid_idx] = points.py[point_idx];
        self.cz[centroid_idx] = points.pz[point_idx];
    }

    fn from_vec(data: &[[f32; 3]]) -> Self {
        let mut cx = Vec::with_capacity(data.len());
        let mut cy = Vec::with_capacity(data.len());
        let mut cz = Vec::with_capacity(data.len());
        for c in data {
            cx.push(c[0]);
            cy.push(c[1]);
            cz.push(c[2]);
        }
        Self { cx, cy, cz }
    }

    fn component_tuple(&self, idx: usize) -> (f32, f32, f32) {
        (self.cx[idx], self.cy[idx], self.cz[idx])
    }

    fn to_vec(&self) -> Vec<[f32; 3]> {
        self.cx
            .iter()
            .zip(self.cy.iter())
            .zip(self.cz.iter())
            .map(|((x, y), z)| [*x, *y, *z])
            .collect()
    }
}

pub fn run_kmeans(points: &[[f32; 3]], cfg: &KMeansConfig) -> KMeansResult {
    let dataset = PointsSoa::from_points(points);
    run_kmeans_soa(&dataset, cfg)
}

pub fn run_kmeans_soa(dataset: &PointsSoa, cfg: &KMeansConfig) -> KMeansResult {
    assert!(cfg.k > 0, "k must be > 0");
    assert!(dataset.len() >= cfg.k, "points must be >= k");

    let mut rng = SmallRng::seed_from_u64(cfg.seed);
    let mut centroids = if let Some(warm) = &cfg.warm_start {
        assert_eq!(warm.len(), cfg.k, "warm_start length must equal k");
        CentroidsSoa::from_vec(warm)
    } else {
        kmeans_plus_plus(dataset, cfg.k, &mut rng)
    };

    let mut counts = vec![0usize; cfg.k];
    let mut iterations = 0;
    let mut inertia = 0.0;

    while iterations < cfg.max_iters {
        let mini_batch_storage = if let Some(batch_size) = cfg.mini_batch {
            if batch_size > 0 && batch_size < dataset.len() {
                Some(sample_batch(&dataset, batch_size, &mut rng))
            } else {
                None
            }
        } else {
            None
        };
        let working = mini_batch_storage.as_ref().unwrap_or(&dataset);

        let (partials, step_inertia) = assignment_step(working, &centroids);
        inertia = step_inertia;

        counts.fill(0);
        let mut shift = 0.0;
        for (idx, part) in partials.into_iter().enumerate() {
            if part.count == 0 {
                let rand_idx = rng.gen_range(0..dataset.len());
                centroids.set_from_soa(idx, &dataset, rand_idx);
                continue;
            }
            let inv = 1.0 / part.count as f32;
            let nx = part.sum_x * inv;
            let ny = part.sum_y * inv;
            let nz = part.sum_z * inv;
            let (ox, oy, oz) = centroids.component_tuple(idx);
            shift += squared_distance_components(ox, oy, oz, nx, ny, nz);
            centroids.cx[idx] = nx;
            centroids.cy[idx] = ny;
            centroids.cz[idx] = nz;
            counts[idx] = part.count;
        }

        iterations += 1;
        if shift.sqrt() < cfg.tol {
            break;
        }
    }

    KMeansResult {
        centroids: centroids.to_vec(),
        counts,
        iterations,
        inertia,
    }
}

#[derive(Clone, Debug, Default)]
struct ClusterPartial {
    sum_x: f32,
    sum_y: f32,
    sum_z: f32,
    count: usize,
}

fn assignment_step(points: &PointsSoa, centroids: &CentroidsSoa) -> (Vec<ClusterPartial>, f32) {
    let k = centroids.len();
    let chunk_size = 1024usize.max(k);
    let total_len = points.len();
    let chunk_count = (total_len + chunk_size - 1) / chunk_size;

    let chunk_partials: Vec<(Vec<ClusterPartial>, f32)> = (0..chunk_count)
        .into_par_iter()
        .map(|chunk_idx| {
            let start = chunk_idx * chunk_size;
            let end = ((chunk_idx + 1) * chunk_size).min(total_len);
            let mut partials = vec![ClusterPartial::default(); k];
            let mut inertia = 0.0f32;
            for idx in start..end {
                let (px, py, pz) = points.component_tuple(idx);
                let (best_idx, best_dist) = best_centroid(px, py, pz, centroids);
                let entry = &mut partials[best_idx];
                entry.sum_x += px;
                entry.sum_y += py;
                entry.sum_z += pz;
                entry.count += 1;
                inertia += best_dist;
            }
            (partials, inertia)
        })
        .collect();

    // Deterministic, numerically steadier merge: accumulate in f64, fixed order
    let mut totals = vec![ClusterPartial::default(); k];
    let mut acc_x: Vec<f64> = vec![0.0; k];
    let mut acc_y: Vec<f64> = vec![0.0; k];
    let mut acc_z: Vec<f64> = vec![0.0; k];
    let mut acc_n: Vec<usize> = vec![0; k];
    let mut total_inertia = 0.0f32;
    for (chunk_partials, chunk_inertia) in chunk_partials {
        for idx in 0..k {
            acc_x[idx] += chunk_partials[idx].sum_x as f64;
            acc_y[idx] += chunk_partials[idx].sum_y as f64;
            acc_z[idx] += chunk_partials[idx].sum_z as f64;
            acc_n[idx] += chunk_partials[idx].count;
        }
        total_inertia += chunk_inertia;
    }
    for idx in 0..k {
        totals[idx].sum_x = acc_x[idx] as f32;
        totals[idx].sum_y = acc_y[idx] as f32;
        totals[idx].sum_z = acc_z[idx] as f32;
        totals[idx].count = acc_n[idx];
    }

    (totals, total_inertia)
}

#[inline]
fn best_centroid(px: f32, py: f32, pz: f32, centroids: &CentroidsSoa) -> (usize, f32) {
    #[cfg(feature = "simd")]
    {
        return best_centroid_simd(px, py, pz, centroids);
    }
    #[cfg(not(feature = "simd"))]
    {
        return best_centroid_scalar(px, py, pz, centroids);
    }
}

#[cfg(feature = "simd")]
fn best_centroid_simd(px: f32, py: f32, pz: f32, centroids: &CentroidsSoa) -> (usize, f32) {
    const LANES: usize = 4;
    let mut best_idx = 0;
    let mut best_dist = f32::MAX;
    let len = centroids.len();
    let px_v = f32x4::splat(px);
    let py_v = f32x4::splat(py);
    let pz_v = f32x4::splat(pz);

    let mut idx = 0;
    while idx + LANES <= len {
        let cx = f32x4::from([
            centroids.cx[idx],
            centroids.cx[idx + 1],
            centroids.cx[idx + 2],
            centroids.cx[idx + 3],
        ]);
        let cy = f32x4::from([
            centroids.cy[idx],
            centroids.cy[idx + 1],
            centroids.cy[idx + 2],
            centroids.cy[idx + 3],
        ]);
        let cz = f32x4::from([
            centroids.cz[idx],
            centroids.cz[idx + 1],
            centroids.cz[idx + 2],
            centroids.cz[idx + 3],
        ]);

        let dx = px_v - cx;
        let dy = py_v - cy;
        let dz = pz_v - cz;
        let dist = dx * dx + dy * dy + dz * dz;
        let dist_arr: [f32; LANES] = dist.into();
        for lane in 0..LANES {
            let d = dist_arr[lane];
            if d < best_dist {
                best_dist = d;
                best_idx = idx + lane;
            }
        }
        idx += LANES;
    }

    while idx < len {
        let d = squared_distance_components(
            px,
            py,
            pz,
            centroids.cx[idx],
            centroids.cy[idx],
            centroids.cz[idx],
        );
        if d < best_dist {
            best_dist = d;
            best_idx = idx;
        }
        idx += 1;
    }
    (best_idx, best_dist)
}

#[cfg(not(feature = "simd"))]
fn best_centroid_scalar(px: f32, py: f32, pz: f32, centroids: &CentroidsSoa) -> (usize, f32) {
    let mut best_idx = 0usize;
    let mut best_dist = f32::MAX;
    for i in 0..centroids.len() {
        let d = squared_distance_components(
            px,
            py,
            pz,
            centroids.cx[i],
            centroids.cy[i],
            centroids.cz[i],
        );
        if d < best_dist {
            best_dist = d;
            best_idx = i;
        }
    }
    (best_idx, best_dist)
}

fn kmeans_plus_plus(points: &PointsSoa, k: usize, rng: &mut SmallRng) -> CentroidsSoa {
    let n = points.len();
    let mut centroids = CentroidsSoa::with_len(k);
    let mut chosen_flags = vec![false; n];
    let first_idx = rng.gen_range(0..n);
    centroids.set_from_soa(0, points, first_idx);
    chosen_flags[first_idx] = true;

    let mut distances = vec![0.0f32; n];
    for i in 0..n {
        distances[i] = squared_distance_components(
            points.px[i],
            points.py[i],
            points.pz[i],
            centroids.cx[0],
            centroids.cy[0],
            centroids.cz[0],
        );
    }

    for centroid_idx in 1..k {
        let mut sum = 0.0;
        for (i, dist) in distances.iter().enumerate() {
            if !chosen_flags[i] {
                sum += *dist;
            }
        }

        let chosen_idx = if sum == 0.0 {
            // Fallback: pick a random unchosen point
            let mut idx;
            loop {
                idx = rng.gen_range(0..n);
                if !chosen_flags[idx] {
                    break;
                }
            }
            idx
        } else {
            let mut target = rng.gen::<f32>() * sum;
            let mut idx = 0;
            for (i, dist) in distances.iter().enumerate() {
                if chosen_flags[i] {
                    continue;
                }
                target -= *dist;
                if target <= 0.0 {
                    idx = i;
                    break;
                }
            }
            idx
        };

        centroids.set_from_soa(centroid_idx, points, chosen_idx);
        chosen_flags[chosen_idx] = true;

        for i in 0..n {
            if chosen_flags[i] {
                distances[i] = 0.0;
                continue;
            }
            let dist = squared_distance_components(
                points.px[i],
                points.py[i],
                points.pz[i],
                centroids.cx[centroid_idx],
                centroids.cy[centroid_idx],
                centroids.cz[centroid_idx],
            );
            if dist < distances[i] {
                distances[i] = dist;
            }
        }
    }

    centroids
}

fn sample_batch(points: &PointsSoa, size: usize, rng: &mut SmallRng) -> PointsSoa {
    if size == 0 {
        return PointsSoa {
            px: Vec::new(),
            py: Vec::new(),
            pz: Vec::new(),
        };
    }
    if size >= points.len() {
        return points.clone();
    }
    let mut px = Vec::with_capacity(size);
    let mut py = Vec::with_capacity(size);
    let mut pz = Vec::with_capacity(size);
    for _ in 0..size {
        let idx = rng.gen_range(0..points.len());
        px.push(points.px[idx]);
        py.push(points.py[idx]);
        pz.push(points.pz[idx]);
    }
    PointsSoa { px, py, pz }
}

#[inline]
fn squared_distance_components(px: f32, py: f32, pz: f32, cx: f32, cy: f32, cz: f32) -> f32 {
    let dx = px - cx;
    let dy = py - cy;
    let dz = pz - cz;
    dx * dx + dy * dy + dz * dz
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_cluster_data_parallel() {
        let mut points = Vec::new();
        for _ in 0..200 {
            points.push([0.05, 0.02, 0.01]);
            points.push([0.9, 0.8, 0.75]);
        }
        let cfg = KMeansConfig {
            k: 2,
            max_iters: 20,
            tol: 1e-4,
            seed: 42,
            warm_start: None,
            mini_batch: None,
        };
        let result = run_kmeans(&points, &cfg);
        assert_eq!(result.centroids.len(), 2);
        assert!(result.counts.iter().all(|&c| c > 0));
        assert!(result.iterations <= 20);
        assert!(result.inertia < 5.0);
    }

    #[test]
    fn warm_start_respected() {
        let points = vec![[0.0, 0.0, 0.0]; 10];
        let cfg = KMeansConfig {
            k: 1,
            max_iters: 5,
            tol: 1e-6,
            seed: 2,
            warm_start: Some(vec![[0.5, 0.5, 0.5]]),
            mini_batch: None,
        };
        let result = run_kmeans(&points, &cfg);
        assert_eq!(result.centroids[0], [0.0, 0.0, 0.0]);
    }

    #[test]
    fn mini_batch_executes() {
        let mut points = Vec::new();
        for _ in 0..1000 {
            points.push([0.1, 0.2, 0.3]);
            points.push([0.9, 0.8, 0.7]);
        }
        let cfg = KMeansConfig {
            k: 2,
            max_iters: 5,
            tol: 1e-3,
            seed: 7,
            warm_start: None,
            mini_batch: Some(256),
        };
        let result = run_kmeans(&points, &cfg);
        assert_eq!(result.centroids.len(), 2);
    }

    #[test]
    fn determinism_across_runs() {
        // Build a deterministic dataset
        let mut points = Vec::new();
        for i in 0..500 {
            let t = (i as f32) / 500.0;
            points.push([0.1 + 0.2 * t, 0.3 + 0.1 * t, 0.5 + 0.05 * t]);
            points.push([0.8 - 0.1 * t, 0.7 - 0.2 * t, 0.6 - 0.1 * t]);
        }
        let cfg = KMeansConfig {
            k: 6,
            max_iters: 30,
            tol: 1e-6,
            seed: 12345,
            warm_start: None,
            mini_batch: None,
        };
        let r1 = run_kmeans(&points, &cfg);
        let r2 = run_kmeans(&points, &cfg);
        assert_eq!(r1.counts, r2.counts);
        assert_eq!(r1.iterations, r2.iterations);
        for (a, b) in r1.centroids.iter().zip(r2.centroids.iter()) {
            for j in 0..3 {
                let diff = (a[j] - b[j]).abs();
                assert!(diff <= 1e-6, "centroid component diff {} > 1e-6", diff);
            }
        }
    }
}
