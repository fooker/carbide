use std::ops;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Position {
    // TODO: Use decimals here?

    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Position {
    pub fn zero() -> Self {
        return Self { x: 0.0, y: 0.0, z: 0.0 };
    }
}

impl ops::Neg for Position {
    type Output = Self;

    fn neg(self) -> Self::Output {
        return Self {
            x: -self.x,
            y: -self.y,
            z: -self.z,
        };
    }
}

impl ops::Add<Self> for Position {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        return Self {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
            z: self.z + rhs.z,
        };
    }
}

impl ops::Sub<Self> for Position {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        return Self {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
            z: self.z - rhs.z,
        };
    }
}

impl ops::Mul<f64> for Position {
    type Output = Self;

    fn mul(self, rhs: f64) -> Self::Output {
        return Self {
            x: self.x * rhs,
            y: self.y * rhs,
            z: self.z * rhs,
        };
    }
}

impl ops::Div<f64> for Position {
    type Output = Self;

    fn div(self, rhs: f64) -> Self::Output {
        return Self {
            x: self.x / rhs,
            y: self.y / rhs,
            z: self.z / rhs,
        };
    }
}

impl From<(f64, f64, f64)> for Position {
    fn from(f: (f64, f64, f64)) -> Self {
        return Self {
            x: f.0,
            y: f.1,
            z: f.2,
        };
    }
}

impl Into<(f64, f64, f64)> for Position {
    fn into(self) -> (f64, f64, f64) {
        return (self.x, self.y, self.z);
    }
}
