#[derive(Debug, PartialEq, Clone, Copy)]
pub struct Vector3(pub f64, pub f64, pub f64);

impl Vector3 {
    #[inline]
    pub fn zero() -> Self {
        Self(0.0, 0.0, 0.0)
    }

    #[inline]
    pub fn distance_sq_from(&self, pt: &Self) -> f64 {
        let dx = pt.0 - self.0;
        let dy = pt.1 - self.1;
        let dz = pt.2 - self.2;
        dx * dx + dy * dy + dz * dz
    }

    #[inline]
    pub fn distance_from(&self, pt: &Self) -> f64 {
        self.distance_sq_from(pt).sqrt()
    }

    #[inline]
    pub fn magnitude_sq(&self) -> f64 {
        self.0 * self.0 + self.1 * self.1 + self.2 * self.2
    }

    #[inline]
    pub fn magnitude(&self) -> f64 {
        self.magnitude_sq().sqrt()
    }

    #[inline]
    pub fn normalize(&mut self) -> &mut Self {
        let num = self.magnitude();
        if num > 1e-5 {
            *self /= num;
        } else {
            *self = Self::zero();
        }
        self
    }

    #[inline]
    pub fn dot(&self, rhs: &Vector3) -> f64 {
        self.0 * rhs.0 + self.1 * rhs.1 + self.2 * rhs.2
    }

    #[inline]
    pub fn slerp(lhs: &Vector3, rhs: &Vector3, percent: f64) -> Vector3 {
        let dot = lhs.dot(rhs).clamp(-1.0, 1.0);
        let theta = dot.acos() * percent;
        let mut relative_vec = rhs - &(lhs * dot);
        relative_vec.normalize();
        &(lhs * theta.cos()) + &(&relative_vec * theta.sin())
    }
}

impl std::ops::Add<&Vector3> for &Vector3 {
    type Output = Vector3;

    #[inline]
    fn add(self, rhs: &Vector3) -> Vector3 {
        Vector3(self.0 + rhs.0, self.1 + rhs.1, self.2 + rhs.2)
    }
}

impl std::ops::AddAssign<&Vector3> for Vector3 {
    #[inline]
    fn add_assign(&mut self, rhs: &Vector3) {
        self.0 += rhs.0;
        self.1 += rhs.1;
        self.2 += rhs.2;
    }
}

impl std::ops::Sub<&Vector3> for &Vector3 {
    type Output = Vector3;

    #[inline]
    fn sub(self, rhs: &Vector3) -> Vector3 {
        Vector3(self.0 - rhs.0, self.1 - rhs.1, self.2 - rhs.2)
    }
}

impl std::ops::SubAssign<&Vector3> for Vector3 {
    #[inline]
    fn sub_assign(&mut self, rhs: &Vector3) {
        self.0 -= rhs.0;
        self.1 -= rhs.1;
        self.2 -= rhs.2;
    }
}

impl std::ops::Mul<f64> for Vector3 {
    type Output = Vector3;

    #[inline]
    fn mul(self, rhs: f64) -> Vector3 {
        Vector3(self.0 * rhs, self.1 * rhs, self.2 * rhs)
    }
}

impl std::ops::Mul<f64> for &Vector3 {
    type Output = Vector3;

    #[inline]
    fn mul(self, rhs: f64) -> Vector3 {
        Vector3(self.0 * rhs, self.1 * rhs, self.2 * rhs)
    }
}

impl std::ops::MulAssign<f64> for Vector3 {
    #[inline]
    fn mul_assign(&mut self, rhs: f64) {
        self.0 *= rhs;
        self.1 *= rhs;
        self.2 *= rhs;
    }
}

impl std::ops::Div<f64> for &Vector3 {
    type Output = Vector3;

    #[inline]
    fn div(self, rhs: f64) -> Vector3 {
        Vector3(self.0 / rhs, self.1 / rhs, self.2 / rhs)
    }
}

impl std::ops::DivAssign<f64> for Vector3 {
    #[inline]
    fn div_assign(&mut self, rhs: f64) {
        self.0 /= rhs;
        self.1 /= rhs;
        self.2 /= rhs;
    }
}
