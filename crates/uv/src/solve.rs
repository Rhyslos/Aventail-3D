//! A small sparse least-squares solver: the normal equations, solved by
//! conjugate gradients with a diagonal preconditioner. Enough for the
//! sizes of game assets, with no outside library.

/// How small the remaining error must get, against the right-hand side,
/// for a solve to count as settled: far below what shows in a texture.
const SETTLED: f64 = 1e-8;

/// A symmetric sparse matrix, built by adding to its entries.
#[derive(Debug, Clone)]
pub(crate) struct Normal {
    rows: Vec<Vec<(usize, f64)>>,
}

impl Normal {
    pub(crate) fn new(n: usize) -> Self {
        Self {
            rows: vec![Vec::new(); n],
        }
    }

    fn add(&mut self, i: usize, j: usize, value: f64) {
        let row = &mut self.rows[i];
        match row.iter_mut().find(|(k, _)| *k == j) {
            Some((_, v)) => *v += value,
            None => row.push((j, value)),
        }
    }

    /// Adds the row `entries` (unknown, weight) of a least-squares system:
    /// its outer product, so the matrix stays `AᵀA`.
    pub(crate) fn add_row(&mut self, entries: &[(usize, f64)]) {
        for &(i, a) in entries {
            for &(j, b) in entries {
                self.add(i, j, a * b);
            }
        }
    }

    /// The matrix packed row after row, for quick multiplying.
    fn packed(&self) -> Packed {
        let mut starts = Vec::with_capacity(self.rows.len() + 1);
        let mut columns = Vec::new();
        let mut values = Vec::new();
        starts.push(0);
        for row in &self.rows {
            for (j, v) in row {
                columns.push(*j);
                values.push(*v);
            }
            starts.push(columns.len());
        }
        Packed {
            starts,
            columns,
            values,
        }
    }

    fn diagonal(&self) -> Vec<f64> {
        self.rows
            .iter()
            .enumerate()
            .map(|(i, row)| {
                row.iter()
                    .find(|(j, _)| *j == i)
                    .map_or(1.0, |(_, v)| if v.abs() > 1e-300 { *v } else { 1.0 })
            })
            .collect()
    }

    /// Solves `self · x = b`, starting from `x`, for at most `steps`
    /// steps. Returns whether it settled.
    pub(crate) fn solve(&self, b: &[f64], x: &mut [f64], steps: usize) -> bool {
        let n = b.len();
        let d = self.diagonal();
        let m = self.packed();
        let ax = m.times(x);
        let mut r: Vec<f64> = (0..n).map(|i| b[i] - ax[i]).collect();
        let mut z: Vec<f64> = (0..n).map(|i| r[i] / d[i]).collect();
        let mut p = z.clone();
        let mut rz: f64 = (0..n).map(|i| r[i] * z[i]).sum();
        let scale: f64 = b.iter().map(|v| v * v).sum::<f64>().sqrt().max(1e-30);
        for _ in 0..steps {
            let norm: f64 = r.iter().map(|v| v * v).sum::<f64>().sqrt();
            if norm <= SETTLED * scale {
                return true;
            }
            let ap = m.times(&p);
            let pap: f64 = (0..n).map(|i| p[i] * ap[i]).sum();
            if pap.abs() < 1e-300 || !pap.is_finite() {
                return false;
            }
            let alpha = rz / pap;
            for i in 0..n {
                x[i] += alpha * p[i];
                r[i] -= alpha * ap[i];
            }
            for i in 0..n {
                z[i] = r[i] / d[i];
            }
            let next: f64 = (0..n).map(|i| r[i] * z[i]).sum();
            let beta = next / rz;
            rz = next;
            for i in 0..n {
                p[i] = z[i] + beta * p[i];
            }
        }
        let norm: f64 = r.iter().map(|v| v * v).sum::<f64>().sqrt();
        norm <= 1e-5 * scale
    }
}

/// A matrix packed row after row.
struct Packed {
    starts: Vec<usize>,
    columns: Vec<usize>,
    values: Vec<f64>,
}

impl Packed {
    fn times(&self, x: &[f64]) -> Vec<f64> {
        (0..self.starts.len() - 1)
            .map(|i| {
                (self.starts[i]..self.starts[i + 1])
                    .map(|k| self.values[k] * x[self.columns[k]])
                    .sum()
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_small_system_is_solved() {
        // Rows of x0 + x1 = 3, x0 - x1 = 1, 2 x1 = 2 (in least squares: x = 2, 1).
        let mut m = Normal::new(2);
        let mut b = vec![0.0; 2];
        for (row, rhs) in [
            (vec![(0, 1.0), (1, 1.0)], 3.0),
            (vec![(0, 1.0), (1, -1.0)], 1.0),
            (vec![(1, 2.0)], 2.0),
        ] {
            m.add_row(&row);
            for (i, a) in &row {
                b[*i] += a * rhs;
            }
        }
        let mut x = vec![0.0; 2];
        assert!(m.solve(&b, &mut x, 50));
        assert!(
            (x[0] - 2.0).abs() < 1e-8 && (x[1] - 1.0).abs() < 1e-8,
            "{x:?}"
        );
    }

    #[test]
    fn nothing_to_solve_settles_at_once() {
        let m = Normal::new(3);
        let mut x = vec![0.0; 3];
        assert!(m.solve(&[0.0; 3], &mut x, 10));
        assert_eq!(x, vec![0.0; 3]);
    }
}
