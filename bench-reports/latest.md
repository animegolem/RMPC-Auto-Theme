# Benchmark Summary (variant: inhouse, ΔE: DE76, matching: standard)

| Image | Samples | Variant | Interactive ms | JS ms (measured) | Rust ms | Δms | Max ΔE | Mean ΔE | P95 ΔE | >20 cnt | Max ΔRGB |
|---|---:|:---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 1.5mb example | 300000 | inhouse | 0.71 | 5021.58 | 276.78 | -4744.80 | 41.27 | 4.22 | 15.89 | 4 | 114.00 |
| 4mb example | 300000 | inhouse | 0.75 | 4518.73 | 637.25 | -3881.48 | 20.76 | 2.79 | 10.77 | 2 | 41.00 |
| 11mb example | 300000 | inhouse | 0.69 | 6259.73 | 291.91 | -5967.82 | 29.91 | 2.99 | 10.09 | 2 | 75.00 |
| 24mb example | 300000 | inhouse | 0.70 | 5698.74 | 690.79 | -5007.95 | 20.01 | 5.19 | 9.87 | 1 | 48.00 |

Average Δms: -4900.51 — Max Δms: +5967.82 — Max ΔE: 41.27
Mean ΔE (share-weighted) avg: 2.51
Speed gate (Rust ≤ +20% vs JS per image): PASS
