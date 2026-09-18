# Attributions

While no code is shared, this crate is deeply inspired by **Color-tool** by Laurent Jégou. It wouldn't exist without you!

---

This crate uses and builds upon work from the following sources under their respective licenses:

## Color space conversions

The color space conversion algorithms in `src/color.rs` are based on:

**Color-tool** by Laurent Jégou
License: CC BY 3.0 (https://creativecommons.org/licenses/by/3.0/)
Source: https://github.com/ljegou/Color-tool

Additional references:

- CIE 15:2018 (Colorimetry, 4th Edition) for LAB/LUV
- IEC 61966-2-1:1999 for sRGB gamma and XYZ transforms

## K-means clustering

The k-means clustering algorithm uses k-means++ initialization as described in:
Arthur, D., & Vassilvitskii, S. (2007). "k-means++: The advantages of careful seeding."
Proceedings of the eighteenth annual ACM-SIAM symposium on Discrete algorithms.

---

This attribution travels with the crate: include it in any distribution that embeds `color-core`.
