# Attributions

This project reuses and extends openly licensed work. Please retain these notices in derivative builds and in any distributed documentation.

## Color Space Algorithms

Color conversions (sRGB gamma, XYZ matrices, CIE Lab/Luv, YUV, HSV/HSL) are derived from:

- **Color-tool** by Laurent Jégou  
  License: CC BY 3.0  
  Source: https://github.com/ljegou/Color-tool  
  Institution: Université Toulouse-2 Jean Jaurès, Dépt. de Géographie, UMR LISST

Supporting references:
- IEC 61966-2-1:1999 — sRGB transfer curve and RGB↔XYZ matrices
- CIE 15:2018 — Colorimetry (Lab/Luv definitions)

## Clustering Algorithm

K-means++ seeding follows:
- D. Arthur & S. Vassilvitskii, *k-means++: The Advantages of Careful Seeding*, SODA 2007.

Baseline benchmarking referenced but does not bundle code from:
- **kmeans-engine** by Stanley Fok  
  License: MIT  
  Source: https://github.com/stanleyfok/kmeans-engine

## UI / Documentation Assets

Any screenshots or icons sourced from design tooling remain under their original licenses (see RAG notes for per-asset licensing). Apple VisionOS components were consulted only for inspiration; this project ships custom assets.

---

Ensure these attributions accompany the binary (About dialog, README excerpt, or distribution notes) whenever the generator is shared.
