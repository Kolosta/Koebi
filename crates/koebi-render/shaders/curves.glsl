// Curves shared by the icon and glyph shaders: quadratic curves monotonic in x and y, the
// parameter where one reaches a coordinate, and the coverage a blurred winding number gives.
// Keep in sync with crates/koebi-svg/src/raster.rs, the reference on the CPU.

// `precise`: no driver fuses these products into FMAs, so drivers run the same arithmetic.
float quad_at(float p0, float p1, float p2, float t) {
    precise float u = 1.0 - t;
    precise float value = u * u * p0 + 2.0 * u * t * p1 + t * t * p2;
    return value;
}

// The parameter where a curve monotonic in one coordinate reaches `y` (monotonic_root in
// raster.rs): the two roots in the form that cancels nothing, the one inside the curve.
float monotonic_root(float y0, float y1, float y2, float y) {
    precise float a = y0 - 2.0 * y1 + y2;
    precise float b = 2.0 * (y1 - y0);
    precise float c = y0 - y;
    precise float t;
    if (abs(a) < 1e-9) {
        t = abs(b) < 1e-12 ? 0.0 : -c / b;
    } else {
        precise float discriminant = sqrt(max(b * b - 4.0 * a * c, 0.0));
        precise float q = -0.5 * (b + (b >= 0.0 ? discriminant : -discriminant));
        precise float first = q / a;
        precise float second = abs(q) > 1e-30 ? c / q : first;
        t = (first >= -1e-4 && first <= 1.0001) ? first : second;
    }
    return clamp(t, 0.0, 1.0);
}

bool spans(float from, float to, float value) {
    return value >= min(from, to) && value < max(from, to);
}

// The coverage a blurred winding number gives: nonzero clamps it, even-odd folds it.
float shade(float winding, bool even_odd) {
    return even_odd ? 1.0 - abs(1.0 - mod(abs(winding), 2.0)) : min(abs(winding), 1.0);
}
